use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{App, Global, Hsla, px, rgb};
use std::{borrow::Cow, time::Duration};

pub const FONT_FAMILY: &str = "Inter";
pub const MONO_FONT_FAMILY: &str = "JetBrains Mono";

/// Canvas and floating-control roles from the shipping TreeView/EtherView.
/// Node dimensions are game-space geometry; controls use the window's rem scale.
#[derive(Clone, Copy)]
pub struct TreeTheme {
    background: Hsla,
    node: Hsla,
    root: Hsla,
    allocated: Hsla,
    keystone: Hsla,
    preview: Hsla,
    stroke: Hsla,
    notable_stroke: Hsla,
    keystone_stroke: Hsla,
    root_stroke: Hsla,
    allocated_stroke: Hsla,
    accent: Hsla,
    socket: Hsla,
    edge: Hsla,
    allocated_edge: Hsla,
    preview_edge: Hsla,
    surface: Hsla,
    surface_end: Hsla,
    control: Hsla,
    control_end: Hsla,
    vignette_opacity: f32,
    search_opacity: f32,
}

impl TreeTheme {
    pub fn background(self) -> Hsla {
        self.background
    }

    pub fn node(self) -> Hsla {
        self.node
    }

    pub fn root(self) -> Hsla {
        self.root
    }

    pub fn allocated(self) -> Hsla {
        self.allocated
    }

    pub fn keystone(self) -> Hsla {
        self.keystone
    }

    pub fn preview(self) -> Hsla {
        self.preview
    }

    pub fn stroke(self) -> Hsla {
        self.stroke
    }

    pub fn notable_stroke(self) -> Hsla {
        self.notable_stroke
    }

    pub fn keystone_stroke(self) -> Hsla {
        self.keystone_stroke
    }

    pub fn root_stroke(self) -> Hsla {
        self.root_stroke
    }

    pub fn allocated_stroke(self) -> Hsla {
        self.allocated_stroke
    }

    pub fn accent(self) -> Hsla {
        self.accent
    }

    pub fn socket(self) -> Hsla {
        self.socket
    }

    pub fn edge(self) -> Hsla {
        self.edge
    }

    pub fn allocated_edge(self) -> Hsla {
        self.allocated_edge
    }

    pub fn preview_edge(self) -> Hsla {
        self.preview_edge
    }

    pub fn surface(self) -> Hsla {
        self.surface
    }

    pub fn surface_end(self) -> Hsla {
        self.surface_end
    }

    pub fn control(self) -> Hsla {
        self.control
    }

    pub fn control_end(self) -> Hsla {
        self.control_end
    }

    pub fn vignette_opacity(self) -> f32 {
        self.vignette_opacity
    }

    pub fn search_opacity(self) -> f32 {
        self.search_opacity
    }

    pub fn incarnation() -> Self {
        Self {
            background: rgb(0x0a0b0f).into(),
            node: rgb(0x1c1d24).into(),
            root: rgb(0x3a3528).into(),
            allocated: rgb(0xc9a55a).into(),
            keystone: rgb(0xe94f37).into(),
            preview: rgb(0x5a4528).into(),
            stroke: rgb(0x3a3528).into(),
            notable_stroke: rgb(0x5a5448).into(),
            keystone_stroke: rgb(0x8a6f3a).into(),
            root_stroke: rgb(0xc9a55a).into(),
            allocated_stroke: rgb(0xd4cfbf).into(),
            accent: rgb(0xe0b864).into(),
            socket: rgb(0xffd66b).opacity(0.85).into(),
            edge: rgb(0x2a2f3a).into(),
            allocated_edge: rgb(0xc48a3a).into(),
            preview_edge: rgb(0x8a6a2a).into(),
            surface: rgb(0x1c1d24).opacity(0.88).into(),
            surface_end: rgb(0x0d0e12).opacity(0.82).into(),
            control: rgb(0x2a2418).into(),
            control_end: rgb(0x1a1410).into(),
            vignette_opacity: 0.55,
            search_opacity: 0.25,
        }
    }

    pub fn ether() -> Self {
        let purple: Hsla = rgb(0xa574c9).into();
        Self {
            root: rgb(0x2e2838).into(),
            allocated: purple,
            keystone: purple,
            preview: purple.opacity(0.28),
            accent: purple,
            edge: rgb(0x262b36).into(),
            allocated_edge: rgb(0x8a5fb0).into(),
            preview_edge: rgb(0x5a4a6e).into(),
            control: rgb(0x221a2c).into(),
            control_end: rgb(0x16121c).into(),
            ..Self::incarnation()
        }
    }
}

