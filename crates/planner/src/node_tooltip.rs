use hsplanner_ui::i18n::{tr, trf};
use std::collections::HashMap;

use gpui_kit::base::Tooltip;
use gpui_kit::component::animation::{EffectTransition, cubic_bezier};
use gpui_kit::{
    App, BoxShadow, Div, FontWeight, Hsla, IntoElement, ParentElement, RenderOnce, Styled, Window,
    div, linear_color_stop, linear_gradient, point, px, relative, rems,
};
use serde::Deserialize;

use crate::{
    build_panel::preview_changes,
    build_session::PreviewResult,
    theme::TooltipTheme,
    tooltip_text::TooltipText,
    tree::{Info, Node},
};

#[derive(Clone, Deserialize)]
pub struct NodeLines {
    pub parsed: Vec<String>,
    pub unsupported: Vec<String>,
}

pub fn load_lines() -> HashMap<usize, NodeLines> {
    serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/node-lines.json")))
        .expect("engine-classified tree descriptions")
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tone {
    Neutral,
    Rare,
    Angelic,
}

fn tier(node: &Node, info: Option<&Info>) -> (&'static str, Tone) {
    match info.map(|info| info.n.as_str()) {
        Some("jewelry") => (tr("tree.tier.jewelry"), Tone::Rare),
        Some("warp") => (tr("tree.tier.warp"), Tone::Rare),
        Some("root") => (tr("tree.tier.start"), Tone::Angelic),
        Some("big") => (tr("tree.tier.notable"), Tone::Rare),
        _ if node.r >= 12. => (tr("tree.tier.keystone"), Tone::Rare),
        _ if node.r >= 10. => (tr("tree.tier.minor"), Tone::Rare),
        _ => (tr("tree.tier.minor"), Tone::Neutral),
    }
}

#[derive(IntoElement)]
pub struct NodeTooltip {
    node: Node,
    info: Option<Info>,
    lines: Option<NodeLines>,
    effects: bool,
    preview: Option<PreviewResult>,
    pending: bool,
    socket: Option<hsplanner_engine::calc::types::TreeSocketContent>,
    allocated: bool,
}

impl NodeTooltip {
    pub fn new(node: &Node, info: Option<&Info>, lines: Option<&NodeLines>) -> Self {
        Self {
            node: node.clone(),
            info: info.cloned(),
            lines: lines.cloned(),
            effects: true,
            preview: None,
            pending: false,
            socket: None,
            allocated: false,
        }
    }

    pub fn socket(
        mut self,
        content: Option<&hsplanner_engine::calc::types::TreeSocketContent>,
        allocated: bool,
    ) -> Self {
        self.socket = content.cloned();
        self.allocated = allocated;
        self
    }

    pub fn effects(mut self, enabled: bool) -> Self {
        self.effects = enabled;
        self
    }

    pub fn performance(mut self, preview: Option<&PreviewResult>, pending: bool) -> Self {
        self.preview = preview.cloned();
        self.pending = pending;
        self
    }
}

fn section(palette: &TooltipTheme) -> Div {
    div()
        .px_3()
        .py_2()
        .border_t_1()
        .border_color(palette.border.opacity(0.7))
}

fn text(content: impl Into<gpui_kit::SharedString>, color: Hsla) -> Div {
    div()
        .text_size(rems(12. / 13.))
        .line_height(relative(1.55))
        .text_color(color)
        .child(content.into())
}

impl RenderOnce for NodeTooltip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = cx.global::<TooltipTheme>();
        let title_tracking = if self.effects {
            palette.title_tracking
        } else {
            0.
        };
        let label_tracking = if self.effects {
            palette.label_tracking
        } else {
            0.
        };
        let tag_tracking = if self.effects {
            palette.tag_tracking
        } else {
            0.
        };
        let fade = palette.tooltip_fade;
        let (label, tone) = tier(&self.node, self.info.as_ref());
        let (accent, title_color, border) = match tone {
            Tone::Neutral => (palette.neutral, palette.text, palette.border_strong),
            Tone::Rare => (
                palette.accent,
                palette.accent_hot,
                palette.accent.opacity(0.6),
            ),
            Tone::Angelic => (
                palette.angelic,
                palette.angelic,
                palette.angelic.opacity(0.6),
            ),
        };
        let rem = window.rem_size();
        let mut panel = Tooltip::new(("tree-node-tooltip", self.node.id))
            .flex()
            .flex_col()
            .min_w(rems(240. / 13.))
            .max_w(rems(480. / 13.))
            .bg(palette.panel)
            .line_height(relative(1.5))
            .text_color(palette.text)
            .rounded(rems(4. / 13.))
            .border_1()
            .border_color(border)
            .overflow_hidden()
            .shadow(vec![BoxShadow {
                color: palette.shadow.opacity(0.8),
                offset: point(px(0.), rem * (8. / 13.)),
                blur_radius: rem * (32. / 13.),
                spread_radius: px(0.),
                inset: false,
            }])
            .child(
                div()
                    .px_3()
                    .py_2()
                    .bg(linear_gradient(
                        180.,
                        linear_color_stop(accent.opacity(0.14), 0.),
                        linear_color_stop(accent.opacity(0.04), 1.),
                    ))
                    .child(
                        div()
                            .text_size(rems(1.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .line_height(relative(1.25))
                            .text_color(title_color)
                            .child(
                                TooltipText::new(
                                    "tooltip-title",
                                    self.info
                                        .as_ref()
                                        .map(|info| info.t.clone())
                                        .unwrap_or_else(|| {
                                            trf(
                                                "tree.node_name",
                                                &[("id", self.node.id.to_string())],
                                            )
                                        }),
                                    title_tracking,
                                )
                                .glow(self.effects.then_some(accent)),
                            ),
                    )
                    .child(
                        div()
                            .mt_0p5()
                            .text_size(rems(10. / 13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(palette.faint)
                            .child(TooltipText::new(
                                "tooltip-tier",
                                label.to_uppercase(),
                                label_tracking,
                            )),
                    ),
            );

        if let Some(info) = self.info {
            if info.n == "jewelry" {
                let (title, lines) = crate::tree_jewelry::description(self.socket.as_ref());
                panel = panel.child(
                    section(palette)
                        .child(
                            div()
                                .mx(rems(-0.75))
                                .mt(rems(-0.5))
                                .mb_2()
                                .px_3()
                                .py_1()
                                .bg(palette.accent.opacity(0.06))
                                .border_b_1()
                                .border_color(palette.border.opacity(0.4))
                                .text_size(rems(10. / 13.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(palette.accent_hot.opacity(0.85))
                                .child(TooltipText::new(
                                    "tooltip-socketed",
                                    tr("tree.socketed"),
                                    label_tracking,
                                )),
                        )
                        .child(text(title, palette.accent_hot))
                        .children(lines.into_iter().map(|line| text(line, palette.text)))
                        .child(
                            text(
                                if self.allocated {
                                    tr("tree.edit_socket")
                                } else {
                                    tr("tree.activate_socket")
                                },
                                palette.faint,
                            )
                            .italic(),
                        ),
                );
            } else {
                if let Some(lines) = self.lines {
                    if !lines.parsed.is_empty() {
                        panel = panel.child(
                            section(palette).flex().flex_col().gap_0p5().children(
                                lines
                                    .parsed
                                    .into_iter()
                                    .map(|line| text(line, palette.text.opacity(0.9))),
                            ),
                        );
                    }
                    if !lines.unsupported.is_empty() {
                        panel = panel.child(
                            section(palette)
                                .child(
                                    div()
                                        .mb_1()
                                        .text_size(rems(10. / 13.))
                                        .text_color(palette.muted)
                                        .child(TooltipText::new(
                                            "tooltip-unsupported",
                                            tr("tree.unsupported"),
                                            label_tracking,
                                        )),
                                )
                                .child(
                                    div().flex().flex_col().gap_0p5().children(
                                        lines
                                            .unsupported
                                            .into_iter()
                                            .map(|line| text(line, palette.text.opacity(0.54))),
                                    ),
                                )
                                .child(
                                    div()
                                        .mt_1()
                                        .text_size(rems(10. / 13.))
                                        .italic()
                                        .text_color(palette.muted.opacity(0.7))
                                        .child(tr("tree.unsupported_help")),
                                ),
                        );
                    }
                }
                if let Some(note) = info.note {
                    panel = panel
                        .child(section(palette).child(text(note, palette.accent_hot).italic()));
                }
                panel = panel.child(section(palette).child(preview_changes(
                    self.preview.as_ref(),
                    self.pending,
                    None,
                    cx,
                )));
            }
            if !info.g.is_empty() {
                panel = panel.child(
                    section(palette)
                        .bg(palette.panel_secondary.opacity(0.4))
                        .flex()
                        .flex_wrap()
                        .gap_1()
                        .children(info.g.into_iter().map(|tag| {
                            div()
                                .px_1p5()
                                .py_0p5()
                                .rounded(rems(2. / 13.))
                                .border_1()
                                .border_color(palette.border_strong)
                                .text_size(rems(10. / 13.))
                                .text_color(palette.muted)
                                .child(TooltipText::new(
                                    gpui_kit::SharedString::from(format!("tooltip-tag-{tag}")),
                                    tag.to_uppercase(),
                                    tag_tracking,
                                ))
                        })),
                );
            }
        } else {
            panel = panel.child(section(palette).child(text(tr("tree.no_data"), palette.faint)));
        }
        if self.effects {
            EffectTransition::new(fade)
                .ease(cubic_bezier(0., 0., 0.58, 1.))
                .fade(0., 1.)
                .apply(panel, ("tooltip-enter", self.node.id))
                .into_any_element()
        } else {
            panel.into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Graph;

    #[test]
    fn classification_covers_every_node_and_keeps_original_lines() {
        let graph = Graph::load();
        let lines = load_lines();
        assert_eq!(lines.len(), graph.info.len());
        for (id, info) in &graph.info {
            let groups = &lines[id];
            for line in groups.parsed.iter().chain(&groups.unsupported) {
                assert!(info.l.contains(line), "altered description for node {id}");
            }
            assert!(
                groups
                    .parsed
                    .iter()
                    .all(|line| !groups.unsupported.contains(line))
            );
        }
        assert!(lines.values().any(|lines| !lines.unsupported.is_empty()));
        assert_eq!(lines[&853].parsed.len(), 1);
        assert_eq!(
            lines[&853].unsupported,
            ["+2500 Lightning Damage dealt by odin"]
        );
    }

    #[test]
    fn special_nodes_keep_the_frontend_tier_and_tone() {
        let graph = Graph::load();
        for node in &graph.nodes {
            let info = &graph.info[&node.id];
            match info.n.as_str() {
                "root" => assert_eq!(tier(node, Some(info)), ("Starting Node", Tone::Angelic)),
                "jewelry" => assert_eq!(tier(node, Some(info)), ("Jewelry Socket", Tone::Rare)),
                "big" => assert_eq!(tier(node, Some(info)), ("Notable", Tone::Rare)),
                _ => {}
            }
        }
    }
}
