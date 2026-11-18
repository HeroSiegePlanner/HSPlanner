use serde::{Deserialize, Serialize};

const MAX_HTML_DEPTH: usize = 128;
const VOID_TAGS: [&str; 6] = ["area", "br", "col", "hr", "img", "wbr"];

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase")]
pub struct Notes {
    pub markdown: String,
    pub original_html: Option<String>,
}

impl Notes {
    pub fn from_html(html: &str) -> Self {
        let clean = ammonia::clean(html);
        // html2md recurses per nesting level; hostile markup would overflow the stack.
        let markdown = if html_depth(&clean) > MAX_HTML_DEPTH {
            ammonia::Builder::empty().clean(&clean).to_string()
        } else {
            html2md::parse_html(&clean)
        };
        Self {
            markdown,
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

/// Deepest element nesting in sanitised HTML, where every non-void tag is closed.
fn html_depth(html: &str) -> usize {
    let bytes = html.as_bytes();
    let (mut depth, mut deepest, mut index) = (0usize, 0usize, 0usize);
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        let closing = bytes.get(index + 1) == Some(&b'/');
        let name_start = index + 1 + usize::from(closing);
        let name_len = bytes[name_start.min(bytes.len())..]
            .iter()
            .take_while(|byte| byte.is_ascii_alphanumeric())
            .count();
        if name_len == 0 {
            index += 1;
            continue;
        }
        let name = html[name_start..name_start + name_len].to_ascii_lowercase();
        if closing {
            depth = depth.saturating_sub(1);
        } else if !VOID_TAGS.contains(&name.as_str()) {
            depth += 1;
            deepest = deepest.max(depth);
        }
        // Quoted attribute values may hold `<` and `>`; they are not markup.
        index = name_start + name_len;
        let mut quote = None;
        while index < bytes.len() {
            match (quote, bytes[index]) {
                (None, b'>') => break,
                (None, b'"' | b'\'') => quote = Some(bytes[index]),
                (Some(open), byte) if byte == open => quote = None,
                _ => {}
            }
            index += 1;
        }
    }
    deepest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostile_nesting_imports_as_plain_text_instead_of_overflowing_the_stack() {
        for open in ["<div>", "<a title=\"</x>\">", "<b>"] {
            let notes = Notes::from_html(&format!("{}deep text", open.repeat(40_000)));
            assert!(notes.markdown.contains("deep text"));
            assert!(notes.to_html().contains("deep text"));
        }
        let ordinary = format!("<p><b>Fire</b>{}</p>", "<br>".repeat(500));
        assert!(Notes::from_html(&ordinary).markdown.contains("**Fire**"));
    }
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