/// Region labels and colors match the Ether summary in the reference app.
pub fn ether_region(key: &str) -> (&'static str, Hsla) {
    let regions = [
        ("Un", "Universal", 0xa574c9),
        ("Ow", "Overworld", 0x7fd966),
        ("Ct", "Chaos Tower", 0xe05c5c),
        ("Cp", "Chaos Pillars", 0xe0985c),
        ("SR", "Shadow Realm", 0x7a8ce0),
        ("Pe", "Prime Evil", 0xc94f6d),
        ("Ur", "Unstable Rift", 0x5cd8d0),
        ("Min", "Mining", 0xc98a3a),
        ("EB", "Eternal Battlefield", 0xb8b04a),
        ("CS", "Cursed Spirit", 0x66d9a8),
        ("US", "Unholy Siege", 0xe07adb),
        ("Dng", "Dungeons", 0x8f9bb0),
        ("Rg", "Ruby Gardens", 0xf27a9d),
        ("Cc", "Colossal Creatures", 0xe8d84a),
    ];
    let region = key.strip_prefix("ether").unwrap_or(key);
    regions
        .into_iter()
        .find_map(|(prefix, name, color)| {
            region
                .strip_prefix(prefix)
                .filter(|suffix| suffix.starts_with("Small") || suffix.starts_with("Big"))
                .map(|_| (name, rgb(color).into()))
        })
        .unwrap_or(("Other", rgb(0x9aa0ab).into()))
}

// Product colour roles for tooltip tones.
pub struct TooltipTheme {
    pub background: Hsla,
    pub panel: Hsla,
    pub panel_secondary: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub faint: Hsla,
    pub accent: Hsla,
    pub accent_hot: Hsla,
    pub accent_deep: Hsla,
    pub angelic: Hsla,
    pub neutral: Hsla,
    pub shadow: Hsla,
    pub positive: Hsla,
    pub negative: Hsla,
    pub stat_orange: Hsla,
    pub stat_blue: Hsla,
    pub synergy: Hsla,
    pub source_item: Hsla,
    pub source_socket: Hsla,
    pub source_skill: Hsla,
    pub source_tree: Hsla,
    pub source_attribute: Hsla,
    pub source_custom: Hsla,
    pub title_tracking: f32,
    pub label_tracking: f32,
    pub tag_tracking: f32,
    pub title_shadows: [(f32, f32); 2],
    pub tooltip_fade: Duration,
}

impl Global for TooltipTheme {}

