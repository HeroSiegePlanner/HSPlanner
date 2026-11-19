//! Files compiled into the server: game icons, fonts, the stylesheet and the tree backdrops.
use std::borrow::Cow;

use crate::tree;

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// A game icon by its path under `assets/`, such as `items/<item id>.png`.
pub fn game(path: &str) -> Option<&'static [u8]> {
    FILES
        .binary_search_by(|(name, _)| (*name).cmp(path))
        .ok()
        .map(|index| FILES[index].1)
}

/// The file name a gem or rune sprite is embedded under; `build.rs` applies the same rule.
pub fn slug(name: &str) -> String {
    name.chars()
        .map(|character| match character {
            character if character.is_ascii_alphanumeric() => character.to_ascii_lowercase(),
            _ => '_',
        })
        .collect()
}

pub fn fixed(name: &str) -> Option<Cow<'static, [u8]>> {
    let bytes: &'static [u8] = match name {
        "style.css" => include_bytes!("../static/style.css"),
        "app.js" => include_bytes!("../static/app.js"),
        "icon.svg" => include_bytes!("../../../app-icon.svg"),
        "inter-400.ttf" => include_bytes!("../../ui/assets/fonts/Inter-Regular.ttf"),
        "inter-500.ttf" => include_bytes!("../../ui/assets/fonts/Inter-Medium.ttf"),
        "inter-600.ttf" => include_bytes!("../../ui/assets/fonts/Inter-SemiBold.ttf"),
        "mono-400.ttf" => include_bytes!("../../ui/assets/fonts/JetBrainsMono-Regular.ttf"),
        "mono-500.ttf" => include_bytes!("../../ui/assets/fonts/JetBrainsMono-Medium.ttf"),
        "mono-600.ttf" => include_bytes!("../../ui/assets/fonts/JetBrainsMono-SemiBold.ttf"),
        _ => {
            return [&tree::INCARNATION, &tree::ETHER]
                .into_iter()
                .find(|tree| tree.file == name)
                .map(|tree| Cow::Borrowed(tree.backdrop().as_bytes()));
        }
    };
    Some(Cow::Borrowed(bytes))
}

pub fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next() {
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    }
}
