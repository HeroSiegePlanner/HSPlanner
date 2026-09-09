//! Item picker (Change item) after the Tauri ItemListRail: search, sort, rarity groups, tooltips.
use super::*;
use crate::gear_sections::{chip, ghost_button, icon_button, units};
use crate::item_tooltip;
use crate::skill_details::stat_name;
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::{Icon, IconName, IndexPath, Sizable};
use hsplanner_engine::calc::commands::{RankSlotItemsInput, rank_slot_items};
use hsplanner_engine::calc::types::{ItemBase, RangedValue};
use hsplanner_ui::controls::{ButtonTone, planner_button};
use hsplanner_ui::tooltip::CursorTooltipExt;
use hsplanner_ui::tooltip_text::TooltipText;

pub(super) type SortSelect = SelectState<Vec<SharedString>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PickerContext {
    document: DocumentKey,
    slot: String,
    mercenary: bool,
    picker: Picker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RankingRequest {
    context: PickerContext,
    revision: u64,
}

impl RankingRequest {
    fn is_current(&self, context: Option<&PickerContext>, revision: u64) -> bool {
        context == Some(&self.context) && revision == self.revision
    }
}

fn item_bases(
    snapshot: &hsplanner_build::BuildSnapshot,
    slot: &str,
    mercenary: bool,
) -> Vec<&'static ItemBase> {
    data::data()
        .items
        .values()
        .filter(|base| gear::accepts(snapshot, slot, base, mercenary))
        .collect()
}

#[derive(Clone)]
pub(super) struct ItemRow {
    id: String,
    base_id: String,
    name: String,
    rarity: String,
    meta: String,
    search: String,
    sort_values: HashMap<String, f64>,
    item: Option<EquippedItem>,
}

const RARITY_ORDER: [&str; 10] = [
    "unholy",
    "angelic",
    "heroic",
    "satanic_set",
    "satanic",
    "common",
    "relic",
    "mythic",
    "rare",
    "uncommon",
];

fn rarity_rank(rarity: &str) -> usize {
    RARITY_ORDER.iter().position(|r| *r == rarity).unwrap_or(99)
}

fn mid(value: RangedValue) -> f64 {
    let (a, b) = value.as_ranged();
    (a + b) / 2.
}

fn sort_values(base: &ItemBase) -> HashMap<String, f64> {
    let mut out: HashMap<String, f64> = base
        .implicit
        .iter()
        .flatten()
        .map(|(k, v)| (k.clone(), mid(*v)))
        .filter(|(_, v)| *v != 0.)
        .collect();
    if let Some(defense) = base.defense_max {
        out.insert("defense".into(), defense);
    }
    if let (Some(min), Some(max)) = (base.damage_min, base.damage_max) {
        out.insert("weapon_damage".into(), (min + max) / 2.);
    }
    if let Some(block) = base.block_chance {
        out.insert("block_chance".into(), block);
    }
    if let Some(sockets) = base.max_sockets.or(base.sockets).filter(|s| *s > 0) {
        out.insert("sockets".into(), f64::from(sockets));
    }
    out
}

fn base_meta(base: &ItemBase) -> String {
    let mut parts = vec![base.base_type.clone()];
    if let Some(grade) = &base.grade {
        parts.push(format!("Grade {grade}"));
    }
    if base.base_type == "Charm" {
        parts.push(format!(
            "{}×{}",
            base.width.unwrap_or(1),
            base.height.unwrap_or(1)
        ));
    }
    if let (Some(min), Some(max)) = (base.defense_min, base.defense_max) {
        parts.push(format!("Def {min}–{max}"));
    }
    if let (Some(min), Some(max)) = (base.damage_min, base.damage_max) {
        parts.push(format!("Dmg {min}–{max}"));
    }
    if let Some(block) = base.block_chance {
        parts.push(format!("Block {block}%"));
    }
    if let Some(sockets) = base.sockets {
        let max = base.max_sockets.unwrap_or(sockets);
        parts.push(if max > sockets {
            format!("{sockets}/{max} sockets")
        } else {
            format!("{sockets} sockets")
        });
    }
    parts.join(" · ")
}

