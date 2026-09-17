# Interface translations

Choose **Settings → Language** to switch between English, 한국어, Русский and 简体中文. The choice takes effect immediately and is stored with the device's preferences. Existing installations and unsupported language codes use English.

All four catalogs live in `crates/ui/locales`: `en.json`, `ko.json`, `ru.json`, and `zh-CN.json`. They are bundled into the executable, so no network request or separate installation of language files is needed. Rebuild the app after editing a catalog.

Use stable semantic keys such as `settings.language` through `hsplanner_ui::i18n::tr`. For sentences containing values, use `trf` with named placeholders, for example `{name}` and `{count}`; each translation may reorder them. Keep each sentence in one entry. Every language must contain the same keys and placeholders. The UI tests check these contracts and the English fallback. Entries under `component.*` also translate standard GPUI controls and text-editing menus through its rust-i18n extension seam.

Persistent settings own the language preference. The UI adapter publishes a `Locale` global for retained controls and updates the component library's locale. Views should resolve labels during rendering; retained placeholders/options must observe `Locale` and update their presentation without clearing the user's input. Element IDs, sort keys and model values must never depend on a translated label.

Catalogs cover application interface copy and game stat/attribute display names and skill tags. Stat searches accept both translated and original names. User-authored build names and notes, canonical game item/skill names and descriptions, imported item text, release notes, and calculation traces and technical diagnostic details retain their source content. Game-data identifiers and the import/export grammar must stay unchanged; translate display labels at the view boundary.

Run `cargo test -p hsplanner-ui -p hsplanner-build -p hsplanner-planner` and `cargo check -p hsplanner` after changes. Review new translations in their actual controls, especially Russian text expansion and CJK typography.
