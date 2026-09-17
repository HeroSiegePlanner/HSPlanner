//! Retained jewelry editor; mutations reach the shared document only on Apply.
use super::*;
use crate::skill_details::{format_value, stat_name};
use gpui_kit::base::{Disableable, Selectable};
use gpui_kit::component::{
    WindowExt,
    button::{Button, ButtonCustomVariant, ButtonVariants},
    input::{Input, InputEvent, InputState},
};
use gpui_kit::{prelude::*, *};
use hsplanner_build::{gear::jewel_affix_allowed, jewelry};
use hsplanner_engine::calc::{
    affix::rolled_affix_value,
    data,
    types::{Affix, AffixSign, EquippedAffix, TreeSocketContent},
};
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::{
    components::{modal_eyebrow, modal_footer, modal_header, modal_status},
    controls::{self, ButtonTone, modal_button},
    theme::MONO_FONT_FAMILY,
    tooltip_text::TooltipText,
};

pub(super) fn description(content: Option<&TreeSocketContent>) -> (String, Vec<String>) {
    match content {
        None => (tr("gear.empty_socket").into(), vec![]),
        Some(TreeSocketContent::Item { id }) => {
            let (name, stats) = match data::get_socketable_by_id(id) {
                Some(data::Socketable::Gem(g)) => (&g.name, &g.stats),
                Some(data::Socketable::Rune(r)) => (&r.name, &r.stats),
                None => {
                    return (
                        trf("jewelry.unknown_socketable", &[("id", (id).to_string())]),
                        vec![],
                    );
                }
            };
            let mut values: Vec<_> = stats.iter().collect();
            values.sort_by_key(|(key, _)| *key);
            (
                name.clone(),
                values
                    .into_iter()
                    .map(|(key, &value)| {
                        format!("{} {}", format_value(value, key, true), stat_name(key))
                    })
                    .collect(),
            )
        }
        Some(TreeSocketContent::Uncut { affixes }) => (
            tr("jewelry.uncut_jewel").into(),
            affixes
                .iter()
                .map(|eq| match data::get_affix(&eq.affix_id) {
                    Some(def) => {
                        let key = def.stat_key.as_deref().unwrap_or("");
                        trf(
                            "jewelry.stat_tier",
                            &[
                                (
                                    "arg0",
                                    (format_value(
                                        eq.custom_value
                                            .unwrap_or_else(|| rolled_affix_value(def, eq.roll)),
                                        key,
                                        true,
                                    ))
                                    .to_string(),
                                ),
                                ("arg1", (stat_name(key)).to_string()),
                                ("arg2", (def.tier).to_string()),
                            ],
                        )
                    }
                    None => trf(
                        "jewelry.unknown_affix",
                        &[("arg0", (eq.affix_id).to_string())],
                    ),
                })
                .collect(),
        ),
    }
}

impl TreeView {
    pub(super) fn open_jewelry(
        &mut self,
        node_id: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.scene.graph.kind != TreeKind::Incarnation
            || self.progression.is_preview()
            || !jewelry::can_edit(self.session.read(cx).snapshot(), node_id)
        {
            return;
        }
        self.hovered = None;
        self.example = None;
        self.drag = None;
        let session = self.session.clone();
        let editor = cx.new(|cx| JewelryEditor::new(session, node_id, window, cx));
        let focus = self.focus.clone();
        window.open_dialog(cx, move |dialog, window, cx| {
            let focus = focus.clone();
            let palette = cx.global::<theme::TooltipTheme>();
            dialog
                .p_0()
                .gap_0()
                .rounded_xl()
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(palette.panel_secondary, 0.),
                    linear_color_stop(palette.background, 1.),
                ))
                .width((window.rem_size() * (640. / 13.)).min(window.viewport_size().width * 0.94))
                .h(window.viewport_size().height * 0.88)
                .margin_top(window.viewport_size().height * 0.06)
                .overlay_closable(false)
                .on_ok(|_, _, _| false)
                .on_close(move |_, window, cx| window.focus(&focus, cx))
                .child(editor.clone())
        });
        cx.notify();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Items,
    Uncut,
}
struct Choice {
    id: String,
    name: String,
    stats: String,
    search: String,
    kind: &'static str,
    tier: u32,
}
struct AffixInput {
    group: String,
    input: Entity<InputState>,
    _subscription: Subscription,
}
struct JewelryEditor {
    session: Entity<Session>,
    document: DocumentKey,
    node_id: u32,
    original: Option<TreeSocketContent>,
    pending: Option<TreeSocketContent>,
    tab: Tab,
    adding: bool,
    search: Entity<InputState>,
    catalog: Vec<Choice>,
    choices: Vec<usize>,
    list: UniformListScrollHandle,
    affix_inputs: Vec<AffixInput>,
    error: Option<String>,
    invalid_rolls: HashMap<String, String>,
    _subscription: Subscription,
    _locale_subscription: Subscription,
}