fn base_search(base: &ItemBase) -> String {
    let mut parts = vec![base.name.clone(), base.base_type.clone()];
    parts.extend(base.grade.iter().map(|g| format!("Grade {g}")));
    for (key, value) in base.implicit.iter().flatten() {
        parts.push(stat_name(key));
        parts.push(item_tooltip::format_ranged(value.as_ranged(), key));
    }
    parts.extend(base.unique_effects.iter().flatten().cloned());
    parts.extend(
        base.procs
            .iter()
            .flatten()
            .flat_map(|p| [p.trigger.clone(), p.target.clone().unwrap_or_default()]),
    );
    for (skill, value) in base.skill_bonuses.iter().flatten() {
        parts.push(skill.clone());
        parts.push(item_tooltip::format_ranged(value.as_ranged(), ""));
    }
    parts.extend(base.description.iter().cloned());
    parts.extend(base.flavor.iter().cloned());
    parts.extend(base.set_id.iter().cloned());
    parts.push(editor::rarity_label(&base.rarity));
    parts.join(" ").to_lowercase()
}

fn sort_label(key: &str) -> String {
    match key {
        "defense" => "Defense".into(),
        "weapon_damage" => "Weapon Damage".into(),
        "block_chance" => "Block Chance".into(),
        "sockets" => "Sockets".into(),
        _ => stat_name(key),
    }
}

fn fmt_sort_value(value: f64, key: &str) -> String {
    let percent = data::game_config()
        .stats
        .iter()
        .any(|s| s.key == key && s.format.as_deref() == Some("percent"));
    let suffix = if percent { "%" } else { "" };
    let (scaled, unit) = match value.abs() {
        v if v >= 1e9 => (value / 1e9, "B"),
        v if v >= 1e6 => (value / 1e6, "M"),
        v if v >= 1e3 => (value / 1e3, "k"),
        _ => return format!("{}{suffix}", (value * 10.).round() / 10.),
    };
    let digits = if scaled.abs() >= 10. { 0 } else { 1 };
    format!("{scaled:.digits$}{unit}{suffix}")
}

impl GearView {
    pub(super) fn invalidate_item_picker(&mut self) {
        self.ranking_revision += 1;
        self.picker_context = None;
        self.item_rows.clear();
        self.sort_select = None;
        self.sort_options.clear();
        self.sort_key = "default".into();
        self.dps_values = None;
        self.dps_pending = false;
    }

