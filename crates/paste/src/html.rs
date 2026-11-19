pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Build pages live at `/b/<id>`, the address the earlier hsplanner.app site used.
pub const BUILD_PATH: &str = "/b/";
pub const SITE_NAME: &str = "HSPlanner";
/// Changes whenever the stylesheet, script or tree layouts do, so browsers refetch them.
pub const ASSET_REV: &str = env!("HSPASTE_ASSET_REV");
const RELEASES_URL: &str = "https://github.com/HeroSiegePlanner/HSPlanner/releases/latest";

pub struct Meta<'a> {
    pub title: &'a str,
    pub description: &'a str,
    /// Absolute address of a preview image, when the site knows its public address.
    pub image: Option<&'a str>,
    /// The site line above the title in a link preview.
    pub site: &'a str,
}

/// What surrounds a page: the shared build it shows, and the footer's right-hand note.
#[derive(Default)]
pub struct Chrome<'a> {
    pub paste_id: Option<&'a str>,
    pub footnote: &'a str,
}

/// Text made safe for element content and quoted attributes.
pub fn esc(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn topbar(chrome: &Chrome) -> String {
    let (crumb, actions) = match chrome.paste_id {
        Some(id) => {
            let id = esc(id);
            (
                format!("<span class=\"crumb\">builds / <b>{id}</b></span>"),
                format!(
                    "<span class=\"chip\">Read-only</span>\
<button type=\"button\" class=\"btn needs-js\" data-copy=\"code\">Copy code</button>\
<button type=\"button\" class=\"btn needs-js\" data-copy=\"link\">Copy link</button>\
<a class=\"btn\" href=\"{BUILD_PATH}{id}/raw\">Raw code</a>\
<a class=\"btn gold\" href=\"/\">Share a build</a>"
                ),
            )
        }
        None => (
            "<span class=\"crumb\">builds</span>".to_owned(),
            format!("<a class=\"btn gold\" href=\"{RELEASES_URL}\">Get HSPlanner</a>"),
        ),
    };
    format!(
        "<header class=\"topbar\"><a class=\"brand-mark\" href=\"/\">HSPlanner</a>{crumb}\
<div class=\"topbar-actions\">{actions}</div></header>"
    )
}

pub fn document(meta: &Meta, chrome: &Chrome, body: &str) -> String {
    let title = esc(meta.title);
    let description = esc(meta.description);
    // The `summary` card is what makes chat apps show the image as a small thumbnail.
    let image = meta.image.map_or_else(String::new, |image| {
        let image = esc(image);
        format!(
            "<meta property=\"og:image\" content=\"{image}\">\n\
<meta name=\"twitter:image\" content=\"{image}\">"
        )
    });
    let site = esc(meta.site);
    let topbar = topbar(chrome);
    let footnote = esc(chrome.footnote);
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<meta name="description" content="{description}">
<meta property="og:site_name" content="{site}">
<meta property="og:type" content="website">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{description}">
<meta name="twitter:card" content="summary">
<meta name="twitter:title" content="{title}">
<meta name="twitter:description" content="{description}">
{image}
<meta name="theme-color" content="#c9a55a">
<link rel="icon" href="/static/icon.svg" type="image/svg+xml">
<link rel="stylesheet" href="/static/style.css?v={ASSET_REV}">
<script src="/static/app.js?v={ASSET_REV}"></script>
</head>
<body>
{topbar}
{body}
<footer class="footer"><div class="wrap footer-inner">
<span class="diamond"></span>
<span>HSPlanner · unofficial fan tool · <a href="{RELEASES_URL}">get the planner</a></span>
<span class="footer-right">{footnote}</span>
</div></footer>
</body>
</html>
"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_neutralises_markup_and_attribute_breakouts() {
        assert_eq!(
            esc("<a href=\"x\" on='y'>&</a>"),
            "&lt;a href=&quot;x&quot; on=&#39;y&#39;&gt;&amp;&lt;/a&gt;"
        );
    }
}