fn jewel_tiers(group: &str) -> Vec<&'static Affix> {
    let mut tiers: Vec<_> = data::data()
        .affixes
        .values()
        .filter(|a| a.group_id == group && jewel_affix_allowed(a))
        .collect();
    tiers.sort_by_key(|a| a.tier);
    tiers
}

fn mono(size: f32, color: Hsla) -> Div {
    div()
        .font_family(MONO_FONT_FAMILY)
        .text_size(rems(size / 13.))
        .text_color(color)
}

impl JewelryEditor {
    fn new(
        session: Entity<Session>,
        node_id: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let document = DocumentKey::from_session(session.read(cx));
        let original = session
            .read(cx)
            .snapshot()
            .tree_socketed
            .get(&node_id)
            .cloned();
        let tab = if matches!(original, Some(TreeSocketContent::Uncut { .. })) {
            Tab::Uncut
        } else {
            Tab::Items
        };
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(tr("jewelry.search")));
        let subscription = cx.subscribe(&search, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.filter(cx);
                cx.notify();
            }
        });
        let locale_subscription =
            cx.observe_global_in::<hsplanner_ui::i18n::Locale>(window, |this, window, cx| {
                this.search.update(cx, |input, cx| {
                    input.set_placeholder(tr("jewelry.search"), window, cx)
                });
                for row in &mut this.catalog {
                    if row.kind == "Affix" {
                        continue;
                    }
                    let (_, lines) =
                        description(Some(&TreeSocketContent::Item { id: row.id.clone() }));
                    row.stats = if lines.is_empty() {
                        "—".to_owned()
                    } else {
                        lines.join(", ")
                    };
                    row.search = format!("{} {}", row.name, row.stats).to_lowercase();
                }
                if let Some(TreeSocketContent::Uncut { affixes }) = &this.pending {
                    for state in &this.affix_inputs {
                        if let Some(def) = affixes
                            .iter()
                            .filter_map(|eq| data::get_affix(&eq.affix_id))
                            .find(|def| def.group_id == state.group)
                        {
                            let placeholder = trf(
                                "gear.named_roll",
                                &[("arg0", stat_name(def.stat_key.as_deref().unwrap_or("")))],
                            );
                            state.input.update(cx, |input, cx| {
                                input.set_placeholder(placeholder, window, cx)
                            });
                        }
                    }
                }
                this.filter(cx);
                cx.notify();
            });
        let mut editor = Self {
            session,
            document,
            node_id,
            pending: original.clone(),
            original,
            tab,
            adding: false,
            search,
            catalog: vec![],
            choices: vec![],
            list: UniformListScrollHandle::new(),
            affix_inputs: vec![],
            error: None,
            invalid_rolls: HashMap::new(),
            _subscription: subscription,
            _locale_subscription: locale_subscription,
        };
        editor.catalog = data::data()
            .gems
            .values()
            .map(|g| {
                (
                    &g.id,
                    &g.name,
                    g.tier,
                    if g.name.to_lowercase().contains("jewel") {
                        "Jewel"
                    } else {
                        "Gem"
                    },
                )
            })
            .chain(
                data::data()
                    .runes
                    .values()
                    .map(|r| (&r.id, &r.name, r.tier, "Rune")),
            )
            .map(|(id, name, tier, kind)| {
                let (_, lines) = description(Some(&TreeSocketContent::Item { id: id.clone() }));
                let stats = if lines.is_empty() {
                    "—".to_owned()
                } else {
                    lines.join(", ")
                };
                Choice {
                    id: id.clone(),
                    name: name.clone(),
                    search: format!("{name} {stats}").to_lowercase(),
                    stats,
                    kind,
                    tier,
                }
            })
            .collect();
        let mut groups = HashSet::new();
        for def in data::data()
            .affixes
            .values()
            .filter(|a| jewel_affix_allowed(a))
        {
            if !groups.insert(def.group_id.clone()) {
                continue;
            }
            let tiers = jewel_tiers(&def.group_id);
            if let Some(top) = tiers.last() {
                editor.catalog.push(Choice {
                    id: top.id.clone(),
                    name: top.description.clone(),
                    stats: String::new(),
                    search: top.description.to_lowercase(),
                    kind: "Affix",
                    tier: tiers.len() as u32,
                });
            }
        }
        editor.catalog.sort_by(|a, b| {
            a.kind
                .cmp(b.kind)
                .then(a.tier.cmp(&b.tier))
                .then(a.name.cmp(&b.name))
                .then(a.id.cmp(&b.id))
        });
        editor.rebuild_affix_inputs(window, cx);
        editor.filter(cx);
        editor
    }

    fn filter(&mut self, cx: &App) {
        let query = self.search.read(cx).value().trim().to_lowercase();
        let used: HashSet<_> = match &self.pending {
            Some(TreeSocketContent::Uncut { affixes }) => affixes
                .iter()
                .filter_map(|a| data::get_affix(&a.affix_id))
                .map(|a| &a.group_id)
                .collect(),
            _ => HashSet::new(),
        };
        self.choices = self
            .catalog
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                (self.tab == Tab::Uncut) == (row.kind == "Affix")
                    && row.search.contains(&query)
                    && (row.kind != "Affix"
                        || data::get_affix(&row.id).is_some_and(|a| !used.contains(&a.group_id)))
            })
            .map(|(i, _)| i)
            .collect();
        self.list.scroll_to_item(0, ScrollStrategy::Top);
    }

    fn rebuild_affix_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.affix_inputs.clear();
        self.invalid_rolls.clear();
        let Some(TreeSocketContent::Uncut { affixes }) = &self.pending else {
            return;
        };
        for eq in affixes {
            let Some(def) = data::get_affix(&eq.affix_id) else {
                continue;
            };
            let group = def.group_id.clone();
            let value = eq
                .custom_value
                .map(|v| if def.sign == AffixSign::Minus { -v } else { v })
                .unwrap_or_else(|| {
                    def.value_min.unwrap_or(0.)
                        + (def.value_max.unwrap_or(0.) - def.value_min.unwrap_or(0.)) * eq.roll
                });
            let input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(trf(
                        "gear.named_roll",
                        &[(
                            "arg0",
                            (stat_name(def.stat_key.as_deref().unwrap_or(""))).to_string(),
                        )],
                    ))
                    .default_value(format!("{}", (value * 100.).round() / 100.))
            });
            let id = group.clone();
            let subscription = cx.subscribe(&input, move |this, input, event: &InputEvent, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let value = input.read(cx).value().parse::<f64>().ok();
                let Some(TreeSocketContent::Uncut { affixes }) = &mut this.pending else {
                    return;
                };
                if let Some(eq) = affixes
                    .iter_mut()
                    .find(|eq| data::get_affix(&eq.affix_id).is_some_and(|a| a.group_id == id))
                {
                    let def = data::get_affix(&eq.affix_id).unwrap();
                    let (lo, hi) = (def.value_min.unwrap_or(0.), def.value_max.unwrap_or(0.));
                    if let Some(value) = value.filter(|v| v.is_finite() && *v >= lo && *v <= hi) {
                        eq.roll = if hi == lo {
                            1.
                        } else {
                            (value - lo) / (hi - lo)
                        };
                        eq.custom_value = None;
                        this.invalid_rolls.remove(&id);
                    } else {
                        this.invalid_rolls.insert(
                            id.clone(),
                            trf(
                                "jewelry.roll_out_of_range",
                                &[("lo", (lo).to_string()), ("hi", (hi).to_string())],
                            ),
                        );
                    }
                }
                cx.notify();
            });
            self.affix_inputs.push(AffixInput {
                group,
                input,
                _subscription: subscription,
            });
        }
    }

    fn pick(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab == Tab::Items {
            self.pending = Some(TreeSocketContent::Item { id: id.into() });
            self.error = None;
        } else {
            self.error = jewelry::add_affix(&mut self.pending, id).err();
            if self.error.is_none() {
                self.adding = false;
                self.search
                    .update(cx, |input, cx| input.set_value("", window, cx));
            }
        }
        self.rebuild_affix_inputs(window, cx);
        self.filter(cx);
        cx.notify();
    }

    fn apply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.error.is_some() || !self.invalid_rolls.is_empty() {
            return;
        }
        if self.document != DocumentKey::from_session(self.session.read(cx)) {
            self.error = Some(tr("jewelry.build_changed").into());
            cx.notify();
            return;
        }
        let node_id = self.node_id;
        let content = self.pending.clone();
        let result = self.session.update(cx, |session, cx| {
            let mut snapshot = session.snapshot().clone();
            jewelry::commit(&mut snapshot, node_id, content)?;
            session.edit(|draft| draft.snapshot.tree_socketed = snapshot.tree_socketed);
            cx.notify();
            Ok::<_, String>(())
        });
        match result {
            Ok(()) => window.close_dialog(cx),
            Err(error) => {
                self.error = Some(error);
                cx.notify();
            }
        }
    }

    fn tier_step(&mut self, group: &str, next: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(TreeSocketContent::Uncut { affixes }) = &mut self.pending
            && let Some(eq) = affixes
                .iter_mut()
                .find(|eq| data::get_affix(&eq.affix_id).is_some_and(|a| a.group_id == group))
        {
            let tiers = jewel_tiers(group);
            if let Some(index) = tiers.iter().position(|a| a.id == eq.affix_id) {
                let index = if next {
                    (index + 1).min(tiers.len() - 1)
                } else {
                    index.saturating_sub(1)
                };
                eq.affix_id = tiers[index].id.clone();
                eq.tier = tiers[index].tier;
                eq.custom_value = None;
            }
        }
        self.error = None;
        self.rebuild_affix_inputs(window, cx);
        cx.notify();
    }

    fn status_text(&self) -> String {
        match &self.pending {
            None => tr("gear.empty_socket").into(),
            Some(TreeSocketContent::Uncut { affixes }) => trf(
                "jewelry.affix_count",
                &[("count", affixes.len().to_string())],
            ),
            Some(TreeSocketContent::Item { id }) => {
                self.catalog.iter().find(|c| &c.id == id).map_or_else(
                    || id.clone(),
                    |c| {
                        trf(
                            "jewelry.item_tier",
                            &[
                                ("arg0", (c.name).to_string()),
                                ("arg1", (c.tier).to_string()),
                            ],
                        )
                    },
                )
            }
        }
    }

    fn choice_rows(
        &mut self,
        range: std::ops::Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<Button> {
        let p = cx.global::<theme::TooltipTheme>();
        range
            .filter_map(|index| {
                let row = self.catalog.get(*self.choices.get(index)?)?;
                let id = row.id.clone();
                let is_affix = row.kind == "Affix";
                let selected =
                    matches!(&self.pending, Some(TreeSocketContent::Item { id }) if id == &row.id);
                let (tier_color, tier_border) = match row.tier {
                    tier if tier >= 4 => (p.stat_orange, p.stat_orange),
                    3 => (p.text, p.accent_hot),
                    2 => (p.accent_hot, p.accent_deep),
                    _ => (p.accent_deep, p.accent_deep),
                };
                let button = Button::new(SharedString::from(format!("jewelry-choice-{}", row.id)))
                    .custom(
                        ButtonCustomVariant::new(cx)
                            .color(if selected {
                                p.accent_hot.opacity(0.1)
                            } else {
                                p.background.opacity(0.)
                            })
                            .foreground(p.text)
                            .hover(p.accent_hot.opacity(0.05))
                            .active(p.accent_hot.opacity(0.08)),
                    )
                    .w_full()
                    .h(rems(40. / 13.))
                    .px_4()
                    .gap_3p5()
                    .justify_start()
                    .rounded_none()
                    .border_0()
                    .border_b_1()
                    .border_dashed()
                    .border_color(p.border)
                    .font_family(theme::FONT_FAMILY)
                    .text_size(rems(1.))
                    .selected(selected)
                    .accessibility_label(trf(
                        if is_affix {
                            "jewelry.add_named"
                        } else {
                            "jewelry.select_named"
                        },
                        &[("name", row.name.clone())],
                    ))
                    .on_click(cx.listener(move |this, _, window, cx| this.pick(&id, window, cx)));
                let button = if is_affix {
                    button
                        .child(div().flex_1().min_w_0().truncate().child(row.name.clone()))
                        .child(mono(10., p.faint).flex_none().child(trf(
                            "jewelry.tiers_count",
                            &[("count", row.tier.to_string())],
                        )))
                } else {
                    button
                        .child(
                            div()
                                .w(rems(36. / 13.))
                                .flex_none()
                                .flex()
                                .justify_center()
                                .children(
                                    crate::gear::presentation::socketable_icon(&row.name).map(
                                        |icon| {
                                            img(icon)
                                                .size(rems(24. / 13.))
                                                .object_fit(ObjectFit::ScaleDown)
                                        },
                                    ),
                                ),
                        )
                        .child(
                            mono(10., p.faint).w(rems(56. / 13.)).flex_none().child(
                                match row.kind {
                                    "Affix" => tr("jewelry.kind_affix"),
                                    "Gem" => tr("jewelry.kind_gem"),
                                    "Rune" => tr("jewelry.kind_rune"),
                                    "Jewel" => tr("jewelry.kind_jewel"),
                                    other => other,
                                }
                                .to_uppercase(),
                            ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if selected { p.accent_hot } else { p.text })
                                .child(row.name.clone()),
                        )
                        .child(
                            mono(11., tier_color)
                                .flex_none()
                                .rounded_sm()
                                .border_1()
                                .border_color(tier_border)
                                .bg(p.accent_deep.opacity(0.25))
                                .px_2()
                                .py_0p5()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(trf(
                                    "item.numbered_tier_short",
                                    &[("arg0", (row.tier).to_string())],
                                )),
                        )
                        .child(
                            mono(10., p.muted)
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .child(row.stats.clone()),
                        )
                };
                Some(button)
            })
            .collect()
    }

    fn tab_button(&self, tab: Tab, label: &'static str, cx: &Context<Self>) -> Button {
        controls::tab(label, label, self.tab == tab, cx).on_click(cx.listener(
            move |this, _, window, cx| {
                this.tab = tab;
                this.adding = false;
                this.search
                    .update(cx, |input, cx| input.set_value("", window, cx));
                this.filter(cx);
                cx.notify();
            },
        ))
    }

    fn search_box(&self, cx: &Context<Self>) -> Div {
        div()
            .flex_none()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(cx.global::<theme::TooltipTheme>().border)
            .child(Input::new(&self.search).planner_style(cx))
    }

    fn choice_list(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<theme::TooltipTheme>();
        div()
            .flex_1()
            .min_h_0()
            .py_1()
            .when(self.choices.is_empty(), |v| {
                v.child(
                    div()
                        .p_8()
                        .text_center()
                        .text_color(p.muted)
                        .child(tr("gear.no_matches")),
                )
            })
            .when(!self.choices.is_empty(), |v| {
                v.child(
                    uniform_list(
                        "jewelry-choices",
                        self.choices.len(),
                        cx.processor(Self::choice_rows),
                    )
                    .track_scroll(&self.list)
                    .size_full(),
                )
            })
    }

    fn uncut_tab(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<theme::TooltipTheme>();
        let affixes: &[EquippedAffix] = match &self.pending {
            Some(TreeSocketContent::Uncut { affixes }) => affixes,
            _ => &[],
        };
        let full = affixes.len() >= jewelry::MAX_AFFIXES;
        let toggle = if self.adding {
            modal_button(
                "add-affix-done",
                tr("jewelry.done"),
                ButtonTone::Neutral,
                cx,
            )
            .on_click(cx.listener(|this, _, window, cx| {
                this.adding = false;
                this.search
                    .update(cx, |input, cx| input.set_value("", window, cx));
                this.filter(cx);
                cx.notify();
            }))
        } else {
            modal_button(
                "add-affix",
                tr("jewelry.add_affix"),
                ButtonTone::Primary,
                cx,
            )
            .disabled(full)
            .on_click(cx.listener(|this, _, _, cx| {
                this.adding = true;
                cx.notify();
            }))
        };
        let tab = div().flex_1().min_h_0().flex().flex_col().child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .justify_between()
                .px_4()
                .py_3()
                .border_b_1()
                .border_color(p.border)
                .child(
                    mono(11., p.muted)
                        .flex()
                        .gap_1()
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(p.accent_hot)
                                .child(affixes.len().to_string()),
                        )
                        .child(trf(
                            "jewelry.affix_capacity",
                            &[("arg0", (jewelry::MAX_AFFIXES).to_string())],
                        )),
                )
                .child(toggle),
        );
        if self.adding {
            return tab.child(self.search_box(cx)).child(self.choice_list(cx));
        }
        if affixes.is_empty() {
            return tab.child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .px_6()
                    .py_12()
                    .text_center()
                    .child(
                        div()
                            .mb_4()
                            .size(rems(54. / 13.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .border_1()
                            .border_dashed()
                            .border_color(p.border_strong)
                            .bg(p.accent.opacity(0.06))
                            .text_size(rems(20. / 13.))
                            .text_color(p.faint)
                            .child("◆"),
                    )
                    .child(
                        div()
                            .mb_1p5()
                            .text_size(rems(14. / 13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.muted)
                            .child(tr("jewelry.no_affixes")),
                    )
                    .child(mono(10., p.faint).child(tr("jewelry.roll_hint"))),
            );
        }
        tab.child(
            div()
                .id("jewelry-affixes")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .px_4()
                .py_4()
                .child(
                    div()
                        .mx_auto()
                        .w_full()
                        .max_w(rems(540. / 13.))
                        .flex()
                        .flex_col()
                        .gap_2p5()
                        .children(
                            affixes
                                .iter()
                                .enumerate()
                                .map(|(index, eq)| self.affix_card(index, eq, cx)),
                        ),
                ),
        )
    }

    fn affix_card(&self, index: usize, eq: &EquippedAffix, cx: &Context<Self>) -> Stateful<Div> {
        let p = cx.global::<theme::TooltipTheme>();
        let def = data::get_affix(&eq.affix_id);
        let group = def.map_or_else(|| eq.affix_id.clone(), |a| a.group_id.clone());
        let name = def.map_or_else(
            || {
                trf(
                    "jewelry.unknown_affix",
                    &[("arg0", (eq.affix_id).to_string())],
                )
            },
            |a| a.description.clone(),
        );
        let stat = def.map_or_else(
            || group.clone(),
            |a| stat_name(a.stat_key.as_deref().unwrap_or("")),
        );
        let value = def.map(|a| {
            format_value(
                eq.custom_value
                    .unwrap_or_else(|| rolled_affix_value(a, eq.roll)),
                a.stat_key.as_deref().unwrap_or(""),
                true,
            )
        });
        let tiers = jewel_tiers(&group);
        let (remove, prev, next) = (group.clone(), group.clone(), group.clone());
        div()
            .id(SharedString::from(format!("jewel-affix-{group}")))
            .flex_none()
            .rounded_sm()
            .border_1()
            .border_color(p.border_strong)
            .px_3p5()
            .py_3()
            .bg(linear_gradient(
                180.,
                linear_color_stop(p.panel_secondary, 0.),
                linear_color_stop(p.background.opacity(0.8), 1.),
            ))
            .flex()
            .flex_col()
            .gap_2p5()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        mono(11., p.accent_hot)
                            .size(rems(22. / 13.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .border_1()
                            .border_color(p.accent_deep)
                            .bg(p.accent_deep.opacity(0.3))
                            .child((index + 1).to_string()),
                    )
                    .child(div().flex_1().min_w_0().text_color(p.text).child(name))
                    .children(value.map(|value| mono(11., p.accent_hot).flex_none().child(value)))
                    .child(
                        controls::icon_button("remove-affix", "×", true, cx)
                            .accessibility_label(trf(
                                "jewelry.remove_affix_named",
                                &[("stat", (stat).to_string())],
                            ))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if let Some(TreeSocketContent::Uncut { affixes }) =
                                    &mut this.pending
                                {
                                    affixes.retain(|eq| {
                                        data::get_affix(&eq.affix_id)
                                            .map_or(eq.affix_id.as_str(), |a| a.group_id.as_str())
                                            != remove
                                    });
                                    if affixes.is_empty() {
                                        this.pending = None;
                                    }
                                }
                                this.error = None;
                                this.rebuild_affix_inputs(window, cx);
                                this.filter(cx);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                mono(10., p.faint)
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_3()
                    .pl(rems(34. / 13.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(tr("jewelry.tier"))
                            .child(
                                controls::icon_button("tier-down", "−", false, cx)
                                    .accessibility_label(trf(
                                        "jewelry.lower_tier",
                                        &[("stat", (stat).to_string())],
                                    ))
                                    .disabled(tiers.first().is_none_or(|a| a.id == eq.affix_id))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.tier_step(&prev, false, window, cx)
                                    })),
                            )
                            .child(div().text_color(p.text).child(trf(
                                "item.numbered_tier_short",
                                &[("arg0", (eq.tier).to_string())],
                            )))
                            .child(
                                controls::icon_button("tier-up", "+", false, cx)
                                    .accessibility_label(trf(
                                        "jewelry.higher_tier",
                                        &[("stat", (stat).to_string())],
                                    ))
                                    .disabled(tiers.last().is_none_or(|a| a.id == eq.affix_id))
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.tier_step(&next, true, window, cx)
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(tr("jewelry.roll"))
                            .children(self.affix_inputs.iter().find(|i| i.group == group).map(
                                |i| {
                                    div()
                                        .w(rems(6.))
                                        .child(Input::new(&i.input).planner_style(cx))
                                },
                            ))
                            .children(def.map(|a| {
                                div().child(format!(
                                    "({}–{})",
                                    a.value_min.unwrap_or(0.),
                                    a.value_max.unwrap_or(0.)
                                ))
                            })),
                    ),
            )
    }
}

