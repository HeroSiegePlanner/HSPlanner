//! GitHub release bodies (our CHANGELOG.md) reduced to titled sections of plain-text entries.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Section {
    pub title: Option<String>,
    pub items: Vec<String>,
}

pub fn parse(markdown: &str) -> Vec<Section> {
    let mut sections = vec![Section::default()];
    let mut open = false;
    for line in markdown.lines() {
        let line = line.trim();
        let section = sections.last_mut().expect("one section");
        if line.is_empty() || line.starts_with("<!--") || is_rule(line) {
            open = false;
        } else if let Some(heading) = heading(line) {
            sections.push(Section {
                title: Some(plain(heading)),
                items: Vec::new(),
            });
            open = false;
        } else if let Some(entry) = bullet(line) {
            section.items.push(plain(entry));
            open = true;
        } else if open && let Some(last) = section.items.last_mut() {
            last.push(' ');
            last.push_str(&plain(line));
        } else {
            section.items.push(plain(line));
            open = true;
        }
    }
    sections.retain(|section| !section.items.is_empty());
    sections
}

fn heading(line: &str) -> Option<&str> {
    let text = line.trim_start_matches('#');
    (text.len() < line.len() && (text.is_empty() || text.starts_with(' '))).then(|| text.trim())
}

fn bullet(line: &str) -> Option<&str> {
    if let Some(rest) = ["- ", "* ", "+ "]
        .iter()
        .find_map(|marker| line.strip_prefix(marker))
    {
        return Some(rest.trim());
    }
    let digits = line.find(|c: char| !c.is_ascii_digit())?;
    let rest = line[digits..].strip_prefix(". ")?;
    (digits > 0).then(|| rest.trim())
}

fn is_rule(line: &str) -> bool {
    line.len() >= 3 && line.chars().all(|c| c == '-' || c == '*' || c == '_')
}

/// Drops code ticks, strong emphasis, quotes and link targets; keeps the readable text.
fn plain(text: &str) -> String {
    let text = text.trim_start_matches('>').trim();
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        let label_end = rest[start..].find("](").map(|i| start + i);
        let target_end = label_end.and_then(|end| rest[end..].find(')').map(|i| end + i));
        let (Some(label_end), Some(target_end)) = (label_end, target_end) else {
            break;
        };
        out.push_str(rest[..start].strip_suffix('!').unwrap_or(&rest[..start]));
        out.push_str(&rest[start + 1..label_end]);
        rest = &rest[target_end + 1..];
    }
    out.push_str(rest);
    out.replace("**", "").replace("__", "").replace('`', "")
}

#[cfg(test)]
mod tests {
    use super::{Section, parse};

    fn section(title: Option<&str>, items: &[&str]) -> Section {
        Section {
            title: title.map(str::to_owned),
            items: items.iter().map(|item| (*item).to_owned()).collect(),
        }
    }

    #[test]
    fn headings_start_sections_and_bullets_become_entries() {
        let notes = parse(
            "## Native desktop application\n\n- HSPlanner now runs on pure `Rust`\n- Notes use **Markdown**.\n\n## Installing this version\n\nDownload the installer from GitHub.\nIt checks for updates.\n",
        );
        assert_eq!(
            notes,
            vec![
                section(
                    Some("Native desktop application"),
                    &["HSPlanner now runs on pure Rust", "Notes use Markdown."]
                ),
                section(
                    Some("Installing this version"),
                    &["Download the installer from GitHub. It checks for updates."]
                ),
            ]
        );
    }

    #[test]
    fn wrapped_bullets_join_and_blank_lines_split_entries() {
        let notes = parse("- first line\n  continues here\n\n- second\n1. numbered\n");
        assert_eq!(
            notes,
            vec![section(
                None,
                &["first line continues here", "second", "numbered"]
            )]
        );
    }

    #[test]
    fn links_images_and_rules_are_reduced_to_text() {
        let notes = parse(
            "# Fix\n\n- See [issue #12](https://x/12) and ![shot](a.png).\n---\n> Quoted __note__\n",
        );
        assert_eq!(
            notes,
            vec![section(
                Some("Fix"),
                &["See issue #12 and shot.", "Quoted note"]
            )]
        );
    }

    #[test]
    fn empty_sections_and_bodies_are_dropped() {
        assert!(parse("").is_empty());
        assert_eq!(
            parse("## Empty\n\n## Feature\n- added"),
            vec![section(Some("Feature"), &["added"])]
        );
        assert_eq!(
            parse("#hashtag text"),
            vec![section(None, &["#hashtag text"])]
        );
    }
}