pub fn init(cx: &mut App) {
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Regular.ttf")),
            Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Medium.ttf")),
            Cow::Borrowed(include_bytes!("../assets/fonts/Inter-SemiBold.ttf")),
            Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Bold.ttf")),
            Cow::Borrowed(include_bytes!("../assets/fonts/Inter-Italic.ttf")),
            Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")),
            Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf")),
            Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-SemiBold.ttf")),
        ])
        .expect("load embedded Inter and JetBrains Mono fonts");
    Theme::change(ThemeMode::Dark, None, cx);
    let palette = TooltipTheme {
        background: rgb(0x0d0e12).into(),
        panel: rgb(0x15161b).into(),
        panel_secondary: rgb(0x1c1d24).into(),
        border: rgb(0x2a2b35).into(),
        border_strong: rgb(0x363742).into(),
        text: rgb(0xd4cfbf).into(),
        muted: rgb(0xa39a8a).into(),
        faint: rgb(0x8f8676).into(),
        accent: rgb(0xc9a55a).into(),
        accent_hot: rgb(0xe0b864).into(),
        accent_deep: rgb(0x8a6f3a).into(),
        angelic: rgb(0xfef08a).into(),
        neutral: rgb(0xa0a0a0).into(),
        shadow: rgb(0x000000).into(),
        positive: rgb(0x74c98a).into(),
        negative: rgb(0xd96b5a).into(),
        stat_orange: rgb(0xd99a5a).into(),
        stat_blue: rgb(0x5a8fc9).into(),
        synergy: rgb(0xa78bfa).into(),
        source_item: rgb(0x22d3ee).into(),
        source_socket: rgb(0xf472b6).into(),
        source_skill: rgb(0xfde047).into(),
        source_tree: rgb(0xfbbf24).into(),
        source_attribute: rgb(0x34d399).into(),
        source_custom: rgb(0xfb923c).into(),
        title_tracking: 0.02,
        label_tracking: 0.12,
        tag_tracking: 0.08,
        title_shadows: [(10., 0.45), (4., 0.25)],
        tooltip_fade: Duration::from_millis(80),
    };
    let theme = Theme::global_mut(cx);
    theme.font_size = px(13.);
    theme.font_family = FONT_FAMILY.into();
    theme.radius = px(3.);
    theme.colors.ring = palette.accent_deep;
    theme.colors.popover = palette.panel;
    theme.colors.popover_foreground = palette.text;
    theme.colors.primary = palette.accent;
    theme.colors.primary_foreground = palette.accent_hot;
    theme.colors.foreground = palette.text;
    theme.colors.background = palette.panel;
    theme.colors.border = palette.border;
    theme.colors.input = palette.border_strong;
    theme.colors.caret = palette.accent_hot;
    theme.colors.muted = palette.panel_secondary;
    theme.colors.muted_foreground = palette.muted;
    theme.colors.button = palette.panel_secondary;
    theme.colors.button_foreground = palette.text;
    theme.colors.button_hover = palette.border;
    theme.colors.button_active = palette.border_strong;
    theme.colors.button_primary = palette.accent;
    theme.colors.button_primary_foreground = palette.panel;
    theme.colors.button_primary_hover = palette.accent_hot;
    theme.colors.button_primary_active = palette.accent;
    theme.colors.link = palette.accent_hot;
    theme.colors.link_hover = palette.angelic;
    theme.colors.list = palette.panel;
    theme.colors.list_hover = palette.panel_secondary;
    theme.colors.list_active = palette.border;
    theme.colors.list_active_border = palette.accent;
    theme.colors.slider_bar = palette.accent_deep;
    theme.colors.slider_thumb = palette.accent_deep;
    theme.colors.overlay = palette.background.opacity(0.65);
    theme.colors.scrollbar_thumb = palette.border_strong;
    theme.colors.scrollbar_thumb_hover = palette.muted;
    theme.colors.selection = palette.accent_deep.opacity(0.35);
    theme.mono_font_family = MONO_FONT_FAMILY.into();
    theme.mono_font_size = px(13.);
    // Components read resolved backgrounds as well as legacy solid colors.
    // Synchronizing only Base leaves sliders/checked controls at framework defaults.
    theme.tokens = theme.colors.into();
    theme.tokens.primary =
        gpui_kit::component::theme::ThemeToken::new(palette.accent, chrome_gold_surface());
    theme.tokens.slider_thumb =
        gpui_kit::component::theme::ThemeToken::new(palette.accent_deep, chrome_gold_surface());
    Theme::sync_base(cx);
    cx.set_global(palette);
}

/// Gold command surface used by the Tauri reference header.
pub fn chrome_gold_surface() -> gpui_kit::Background {
    gpui_kit::linear_gradient(
        180.,
        gpui_kit::linear_color_stop(rgb(0x3a2f1a), 0.),
        gpui_kit::linear_color_stop(rgb(0x2a2418), 1.),
    )
}

/// Opaque neutral emphasis; gold is reserved for text and selection markers.
pub fn library_highlight(cx: &App) -> gpui_kit::Background {
    let p = cx.global::<TooltipTheme>();
    gpui_kit::linear_gradient(
        180.,
        gpui_kit::linear_color_stop(p.panel_secondary, 0.),
        gpui_kit::linear_color_stop(p.panel, 1.),
    )
}

pub fn class_color(id: &str) -> Hsla {
    let hash = id
        .encode_utf16()
        .fold(0i32, |hash, c| hash.wrapping_mul(31).wrapping_add(c as i32));
    gpui_kit::hsla((hash.unsigned_abs() % 360) as f32 / 360., 0.58, 0.58, 1.)
}

pub fn mana_color() -> Hsla {
    rgb(0x5a8fc9).into()
}

/// Attribute and defense roles from the reference theme.
pub fn stat_color(key: &str, cx: &App) -> Hsla {
    match key {
        "strength" | "orange" => rgb(0xd99a5a).into(),
        "dexterity" | "green" => rgb(0x74c98a).into(),
        "intelligence" | "purple" => rgb(0xa574c9).into(),
        "energy" | "mana" | "blue" => mana_color(),
        "vitality" | "life" | "red" => rgb(0xd96b5a).into(),
        "cyan" => rgb(0x5ac9c0).into(),
        "fire" | "cold" | "lightning" | "poison" | "arcane" | "physical" => damage_color(key),
        "armor" => cx.global::<TooltipTheme>().text,
        _ => cx.global::<TooltipTheme>().muted,
    }
}

