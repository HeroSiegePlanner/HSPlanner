//! Bundled interface translations. Saved Settings own the language preference;
//! this adapter publishes it to GPUI and pure presentation helpers alike.
use gpui_kit::{App, Global};
use std::{
    collections::HashMap,
    sync::{
        LazyLock,
        atomic::{AtomicU8, Ordering},
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Language {
    #[default]
    English,
    Korean,
    Russian,
    Chinese,
}

impl Language {
    pub const ALL: [Self; 4] = [Self::English, Self::Korean, Self::Russian, Self::Chinese];

    /// Unknown or unsupported preferences safely retain the English interface.
    pub fn from_code(code: &str) -> Self {
        match code {
            "ko" => Self::Korean,
            "ru" => Self::Russian,
            "zh-CN" => Self::Chinese,
            _ => Self::English,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Korean => "ko",
            Self::Russian => "ru",
            Self::Chinese => "zh-CN",
        }
    }

    /// Autonyms remain recognizable after choosing an unfamiliar language.
    pub fn name(self) -> &'static str {
        text_for(self, "language.name")
    }
}

#[derive(Clone, Copy)]
pub struct Locale {
    language: Language,
}
impl Global for Locale {}
impl Locale {
    pub fn language(&self) -> Language {
        self.language
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(Language::English as u8);
static COMPONENT_TRANSLATIONS: ComponentTranslations = ComponentTranslations;
static REGISTER_COMPONENT_TRANSLATIONS: std::sync::Once = std::sync::Once::new();

// GPUI Kit's bundled catalog has no Korean/Russian. Its documented rust-i18n
// extension seam lets our JSON also translate standard controls and edit menus.
struct ComponentTranslations;
impl rust_i18n::Backend for ComponentTranslations {
    fn available_locales(&self) -> Vec<std::borrow::Cow<'_, str>> {
        Language::ALL
            .into_iter()
            .map(|language| language.code().into())
            .collect()
    }
    fn translate(&self, locale: &str, key: &str) -> Option<std::borrow::Cow<'_, str>> {
        lookup(
            &CATALOGS[Language::from_code(locale) as usize],
            &CATALOGS[0],
            key,
        )
        .map(Into::into)
    }
    fn messages_for_locale(
        &self,
        locale: &str,
    ) -> Option<Vec<(std::borrow::Cow<'_, str>, std::borrow::Cow<'_, str>)>> {
        Some(
            CATALOGS[Language::from_code(locale) as usize]
                .iter()
                .filter(|(key, _)| key.starts_with("component."))
                .map(|(key, value)| (key.as_str().into(), value.as_str().into()))
                .collect(),
        )
    }
}
static CATALOGS: LazyLock<[HashMap<String, String>; 4]> = LazyLock::new(|| {
    [
        include_str!("../locales/en.json"),
        include_str!("../locales/ko.json"),
        include_str!("../locales/ru.json"),
        include_str!("../locales/zh-CN.json"),
    ]
    .map(|source| serde_json::from_str(source).expect("valid bundled translations"))
});

pub fn language() -> Language {
    Language::ALL[CURRENT.load(Ordering::Relaxed) as usize]
}

/// Called on the UI thread when loading or editing Settings, before notifying
/// document observers. Changing presentation never edits a build or its history.
pub fn apply_language(code: &str, cx: &mut App) {
    REGISTER_COMPONENT_TRANSLATIONS.call_once(|| {
        gpui_kit::component::_rust_i18n_extend(&COMPONENT_TRANSLATIONS, "component");
    });
    let selected = Language::from_code(code);
    if cx
        .try_global::<Locale>()
        .is_some_and(|locale| locale.language == selected)
    {
        return;
    }
    CURRENT.store(selected as u8, Ordering::Relaxed);
    gpui_kit::component::set_locale(selected.code());
    cx.set_global(Locale { language: selected });
    cx.refresh_windows();
}

pub fn text_for(language: Language, key: &str) -> &'static str {
    lookup(&CATALOGS[language as usize], &CATALOGS[0], key).unwrap_or_else(|| {
        log::error!("Missing interface translation: {key}");
        "⟦missing translation⟧"
    })
}

fn lookup<'a>(
    catalog: &'a HashMap<String, String>,
    english: &'a HashMap<String, String>,
    key: &str,
) -> Option<&'a str> {
    catalog
        .get(key)
        .or_else(|| english.get(key))
        .map(String::as_str)
}

pub fn tr(key: &str) -> &'static str {
    text_for(language(), key)
}

/// Display adapter for data-driven labels. Unknown future game definitions
/// keep their source label while known labels share the interface catalogs.
pub fn tr_or<'a>(key: &str, fallback: &'a str) -> &'a str {
    lookup(&CATALOGS[language() as usize], &CATALOGS[0], key).unwrap_or(fallback)
}

