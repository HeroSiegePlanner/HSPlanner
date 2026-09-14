//! Wire-compatible with Tauri's bugReport.ts; no automatic retries or live-test submissions.
use gpui_kit::http_client::{AsyncBody, HttpClient, HttpRequestExt, RedirectPolicy, Request};
use serde_json::{Value, json};
use std::{io::Read, path::Path, sync::Arc, time::Duration};

pub(super) const MAX_SHOTS: usize = 3;
pub(super) const MAX_BYTES: usize = 8 * 1024 * 1024;
include!(concat!(env!("OUT_DIR"), "/bug-report-config.rs"));

pub(super) fn endpoint() -> Option<String> {
    std::env::var("HSPLANNER_BUG_REPORT_URL")
        .ok()
        .or_else(|| COMPILED_ENDPOINT.map(str::to_owned))
        .filter(|s| !s.trim().is_empty())
}

#[derive(Clone)]
pub(super) struct Shot {
    pub id: String,
    pub name: String,
    pub bytes: Arc<[u8]>,
    pub mime: &'static str,
    pub extension: &'static str,
}
impl Shot {
    pub fn from_bytes(name: String, bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.len() > MAX_BYTES {
            return Err("Each screenshot must be at most 8 MB.".into());
        }
        let format =
            image::guess_format(&bytes).map_err(|_| "Choose a PNG, JPEG, WebP or GIF image.")?;
        let (mime, extension) = match format {
            image::ImageFormat::Png => ("image/png", "png"),
            image::ImageFormat::Jpeg => ("image/jpeg", "jpg"),
            image::ImageFormat::WebP => ("image/webp", "webp"),
            image::ImageFormat::Gif => ("image/gif", "gif"),
            _ => return Err("Choose a PNG, JPEG, WebP or GIF image.".into()),
        };
        // Decode with bounded dimensions before accepting untrusted image bytes.
        let mut reader = image::ImageReader::with_format(std::io::Cursor::new(&bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(16384);
        limits.max_image_height = Some(16384);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        reader
            .decode()
            .map_err(|_| "The screenshot is damaged or too large to decode.")?;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            bytes: bytes.into(),
            mime,
            extension,
        })
    }
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| "Could not open the selected image.")?
            .take((MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "Could not read the selected image.")?;
        Self::from_bytes(
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into(),
            bytes,
        )
    }
}