    /// Builds the rows and sort options for the Items / Stash tabs.
    pub(super) fn prepare_item_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.invalidate_item_picker();
        let session = self.session.read(cx);
        let snapshot = session.snapshot();
        self.picker_context = Some(PickerContext {
            document: DocumentKey::from_session(session),
            slot: self.slot.clone(),
            mercenary: self.mercenary,
            picker: self.picker,
        });
        let mut rows: Vec<ItemRow> = match self.picker {
            Picker::Items => item_bases(snapshot, &self.slot, self.mercenary)
                .into_iter()
                .map(|base| ItemRow {
                    id: base.id.clone(),
                    base_id: base.id.clone(),
                    name: base.name.clone(),
                    rarity: base.rarity.clone(),
                    meta: base_meta(base),
                    search: base_search(base),
                    sort_values: sort_values(base),
                    item: None,
                })
                .collect(),
            Picker::Stash => session
                .draft()
                .stash
                .iter()
                .filter_map(|entry| {
                    let base = data::get_item(&entry.item.base_id)?;
                    gear::accepts(snapshot, &self.slot, base, self.mercenary).then(|| ItemRow {
                        id: entry.id.clone(),
                        base_id: base.id.clone(),
                        name: base.name.clone(),
                        rarity: base.rarity.clone(),
                        meta: format!(
                            "{} · {} stars · {} sockets",
                            base.base_type,
                            entry.item.stars.unwrap_or(0),
                            entry.item.socket_count
                        ),
                        search: base_search(base),
                        sort_values: sort_values(base),
                        item: Some(entry.item.clone()),
                    })
                })
                .collect(),
            _ => return,
        };
        if self.picker == Picker::Items {
            rows.sort_by(|a, b| {
                rarity_rank(&a.rarity)
                    .cmp(&rarity_rank(&b.rarity))
                    .then_with(|| a.name.cmp(&b.name))
            });
        }
        self.item_rows = rows;
        if self.sort_select.is_none() {
            self.build_sort_select(window, cx);
        }
    }

    fn build_sort_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for row in &self.item_rows {
            for key in row.sort_values.keys() {
                *counts.entry(key.as_str()).or_default() += 1;
            }
        }
        let mut stats: Vec<(String, String, usize)> = counts
            .into_iter()
            .map(|(key, count)| (key.to_owned(), sort_label(key), count))
            .collect();
        stats.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(&b.1)));
        let mut options = vec![("default".to_owned(), "Default".to_owned())];
        if !self.mercenary {
            options.push(("dps".into(), "DPS".into()));
        }
        options.extend(stats.into_iter().map(|(key, label, _)| (key, label)));
        let labels: Vec<SharedString> = options
            .iter()
            .map(|(_, label)| SharedString::from(label.clone()))
            .collect();
        let select = cx.new(|cx| SelectState::new(labels, Some(IndexPath::new(0)), window, cx));
        self._subscriptions.push(cx.subscribe(
            &select,
            |this, _, event: &SelectEvent<Vec<SharedString>>, cx| {
                let SelectEvent::Confirm(Some(label)) = event else {
                    return;
                };
                let Some((key, _)) = this.sort_options.iter().find(|(_, l)| *l == label.as_ref())
                else {
                    return;
                };
                this.sort_key = key.clone();
                if this.sort_key == "dps" && this.dps_values.is_none() {
                    this.rank_dps(cx);
                }
                cx.notify();
            },
        ));
        self.sort_options = options;
        self.sort_select = Some(select);
    }

    fn rank_dps(&mut self, cx: &mut Context<Self>) {
        let Some(context) = self.picker_context.clone() else {
            return;
        };
        self.ranking_revision += 1;
        let request = RankingRequest {
            context,
            revision: self.ranking_revision,
        };
        self.dps_pending = true;
        let planner = self.session.read(cx).snapshot().planner_input();
        let input = RankSlotItemsInput {
            perf: planner.build,
            slot: self.slot.clone(),
            base_ids: self
                .item_rows
                .iter()
                .map(|row| row.base_id.clone())
                .collect(),
            active_skill_ids: planner.active_skill_ids,
        };
        let task = cx.background_spawn(async move { rank_slot_items(input) });
        cx.spawn(async move |this, cx| {
            let values = task.await;
            let _ = this.update(cx, |this, cx| {
                if !request.is_current(this.picker_context.as_ref(), this.ranking_revision)
                    || request.context.document != DocumentKey::from_session(this.session.read(cx))
                {
                    return;
                }
                this.dps_pending = false;
                this.dps_values = Some(values);
                cx.notify();
            });
        })
        .detach();
    }

    fn sort_badge(&self, row: &ItemRow) -> Option<String> {
        match self.sort_key.as_str() {
            "default" => None,
            "dps" => self
                .dps_values
                .as_ref()?
                .get(&row.base_id)
                .map(|v| fmt_sort_value(*v, "dps")),
            key => row.sort_values.get(key).map(|v| fmt_sort_value(*v, key)),
        }
    }

    fn visible_item_rows(&self, cx: &App) -> Vec<ItemRow> {
        let query = self.search.read(cx).value().trim().to_lowercase();
        let mut rows: Vec<ItemRow> = self
            .item_rows
            .iter()
            .filter(|row| query.is_empty() || row.search.contains(&query))
            .cloned()
            .collect();
        let value = |row: &ItemRow| -> f64 {
            match self.sort_key.as_str() {
                "dps" => self
                    .dps_values
                    .as_ref()
                    .and_then(|v| v.get(&row.base_id))
                    .copied()
                    .unwrap_or(-1.),
                key => row
                    .sort_values
                    .get(key)
                    .copied()
                    .unwrap_or(f64::NEG_INFINITY),
            }
        };
        if self.sort_key != "default" {
            rows.sort_by(|a, b| value(b).total_cmp(&value(a)));
        }
        rows
    }

    pub(super) fn item_picker(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let (faint, muted, accent, border, panel_secondary) =
            (p.faint, p.muted, p.accent, p.border, p.panel_secondary);
        let rows = self.visible_item_rows(cx);
        let grouped = self.picker == Picker::Items && self.sort_key == "default";
        let selected = self.candidate.as_ref().map(|item| item.base_id.clone());
        let equipped_ids = item_tooltip::equipped_ids(&self.session.read(cx).snapshot().inventory);
        let mut list = div()
            .id("gear-choices")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col();
        if rows.is_empty() {
            list = list.child(
                div()
                    .p_10()
                    .text_center()
                    .text_size(units(13.))
                    .text_color(muted)
                    .child("No items match"),
            );
        }
        let mut last_group: Option<String> = None;
        for row in &rows {
            if grouped && last_group.as_deref() != Some(row.rarity.as_str()) {
                last_group = Some(row.rarity.clone());
                list = list.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_4()
                        .py_1p5()
                        .border_b_1()
                        .border_color(border)
                        .bg(panel_secondary)
                        .text_size(units(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(muted)
                        .child(div().size_1().rounded_full().bg(accent))
                        .child(editor::rarity_label(&row.rarity)),
                );
            }
            let is_selected = selected.as_deref() == Some(row.base_id.as_str());
            list = list.child(self.item_row(row, is_selected, &equipped_ids, cx));
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(self.picker_tab("items", "Items", Picker::Items, cx))
                            .child(self.picker_tab("stash", "Stash", Picker::Stash, cx))
                            .child(div().flex_1())
                            .when(self.candidate.is_some(), |v| {
                                v.child(ghost_button("back-to-configure", "← Back", cx).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.choosing = false;
                                        cx.notify();
                                    }),
                                ))
                            }),
                    )
                    .child(
                        Input::new(&self.search)
                            .planner_style(cx)
                            .prefix(Icon::new(IconName::Search).size_3p5().text_color(faint)),
                    )
                    .when(self.picker == Picker::Items, |v| {
                        v.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .font_family(theme::MONO_FONT_FAMILY)
                                        .text_size(units(9.))
                                        .text_color(faint)
                                        .child(TooltipText::new("sort-label", "SORT", 0.18)),
                                )
                                .children(self.sort_select.as_ref().map(|state| {
                                    div()
                                        .w(units(200.))
                                        .child(Select::new(state).xsmall().menu_width(units(220.)))
                                }))
                                .when(self.sort_key == "dps" && self.dps_pending, |v| {
                                    v.child(
                                        div()
                                            .font_family(theme::MONO_FONT_FAMILY)
                                            .text_size(units(9.))
                                            .text_color(faint)
                                            .child(TooltipText::new(
                                                "sort-computing",
                                                "COMPUTING…",
                                                0.14,
                                            )),
                                    )
                                }),
                        )
                    }),
            )
            .child(list)
    }

    fn picker_tab(
        &self,
        id: &'static str,
        label: &str,
        picker: Picker,
        cx: &Context<Self>,
    ) -> Button {
        let active = self.picker == picker;
        let tone = if active {
            ButtonTone::Primary
        } else {
            ButtonTone::Neutral
        };
        planner_button(id, tone, cx)
            .small()
            .label(label.to_owned())
            .on_click(
                cx.listener(move |this, _, window, cx| this.choose_picker(picker, window, cx)),
            )
    }

    fn item_row(
        &self,
        row: &ItemRow,
        selected: bool,
        equipped_ids: &[String],
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let p = cx.global::<TooltipTheme>();
        let color = theme::rarity_color(&row.rarity, cx);
        let icon = presentation::item_icon(&row.base_id);
        let id = row.id.clone();
        let badge = self.sort_badge(row);
        let is_stash = row.item.is_some();
        let remove = row.id.clone();
        let inner = div()
            .id(SharedString::from(format!("choice-{}", row.id)))
            .relative()
            .w_full()
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_2p5()
            .border_b_1()
            .border_color(p.border)
            .cursor_pointer()
            .when(selected, |v| v.bg(p.accent_hot.opacity(0.05)))
            .hover(|v| v.bg(p.accent_hot.opacity(0.05)))
            .on_click(cx.listener(move |this, _, _, cx| this.pick(&id, cx)))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w_0p5()
                    .bg(p.accent)
                    .when(!selected, |v| v.invisible()),
            )
            .child(
                div()
                    .size(units(36.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .map(|v| match icon {
                        Some(icon) => v.child(img(icon).size_full().object_fit(ObjectFit::Contain)),
                        None => v.child(
                            div()
                                .size(units(20.))
                                .rounded_sm()
                                .border_1()
                                .border_color(color.opacity(0.6))
                                .bg(color.opacity(0.35)),
                        ),
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(units(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(color)
                    .child(row.name.clone()),
            )
            .children(badge.map(|badge| {
                chip(badge, p.accent_hot, p.accent_deep.opacity(0.4))
                    .flex_none()
                    .px_1p5()
                    .py_0p5()
                    .text_size(units(10.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .bg(p.accent_deep.opacity(0.1))
            }))
            .child(
                div()
                    .flex_none()
                    .max_w(units(180.))
                    .truncate()
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(units(9.))
                    .text_color(p.muted.opacity(0.8))
                    .child(row.meta.clone()),
            )
            .when(is_stash, |v| {
                v.child(
                    icon_button(
                        SharedString::from(format!("stash-remove-{}", row.id)),
                        "×",
                        cx,
                    )
                    .cursor_tooltip("Remove from stash")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.session.update(cx, |session, cx| {
                            session.edit(|draft| draft.stash.retain(|entry| entry.id != remove));
                            cx.notify();
                        });
                        this.prepare_item_picker(window, cx);
                        cx.notify();
                    })),
                )
            });
        let tip_id = SharedString::from(format!("choice-tip-{}", row.id));
        match (&row.item, data::get_item(&row.base_id)) {
            (Some(item), _) => {
                item_tooltip::with_item_tooltip(tip_id, item, equipped_ids.to_vec(), inner)
            }
            (None, Some(base)) => {
                item_tooltip::with_base_tooltip(tip_id, base, equipped_ids.to_vec(), inner)
            }
            (None, None) => div().id(tip_id).child(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hsplanner_build::{BuildSnapshot, session::WorkspaceState};

    #[::core::prelude::v1::test]
    fn initial_and_changed_slots_use_only_their_eligible_items() {
        let mut snapshot = BuildSnapshot::default();
        let weapons = item_bases(&snapshot, "weapon", false);
        let armors = item_bases(&snapshot, "armor", false);
        assert!(!weapons.is_empty() && !armors.is_empty());
        assert!(weapons.iter().all(|base| base.slot == "weapon"));
        assert!(armors.iter().all(|base| base.slot == "armor"));
        let two_handed = weapons
            .iter()
            .find(|base| base.two_handed == Some(true))
            .unwrap();
        snapshot
            .merc_inventory
            .insert("weapon".into(), gear::make_item(&two_handed.id).unwrap());
        assert!(item_bases(&snapshot, "offhand", true).is_empty());
        let belts = item_bases(&snapshot, "belt", true);
        assert!(!belts.is_empty() && belts.iter().all(|base| base.slot == "belt"));
    }

    #[::core::prelude::v1::test]
    fn ranking_rejects_changed_slot_profile_document_and_superseded_requests() {
        let mut state = WorkspaceState::default();
        state.draft.build_id = Some("review-build".into());
        state.draft.profile_id = Some("profile-one".into());
        let context = PickerContext {
            document: DocumentKey::from_session(&Session::new(state.clone())),
            slot: "weapon".into(),
            mercenary: false,
            picker: Picker::Items,
        };
        let request = RankingRequest {
            context: context.clone(),
            revision: 4,
        };
        assert!(request.is_current(Some(&context), 4));
        assert!(!request.is_current(Some(&context), 5));
        assert!(!request.is_current(None, 4));
        let mut changed = context.clone();
        changed.slot = "armor".into();
        assert!(!request.is_current(Some(&changed), 4));
        changed = context.clone();
        changed.picker = Picker::Stash;
        assert!(!request.is_current(Some(&changed), 4));
        state.draft.profile_id = Some("profile-two".into());
        changed = context.clone();
        changed.document = DocumentKey::from_session(&Session::new(state.clone()));
        assert!(!request.is_current(Some(&changed), 4));
        state.draft.profile_id = Some("profile-one".into());
        state.revision += 1;
        changed.document = DocumentKey::from_session(&Session::new(state));
        assert!(!request.is_current(Some(&changed), 4));
    }
}