impl Render for JewelryEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<theme::TooltipTheme>();
        let dirty = !jewelry::same_content(self.original.as_ref(), self.pending.as_ref());
        let has_pending = self.pending.is_some();
        let status_color = if has_pending { p.accent_hot } else { p.faint };
        let eyebrow = modal_eyebrow("jewelry-eyebrow", tr("jewelry.socket")).child(
            div().text_color(p.accent_hot).child(TooltipText::new(
                "jewelry-eyebrow-id",
                format!("#{}", self.node_id),
                0.12,
            )),
        );
        let tabs = div()
            .flex_none()
            .flex()
            .border_b_1()
            .border_color(p.border)
            .bg(p.background)
            .child(self.tab_button(Tab::Items, tr("jewelry.items_tab"), cx))
            .child(self.tab_button(Tab::Uncut, tr("jewelry.craft_tab"), cx));
        let body = if self.tab == Tab::Items {
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .child(self.search_box(cx))
                .child(self.choice_list(cx))
        } else {
            self.uncut_tab(cx)
        };
        let error = self
            .error
            .as_ref()
            .or_else(|| self.invalid_rolls.values().next())
            .cloned();
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(modal_header(
                eyebrow,
                tr("jewelry.insert_socketable"),
                None,
                cx,
            ))
            .child(tabs)
            .child(body)
            .children(error.map(|error| {
                div()
                    .flex_none()
                    .px_4()
                    .py_2()
                    .text_size(rems(12. / 13.))
                    .text_color(p.negative)
                    .child(error)
            }))
            .child(
                modal_footer(cx)
                    .bg(p.shadow.opacity(0.3))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().size_1p5().flex_none().rounded_full().bg(status_color))
                            .child(modal_status(self.status_text(), status_color)),
                    )
                    .when(has_pending, |view| {
                        view.child(
                            modal_button(
                                "clear-jewelry",
                                tr("gear.clear"),
                                ButtonTone::Neutral,
                                cx,
                            )
                            .on_click(cx.listener(
                                |this, _, window, cx| {
                                    this.pending = None;
                                    this.error = None;
                                    this.adding = false;
                                    this.rebuild_affix_inputs(window, cx);
                                    this.filter(cx);
                                    cx.notify();
                                },
                            )),
                        )
                    })
                    .child(
                        modal_button(
                            "apply-jewelry",
                            if !has_pending && self.original.is_some() {
                                tr("gear.remove")
                            } else {
                                tr("jewelry.insert")
                            },
                            ButtonTone::Primary,
                            cx,
                        )
                        .disabled(!dirty || self.error.is_some() || !self.invalid_rolls.is_empty())
                        .on_click(cx.listener(|this, _, window, cx| this.apply(window, cx))),
                    ),
            )
    }
}