pub(super) struct Report {
    pub kind: usize,
    pub title: String,
    pub description: String,
    pub steps: String,
    pub expected: String,
    pub contact: String,
    pub build: Option<(String, String)>,
    pub shots: Vec<Shot>,
}
impl Report {
    pub fn validate(&self) -> Result<(), String> {
        for (label, value, min, max) in [
            ("Title", &self.title, 3, 100),
            ("Description", &self.description, 10, 1000),
            ("Steps", &self.steps, 0, 600),
            ("Expected result", &self.expected, 0, 400),
            ("Contact", &self.contact, 0, 80),
        ] {
            let len = value.trim().chars().count();
            if len < min || len > max {
                return Err(format!("{label}: enter {min}–{max} characters."));
            }
        }
        if self.kind > 2 {
            return Err("Choose a report type.".into());
        }
        if self.shots.len() > MAX_SHOTS
            || self.shots.iter().any(|shot| shot.bytes.len() > MAX_BYTES)
        {
            return Err("Attach up to 3 images, at most 8 MB each.".into());
        }
        Ok(())
    }
    fn payload(&self) -> Value {
        let mut fields = Vec::new();
        for (name, value) in [
            ("Steps to reproduce", &self.steps),
            ("Expected instead", &self.expected),
            ("Contact", &self.contact),
        ] {
            if !value.trim().is_empty() {
                fields.push(json!({"name": name, "value": value.trim()}));
            }
        }
        if let Some((label, _)) = &self.build {
            fields.push(json!({"name":"Build", "value": label.chars().take(120).collect::<String>(), "inline": true}));
        }
        let mut embed = json!({"author":{"name":(["Bug report","Wrong data","Idea or request"][self.kind])}, "title":self.title.trim(), "description":self.description.trim(), "color":([0xd96b5a,0xe0b864,0x74c98a][self.kind]), "fields":fields, "footer":{"text":format!("HSPlanner v{} · {} · {}", env!("CARGO_PKG_VERSION"), if cfg!(debug_assertions) {"Dev"} else {"Stable"}, std::env::consts::OS)}});
        let mut attachments: Vec<_> = self
            .shots
            .iter()
            .enumerate()
            .map(|(i, shot)| json!({"id":i,"filename":format!("shot-{}.{}", i+1, shot.extension)}))
            .collect();
        if let Some(shot) = self.shots.first() {
            embed["image"] = json!({"url": format!("attachment://shot-1.{}", shot.extension)});
        }
        if self.build.is_some() {
            attachments.push(json!({"id":attachments.len(), "filename":"build.hsp"}));
        }
        json!({"embeds":[embed], "attachments":attachments, "allowed_mentions":{"parse":[]}})
    }
    fn multipart(&self, boundary: &str) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"payload_json\"\r\nContent-Type: application/json\r\n\r\n{}\r\n", self.payload()).as_bytes());
        let mut file = |index: usize, name: &str, mime: &str, bytes: &[u8]| {
            body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"files[{index}]\"; filename=\"{name}\"\r\nContent-Type: {mime}\r\n\r\n").as_bytes());
            body.extend_from_slice(bytes);
            body.extend_from_slice(b"\r\n");
        };
        for (i, shot) in self.shots.iter().enumerate() {
            file(
                i,
                &format!("shot-{}.{}", i + 1, shot.extension),
                shot.mime,
                &shot.bytes,
            );
        }
        if let Some((_, code)) = &self.build {
            file(
                self.shots.len(),
                "build.hsp",
                "application/octet-stream",
                code.as_bytes(),
            );
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        body
    }
}
fn status_error(status: u16) -> Result<(), String> {
    match status {
        200..=299 => Ok(()),
        429 => Err("Too many reports. Try again in a minute.".into()),
        500..=599 => Err("The report server had a problem. Try again shortly.".into()),
        _ => Err(format!("Sending the report failed ({status}).")),
    }
}
pub(super) async fn send(
    http: Arc<dyn HttpClient>,
    endpoint: String,
    report: Report,
) -> Result<(), String> {
    report.validate()?;
    let boundary = format!("hsplanner-{}", uuid::Uuid::new_v4());
    let request = Request::post(endpoint.trim())
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .follow_redirects(RedirectPolicy::NoFollow)
        .timeout(Duration::from_secs(30))
        .body(AsyncBody::from(report.multipart(&boundary)))
        .map_err(|_| "Invalid report destination configuration.")?;
    let response = http
        .send(request)
        .await
        .map_err(|_| "Could not confirm delivery. Check your connection before trying again.")?;
    status_error(response.status().as_u16())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct TestClient {
        status: u16,
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }
    impl HttpClient for TestClient {
        fn user_agent(&self) -> Option<&gpui_kit::http_client::http::HeaderValue> {
            None
        }
        fn proxy(&self) -> Option<&gpui_kit::http_client::Url> {
            None
        }
        fn send(
            &self,
            request: Request<AsyncBody>,
        ) -> futures::future::BoxFuture<
            'static,
            anyhow::Result<gpui_kit::http_client::Response<AsyncBody>>,
        > {
            use std::sync::atomic::Ordering;
            self.calls.fetch_add(1, Ordering::SeqCst);
            let status = self.status;
            Box::pin(async move {
                use futures::AsyncReadExt;
                assert_eq!(request.method(), "POST");
                assert_eq!(request.uri(), "https://reports.invalid/test");
                assert!(
                    request.headers()["content-type"]
                        .to_str()
                        .unwrap()
                        .starts_with("multipart/form-data; boundary=hsplanner-")
                );
                let mut bytes = Vec::new();
                request.into_body().read_to_end(&mut bytes).await?;
                assert!(String::from_utf8_lossy(&bytes).contains("payload_json"));
                if status == 0 {
                    anyhow::bail!("network error with secret destination");
                }
                Ok(gpui_kit::http_client::Response::builder()
                    .status(status)
                    .body(AsyncBody::default())?)
            })
        }
    }
    #[test]
    fn request_transport_handles_success_rejection_and_network_failure_without_retries() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        for status in [204, 400, 429, 500, 0] {
            let calls = Arc::new(AtomicUsize::new(0));
            let client = Arc::new(TestClient {
                status,
                calls: calls.clone(),
            });
            let result = futures::executor::block_on(send(
                client,
                "https://reports.invalid/test".into(),
                report(),
            ));
            assert_eq!(result.is_ok(), status == 204);
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            if status == 0 {
                assert!(!result.unwrap_err().contains("secret"));
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let client = Arc::new(TestClient {
            status: 204,
            calls: calls.clone(),
        });
        let mut invalid = report();
        invalid.title.clear();
        assert!(
            futures::executor::block_on(send(
                client,
                "https://reports.invalid/test".into(),
                invalid
            ))
            .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
    fn report() -> Report {
        Report {
            kind: 0,
            title: "Test report".into(),
            description: "A reproducible problem".into(),
            steps: String::new(),
            expected: String::new(),
            contact: String::new(),
            build: None,
            shots: vec![],
        }
    }
    #[test]
    fn validates_before_sending_and_keeps_optional_build_out() {
        let mut r = report();
        assert!(r.validate().is_ok());
        assert!(r.payload()["attachments"].as_array().unwrap().is_empty());
        r.title = "ab".into();
        assert!(r.validate().is_err());
        r.title = "x".repeat(101);
        assert!(r.validate().is_err());
    }
    #[test]
    fn serializes_attachments_without_user_filenames_or_mentions() {
        let mut r = report();
        r.title = "@everyone report".into();
        r.build = Some(("Current build".into(), "test-code".into()));
        r.shots.push(Shot {
            id: "1".into(),
            name: "private\r\nname.png".into(),
            bytes: vec![1, 2, 3].into(),
            mime: "image/png",
            extension: "png",
        });
        let payload = r.payload();
        assert_eq!(payload["attachments"][1]["filename"], "build.hsp");
        assert_eq!(payload["allowed_mentions"]["parse"], json!([]));
        let bytes = r.multipart("test-boundary");
        let body = String::from_utf8_lossy(&bytes);
        assert!(body.contains("name=\"files[1]\""));
        assert!(body.contains("test-code"));
        assert!(!body.contains("private"));
        assert!(body.ends_with("--test-boundary--\r\n"));
    }
    #[test]
    fn handles_server_and_rate_limit_failures() {
        assert!(status_error(204).is_ok());
        assert!(status_error(429).unwrap_err().contains("minute"));
        assert!(status_error(500).is_err());
        assert!(status_error(302).is_err());
    }
    #[test]
    fn rejects_bad_or_oversized_screenshots() {
        assert!(Shot::from_bytes("fake.png".into(), b"not an image".to_vec()).is_err());
        assert!(Shot::from_bytes("big.png".into(), vec![0; MAX_BYTES + 1]).is_err());
    }
}