/// Damage-type RGB values.
pub fn damage_color(key: &str) -> Hsla {
    rgb(match key {
        "fire" => 0xf87171,
        "cold" => 0x38bdf8,
        "lightning" => 0xfacc15,
        "poison" => 0x4ade80,
        "arcane" => 0xc084fc,
        "explosion" => 0xfb923c,
        "magic" => 0xf472b6,
        _ => 0xd4cfbf,
    })
    .into()
}

pub fn inventory_surface() -> gpui_kit::Background {
    gpui_kit::linear_gradient(
        180.,
        gpui_kit::linear_color_stop(rgb(0x0c0804), 0.),
        gpui_kit::linear_color_stop(rgb(0x070302), 1.),
    )
}

pub fn inventory_cell(blocked: bool) -> Hsla {
    rgb(if blocked { 0x070403 } else { 0x120c08 }).into()
}

pub fn inventory_border(blocked: bool) -> Hsla {
    rgb(if blocked { 0x3a2a18 } else { 0x5a4528 }).into()
}

pub fn rarity_color(rarity: &str, cx: &App) -> Hsla {
    match rarity {
        "rare" => cx.global::<TooltipTheme>().accent_hot,
        "uncommon" => rgb(0x38bdf8).into(),
        "mythic" => rgb(0xc084fc).into(),
        "satanic" => rgb(0xef4444).into(),
        "heroic" => rgb(0x4ade80).into(),
        "angelic" => rgb(0xfef08a).into(),
        "satanic_set" => rgb(0xa3e635).into(),
        "unholy" => rgb(0xf472b6).into(),
        "relic" => rgb(0xfdba74).into(),
        _ => rgb(0xffffff).into(),
    }
}

pub const UI_ZOOM_STEPS: [f32; 6] = [1., 1.15, 1.25, 1.5, 1.75, 2.];

pub fn normalize_zoom(zoom: f32) -> f32 {
    UI_ZOOM_STEPS
        .into_iter()
        .find(|step| (*step - zoom).abs() < 0.001)
        .unwrap_or(1.)
}

pub fn apply_zoom(zoom: f32, cx: &mut App) {
    let size = px(13. * normalize_zoom(zoom));
    let theme = Theme::global_mut(cx);
    theme.font_size = size;
    theme.mono_font_size = size;
    theme.radius = px(3. * normalize_zoom(zoom));
    Theme::sync_base(cx);
    cx.refresh_windows();
}

/// Item rarity washes from the reference's Tailwind 4 palette (sRGB projection).
pub fn rarity_surface(rarity: &str, cx: &App) -> Hsla {
    let color: Hsla = match rarity {
        "common" => return rgb(0xffffff).opacity(0.05).into(),
        "uncommon" => rgb(0x00a6f4).into(),
        "rare" => rgb(0xf0b100).into(),
        "mythic" => rgb(0xad46ff).into(),
        "satanic" => rgb(0xfb2c36).into(),
        "heroic" => rgb(0x00c950).into(),
        "angelic" => rgb(0xfdc700).into(),
        "satanic_set" => rgb(0x7ccf00).into(),
        "unholy" => rgb(0xf6339a).into(),
        "relic" => rgb(0xff6900).into(),
        _ => cx.global::<TooltipTheme>().panel_secondary,
    };
    color.opacity(0.1)
}

/// Attribute, condition and override rows from the Tauri Config panels.
pub fn config_tile_surface(selected: bool, cx: &App) -> gpui_kit::Background {
    let p = cx.global::<TooltipTheme>();
    gpui_kit::linear_gradient(
        180.,
        gpui_kit::linear_color_stop(
            if selected {
                gpui_kit::Hsla::from(rgb(0x3a2e18)).opacity(0.5)
            } else {
                p.panel_secondary
            },
            0.,
        ),
        gpui_kit::linear_color_stop(
            if selected {
                p.panel_secondary.opacity(0.5)
            } else {
                p.background.opacity(0.7)
            },
            1.,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_imported_zoom_cannot_collapse_or_expand_the_window() {
        for value in [0., -1., f32::NAN, f32::INFINITY, 1.37] {
            assert_eq!(normalize_zoom(value), 1.);
        }
        for value in UI_ZOOM_STEPS {
            assert_eq!(normalize_zoom(value), value);
        }
    }
}
