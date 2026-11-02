//! Public GitHub issues, read without an account so reporters can spot an existing report.
use crate::update::{REPO, read};
use gpui_kit::http_client::{AsyncBody, HttpClient, HttpRequestExt, RedirectPolicy, Request};
use hsplanner_ui::i18n::tr;
use serde::Deserialize;
use std::{collections::HashSet, sync::Arc, time::Duration};

const PAGE: usize = 100;
// GitHub search stops at 1000 results.
const MAX_PAGES: usize = 10;
const SIMILARITY: f32 = 0.6;
const MAX_SIMILAR: usize = 3;
const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "not", "does", "doesn", "doesnt", "dont", "cant", "isn", "when",
    "from", "this", "that", "into", "bug", "feature", "request", "idea", "issue", "wrong",
    "broken", "data", "skill", "item", "show", "shows", "work", "works", "working", "dps",
];

#[derive(Clone, Debug, Deserialize)]
pub(super) struct Issue {
    pub number: u64,
    pub title: String,
    #[serde(rename = "html_url")]
    pub url: String,
    pub state: String,
    #[serde(default)]
    pub body: Option<String>,
}
impl Issue {
    pub fn is_open(&self) -> bool {
        self.state == "open"
    }
    /// Markdown body without unanswered template sections; inline screenshots become links.
    pub fn details(&self) -> String {
        let body = images_as_links(
            &self
                .body
                .as_deref()
                .unwrap_or_default()
                .replace("\r\n", "\n"),
        );
        let mut sections = vec![String::new()];
        for line in body.lines() {
            if line.starts_with("### ") {
                sections.push(String::new());
            }
            if let Some(section) = sections.last_mut() {
                section.push_str(line);
                section.push('\n');
            }
        }
        sections
            .into_iter()
            .filter(|section| {
                let answer = match section.strip_prefix("### ") {
                    Some(rest) => rest.split_once('\n').map_or("", |(_, answer)| answer),
                    None => section,
                }
                .trim();
                answer != "_No response_" && answer.chars().any(char::is_alphanumeric)
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_owned()
    }
}

fn images_as_links(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<img") {
        out.push_str(&rest[..start]);
        let end = rest[start..]
            .find('>')
            .map_or(rest.len(), |end| start + end + 1);
        if let Some(src) = rest[start..end]
            .split("src=\"")
            .nth(1)
            .and_then(|src| src.split('"').next())
        {
            out.push_str(&format!("[{}]({src})", tr("report.issue_screenshot")));
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

#[derive(Deserialize)]
struct Page {
    items: Vec<Issue>,
}

pub(super) fn issues_url() -> String {
    format!("https://github.com/{REPO}/issues")
}

pub(super) async fn fetch(http: Arc<dyn HttpClient>) -> Result<Vec<Issue>, String> {
    let mut issues = Vec::new();
    for page in 1..=MAX_PAGES {
        let items = match fetch_page(&http, page).await {
            Ok(items) => items,
            // Later pages are a bonus: keep what arrived.
            Err(_) if page > 1 => break,
            Err(error) => return Err(error),
        };
        let last = items.len() < PAGE;
        issues.extend(items);
        if last {
            break;
        }
    }
    // An issue filed between page requests shifts the next page by one.
    issues.dedup_by_key(|issue| issue.number);
    Ok(issues)
}

async fn fetch_page(http: &Arc<dyn HttpClient>, page: usize) -> Result<Vec<Issue>, String> {
    let url = format!(
        "https://api.github.com/search/issues?q=repo:{REPO}+is:issue&sort=created&order=desc&per_page={PAGE}&page={page}"
    );
    let request = Request::get(url)
        .header("Accept", "application/vnd.github+json")
        .follow_redirects(RedirectPolicy::FollowAll)
        .timeout(Duration::from_secs(10))
        .body(AsyncBody::default())
        .map_err(|_| tr("report.issues_failed"))?;
    let response = http
        .send(request)
        .await
        .map_err(|_| tr("report.issues_failed"))?;
    let body = read(response)
        .await
        .map_err(|_| tr("report.issues_failed"))?;
    parse(&body)
}

fn parse(body: &[u8]) -> Result<Vec<Issue>, String> {
    serde_json::from_slice::<Page>(body)
        .map(|page| page.items)
        .map_err(|_| tr("report.issues_failed").into())
}

fn words(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(|word| {
            let stem = word
                .strip_suffix('s')
                .filter(|stem| stem.chars().count() >= 3)
                .unwrap_or(word);
            (stem.chars().count() >= 3 && !STOPWORDS.contains(&word) && !STOPWORDS.contains(&stem))
                .then(|| stem.to_owned())
        })
        .collect()
}

/// Issues whose title words mostly appear in the report text, best match first.
pub(super) fn similar(issues: &[Issue], report: &str) -> Vec<Issue> {
    let report = words(report);
    let mut scored: Vec<_> = issues
        .iter()
        .filter_map(|issue| {
            let title = words(&issue.title);
            let hits = title.intersection(&report).count();
            let score = hits as f32 / title.len().max(1) as f32;
            (hits > 0 && score >= SIMILARITY).then_some((score, issue))
        })
        .collect();
    scored.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then(b.1.is_open().cmp(&a.1.is_open()))
            .then(b.1.number.cmp(&a.1.number))
    });
    scored
        .into_iter()
        .take(MAX_SIMILAR)
        .map(|(_, issue)| issue.clone())
        .collect()
}

/// List filter: every query word is in the title, is the issue number, or starts it after `#`.
pub(super) fn matches(issue: &Issue, query: &str) -> bool {
    let title = issue.title.to_lowercase();
    let number = issue.number.to_string();
    query.to_lowercase().split_whitespace().all(|word| {
        title.contains(word)
            || word == number
            || word
                .strip_prefix('#')
                .is_some_and(|digits| number.starts_with(digits))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(number: u64, title: &str, state: &str) -> Issue {
        Issue {
            number,
            title: title.into(),
            url: format!("https://github.com/{REPO}/issues/{number}"),
            state: state.into(),
            body: None,
        }
    }

    #[test]
    fn details_drop_unanswered_template_sections_and_link_screenshots() {
        let mut deathcrown = issue(218, "[BUG] High Roller's Deathcrown", "closed");
        deathcrown.body = Some("### Describe your issue\r\n\r\nRoll The Dice never activates.\r\n\r\n### Steps to reproduce the behavior\r\n\r\n_No response_\r\n\r\n### If applicable, add screenshots\r\n\r\n<img width=\"935\" alt=\"Image\" src=\"https://github.com/user-attachments/assets/1\" />\r\n\r\n### Add any other context\r\n\r\n".into());
        let details = deathcrown.details();
        assert!(details.starts_with("### Describe your issue\n\nRoll The Dice never activates."));
        assert!(!details.contains("Steps to reproduce"));
        assert!(!details.contains("other context"));
        assert!(!details.contains("<img"));
        assert!(details.contains("](https://github.com/user-attachments/assets/1)"));
        assert_eq!(issue(1, "Applicating closing", "closed").details(), "");
        let mut dot = issue(2, "[BUG] Etherwalk", "closed");
        dot.body = Some("### What did you expect instead\n\n.\n".into());
        assert_eq!(dot.details(), "");
    }

    fn tracker() -> Vec<Issue> {
        vec![
            issue(218, "[BUG] High Roller's Deathcrown", "closed"),
            issue(
                209,
                "[BUG] Resistance -> Life Incarnation Tree Nodes",
                "closed",
            ),
            issue(208, "[BUG] Etherwalk", "closed"),
            issue(198, "[FEATURE] Inventory Net Change", "open"),
            issue(
                167,
                "[FEATURE] Option to make incarnation tree background darker",
                "open",
            ),
        ]
    }

    #[test]
    fn parses_search_results_and_rejects_unexpected_json() {
        let body = br#"{"total_count":1,"items":[{"number":218,"title":"[BUG] High Roller's Deathcrown","html_url":"https://github.com/HeroSiegePlanner/HSPlanner/issues/218","state":"closed","labels":[]}]}"#;
        let issues = parse(body).unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 218);
        assert!(issues[0].url.ends_with("/issues/218"));
        assert!(!issues[0].is_open());
        assert!(parse(br#"{"message":"API rate limit exceeded"}"#).is_err());
    }

    #[test]
    fn finds_reports_naming_the_same_item_or_nodes() {
        let found = similar(&tracker(), "Etherwalk boots give no movement speed");
        assert_eq!(found.iter().map(|i| i.number).collect::<Vec<_>>(), [208]);

        let found = similar(
            &tracker(),
            "Incarnation tree resistance nodes do not convert to life",
        );
        assert_eq!(found[0].number, 209);
    }

    #[test]
    fn ignores_reports_sharing_only_generic_words() {
        assert!(similar(&tracker(), "The skill shows wrong data when I click a bug").is_empty());
        assert!(similar(&tracker(), "Frost Nova shows 0 DPS on the tree").is_empty());
        let skills = [issue(1, "[BUG] Skills", "open")];
        assert!(similar(&skills, "my skills show wrong data").is_empty());
    }

    #[test]
    fn list_filter_matches_title_words_and_issue_number() {
        let deathcrown = &tracker()[0];
        assert!(matches(deathcrown, ""));
        assert!(matches(deathcrown, "deathcrown HIGH"));
        assert!(matches(deathcrown, "#218"));
        assert!(matches(deathcrown, "#21"));
        assert!(matches(deathcrown, "218"));
        assert!(!matches(deathcrown, "#22"));
        assert!(!matches(deathcrown, "etherwalk"));
    }
}
