use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Notes {
    pub markdown: String,
    pub original_html: Option<String>,
}

impl Notes {
    pub fn from_html(html: &str) -> Self {
        Self {
            markdown: html2md::parse_html(&ammonia::clean(html)),
            original_html: (!html.is_empty()).then(|| html.to_owned()),
        }
    }

    pub fn to_html(&self) -> String {
        let mut html = String::new();
        let parser = pulldown_cmark::Parser::new_ext(
            &self.markdown,
            pulldown_cmark::Options::ENABLE_STRIKETHROUGH | pulldown_cmark::Options::ENABLE_TABLES,
        );
        pulldown_cmark::html::push_html(&mut html, parser);
        ammonia::clean(&html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_formatting_and_preserves_original_while_exporting_safe_html() {
        let source = "<h2>Build</h2><ul><li><b>Fire</b></li></ul><a href=\"https://example.com\">Guide</a><script>alert(1)</script>";
        let notes = Notes::from_html(source);
        assert_eq!(notes.original_html.as_deref(), Some(source));
        assert!(notes.markdown.contains("**Fire**"));
        assert!(notes.markdown.contains("https://example.com"));
        assert!(!notes.to_html().contains("script"));
        assert!(notes.to_html().contains("<li>"));
    }
}
