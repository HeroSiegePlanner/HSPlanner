//! Addresses of hsplanner.app, the site that turns a share code into a short link.
//! Only this host is ever named, so a pasted link cannot send a request elsewhere.
use serde_json::Value;

pub const SITE_URL: &str = "https://hsplanner.app";
pub const API_URL: &str = "https://hsplanner.app/api/pastes";
const HOST: &str = "hsplanner.app";
const ID_LENGTH: usize = 10;

fn valid_id(id: &str) -> bool {
    id.len() == ID_LENGTH && id.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

fn strip_ignoring_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map(|_| &text[prefix.len()..])
}

/// The build id in a pasted `hsplanner.app/b/<id>` link; any other text gives none.
pub fn build_id(input: &str) -> Option<&str> {
    let link = input.trim();
    let link = strip_ignoring_case(link, "https://")
        .or_else(|| strip_ignoring_case(link, "http://"))
        .unwrap_or(link);
    let link = strip_ignoring_case(link, "www.").unwrap_or(link);
    let path = strip_ignoring_case(link, HOST)?;
    let id = path.strip_prefix("/b/")?.split(['/', '?', '#']).next()?;
    valid_id(id).then_some(id)
}

pub fn build_url(id: &str) -> String {
    format!("{SITE_URL}/b/{id}")
}

/// Where the share code behind a build page is served as plain text.
pub fn raw_url(id: &str) -> String {
    format!("{SITE_URL}/b/{id}/raw")
}

/// The link for a build the site just stored, from its API reply. The address in the
/// reply is not used: the link is rebuilt from the id, so it always points at the site.
pub fn shared_link(reply: &[u8]) -> Option<String> {
    let reply: Value = serde_json::from_slice(reply).ok()?;
    let id = reply.get("data")?.get("id")?.as_str()?;
    (reply.get("success")?.as_bool()? && valid_id(id)).then(|| build_url(id))
}