/// Named substitutions allow translators to reorder a complete sentence.
/// Values are copied once and never interpreted as translation templates.
pub fn trf(key: &str, arguments: &[(&str, String)]) -> String {
    interpolate(tr(key), arguments)
}

fn interpolate(template: &str, arguments: &[(&str, String)]) -> String {
    let mut output = String::with_capacity(template.len());
    let mut remaining = template;
    while let Some(start) = remaining.find('{') {
        output.push_str(&remaining[..start]);
        let Some(end) = remaining[start..].find('}').map(|end| end + start) else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let name = &remaining[start + 1..end];
        if let Some((_, value)) = arguments.iter().find(|(key, _)| *key == name) {
            output.push_str(value);
        } else {
            output.push_str(&remaining[start..=end]);
        }
        remaining = &remaining[end + 1..];
    }
    output.push_str(remaining);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn placeholders(text: &str) -> BTreeSet<&str> {
        text.split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name))
            .collect()
    }

    #[test]
    fn all_languages_have_complete_matching_templates() {
        let english = &CATALOGS[0];
        for language in Language::ALL {
            let catalog = &CATALOGS[language as usize];
            assert_eq!(
                catalog.len(),
                english.len(),
                "{} key count",
                language.code()
            );
            for (key, source) in english {
                let translated = catalog
                    .get(key)
                    .unwrap_or_else(|| panic!("{}: missing {key}", language.code()));
                assert!(
                    !translated.trim().is_empty(),
                    "{}: empty {key}",
                    language.code()
                );
                assert_eq!(
                    placeholders(source),
                    placeholders(translated),
                    "{}: {key}",
                    language.code()
                );
            }
        }
    }

    #[test]
    fn defaults_and_unknown_preferences_use_english() {
        assert_eq!(Language::default(), Language::English);
        assert_eq!(Language::from_code("future-locale"), Language::English);
        for language in Language::ALL {
            assert_eq!(Language::from_code(language.code()), language);
            assert!(!language.name().is_empty());
        }
    }

    #[test]
    fn missing_translation_falls_back_to_english() {
        let incomplete = HashMap::new();
        assert_eq!(
            lookup(&incomplete, &CATALOGS[0], "settings.language"),
            Some("Language")
        );
        assert_eq!(text_for(Language::Korean, "settings.language"), "언어");
        assert_eq!(text_for(Language::Russian, "settings.language"), "Язык");
        assert_eq!(text_for(Language::Chinese, "settings.language"), "语言");
    }

    #[test]
    fn component_edit_menus_use_the_same_catalogs() {
        use rust_i18n::Backend;
        let backend = rust_i18n::NamespacedBackend::new(&COMPONENT_TRANSLATIONS, "component");
        assert_eq!(
            backend.translate("ko", "Input.Copy").as_deref(),
            Some("복사")
        );
        assert_eq!(
            backend.translate("ru", "Input.Paste").as_deref(),
            Some("Вставить")
        );
        assert_eq!(
            backend.translate("zh-CN", "Dialog.cancel").as_deref(),
            Some("取消")
        );
        assert_eq!(backend.available_locales().len(), 4);
    }

    #[test]
    fn interface_translation_calls_reference_existing_keys() {
        fn visit(directory: &std::path::Path) {
            for entry in std::fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    visit(&path);
                } else if path.extension().is_some_and(|extension| extension == "rs") {
                    let source = std::fs::read_to_string(&path).unwrap();
                    let production = source.split("#[cfg(test)]").next().unwrap();
                    for call in ["tr(", "trf(", "tr_or("] {
                        for (offset, _) in production.match_indices(call) {
                            if production[..offset]
                                .chars()
                                .next_back()
                                .is_some_and(|ch| ch.is_alphanumeric() || ch == '_')
                            {
                                continue;
                            }
                            let Some(part) = production[offset + call.len()..]
                                .trim_start()
                                .strip_prefix('"')
                            else {
                                continue;
                            };
                            let key = part.split('"').next().unwrap();
                            assert!(
                                CATALOGS[0].contains_key(key),
                                "{}: missing {key}",
                                path.display()
                            );
                        }
                    }
                }
            }
        }
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        for name in ["app", "ui", "planner", "library", "notes"] {
            visit(&crates.join(name).join("src"));
        }
    }

    #[test]
    fn interpolation_reorders_arguments_without_reinterpreting_user_text() {
        assert_eq!(
            interpolate(
                "{count}: {name}",
                &[("name", "{count} 한글 中文".into()), ("count", "3".into())]
            ),
            "3: {count} 한글 中文"
        );
        assert_eq!(interpolate("{unknown}", &[]), "{unknown}");
    }
}
