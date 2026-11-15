//! Talks to hsplanner.app: stores a share code for a link, and reads the code behind one.
//! Failures come back as `site.error.*` catalog keys.
use futures::AsyncReadExt;
use gpui_kit::http_client::{AsyncBody, HttpClient, HttpRequestExt, RedirectPolicy, Request};
use hsplanner_build::{codec::MAX_CODE_LENGTH, site};
use std::{sync::Arc, time::Duration};

const TIMEOUT: Duration = Duration::from_secs(20);
/// A reply is one share code or a short JSON object; anything longer is not the site's.
const MAX_REPLY: usize = MAX_CODE_LENGTH + 1024;

const OFFLINE: &str = "site.error.offline";
const FAILED: &str = "site.error.failed";
const NOT_FOUND: &str = "site.error.not_found";

async fn exchange(
    http: Arc<dyn HttpClient>,
    request: Request<AsyncBody>,
) -> Result<Vec<u8>, String> {
    let mut response = http.send(request).await.map_err(|_| OFFLINE)?;
    match response.status().as_u16() {
        200..=299 => {}
        404 => return Err(NOT_FOUND.into()),
        413 => return Err("library.error.share_size".into()),
        429 => return Err("site.error.rate_limited".into()),
        _ => return Err(FAILED.into()),
    }
    let mut body = Vec::new();
    response
        .body_mut()
        .take(MAX_REPLY as u64 + 1)
        .read_to_end(&mut body)
        .await
        .map_err(|_| OFFLINE)?;
    if body.len() > MAX_REPLY {
        return Err(FAILED.into());
    }
    Ok(body)
}

/// Stores `code` on the site and returns the link to its page.
pub async fn publish(http: Arc<dyn HttpClient>, code: String) -> Result<String, String> {
    let request = Request::post(site::API_URL)
        .header("Content-Type", "text/plain; charset=utf-8")
        .follow_redirects(RedirectPolicy::NoFollow)
        .timeout(TIMEOUT)
        .body(AsyncBody::from(code))
        .map_err(|_| FAILED)?;
    // "No such build" makes no sense for an upload; a 404 here means the site is not ours.
    let reply = exchange(http, request).await.map_err(|error| {
        if error == NOT_FOUND {
            FAILED.into()
        } else {
            error
        }
    })?;
    site::shared_link(&reply).ok_or_else(|| FAILED.into())
}

/// The share code behind build `id`, as the site stores it.
pub async fn fetch_code(http: Arc<dyn HttpClient>, id: String) -> Result<String, String> {
    let request = Request::get(site::raw_url(&id))
        .follow_redirects(RedirectPolicy::NoFollow)
        .timeout(TIMEOUT)
        .body(AsyncBody::default())
        .map_err(|_| FAILED)?;
    let body = exchange(http, request).await?;
    String::from_utf8(body).map_err(|_| FAILED.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::http_client::{Response, Url, http::HeaderValue};
    use std::sync::Mutex;

    /// Method, address and body of one request.
    type Call = (String, String, Vec<u8>);

    /// Answers every request with one canned reply and records what was asked.
    struct Site {
        status: u16,
        body: Vec<u8>,
        seen: Arc<Mutex<Vec<Call>>>,
    }

    impl Site {
        fn answering(status: u16, body: impl Into<Vec<u8>>) -> Arc<Self> {
            Arc::new(Self {
                status,
                body: body.into(),
                seen: Arc::default(),
            })
        }
    }

    impl HttpClient for Site {
        fn user_agent(&self) -> Option<&HeaderValue> {
            None
        }
        fn proxy(&self) -> Option<&Url> {
            None
        }
        fn send(
            &self,
            request: Request<AsyncBody>,
        ) -> futures::future::BoxFuture<'static, anyhow::Result<Response<AsyncBody>>> {
            let (status, body, seen) = (self.status, self.body.clone(), self.seen.clone());
            Box::pin(async move {
                let (method, uri) = (request.method().to_string(), request.uri().to_string());
                let mut sent = Vec::new();
                request.into_body().read_to_end(&mut sent).await?;
                seen.lock().unwrap().push((method, uri, sent));
                anyhow::ensure!(status != 0, "connection refused");
                Ok(Response::builder()
                    .status(status)
                    .body(AsyncBody::from(body))?)
            })
        }
    }

    fn run<T>(future: impl Future<Output = T>) -> T {
        futures::executor::block_on(future)
    }

    #[test]
    fn publish_posts_the_code_and_returns_the_site_link() {
        let reply = r#"{"success":true,"data":{"id":"PfGpnVEmjB","url":"x"},"error":null}"#;
        let site = Site::answering(201, reply);

        let link = run(publish(site.clone(), "N4Igcode".into()));

        assert_eq!(link.as_deref(), Ok("https://hsplanner.app/b/PfGpnVEmjB"));
        let seen = site.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, "POST");
        assert_eq!(seen[0].1, "https://hsplanner.app/api/pastes");
        assert_eq!(seen[0].2, b"N4Igcode");
    }

    #[test]
    fn fetch_code_reads_the_raw_code_of_one_build() {
        let site = Site::answering(200, "N4Igcode");

        let code = run(fetch_code(site.clone(), "PfGpnVEmjB".into()));

        assert_eq!(code.as_deref(), Ok("N4Igcode"));
        let seen = site.seen.lock().unwrap();
        assert_eq!(seen[0].0, "GET");
        assert_eq!(seen[0].1, "https://hsplanner.app/b/PfGpnVEmjB/raw");
    }

    #[test]
    fn failures_name_what_went_wrong_without_retrying() {
        for (status, expected) in [
            (0, "site.error.offline"),
            (404, "site.error.not_found"),
            (413, "library.error.share_size"),
            (429, "site.error.rate_limited"),
            (500, "site.error.failed"),
            (302, "site.error.failed"),
        ] {
            let site = Site::answering(status, "");
            let error = run(fetch_code(site.clone(), "PfGpnVEmjB".into())).unwrap_err();
            assert_eq!(error, expected, "status {status}");
            assert_eq!(site.seen.lock().unwrap().len(), 1);
        }
    }

    #[test]
    fn replies_that_are_too_long_or_not_the_sites_are_refused() {
        let long = Site::answering(200, vec![b'a'; MAX_REPLY + 1]);
        assert_eq!(
            run(fetch_code(long, "PfGpnVEmjB".into())).unwrap_err(),
            FAILED
        );
        let stranger = Site::answering(404, "");
        assert_eq!(
            run(publish(stranger, "N4Igcode".into())).unwrap_err(),
            FAILED
        );
        let junk = Site::answering(201, "<html>captive portal</html>");
        assert_eq!(run(publish(junk, "N4Igcode".into())).unwrap_err(), FAILED);
        let binary = Site::answering(200, vec![0xff, 0xfe]);
        assert_eq!(
            run(fetch_code(binary, "PfGpnVEmjB".into())).unwrap_err(),
            FAILED
        );
    }
}
