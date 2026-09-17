use crate::build_session::DocumentKey;
use gpui_kit::base::Disableable;
use gpui_kit::component::{
    Sizable, WindowExt,
    button::Button,
    input::{Input, InputEvent, InputState},
    slider::{SliderEvent, SliderState},
};
use gpui_kit::{prelude::*, *};
use hsplanner_build::{gear, session::Session};
use hsplanner_engine::calc::{
    data,
    performance_diff::{PerformanceDiff, compare_planner},
    planner::evaluate,
    types::{EquippedItem, SocketType},
};
use hsplanner_ui::controls::PlannerControl;
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::theme::{self, TooltipTheme};

#[path = "gear_text_edit.rs"]
mod text_edit;

#[path = "gear_editor.rs"]
pub(crate) mod editor;

#[path = "gear_import.rs"]
mod import;
#[path = "gear_picker.rs"]
pub(crate) mod picker;
#[path = "gear_presentation.rs"]
pub(crate) mod presentation;
use std::{collections::HashMap, sync::Arc};

include!(concat!(env!("OUT_DIR"), "/item-icons.rs"));
include!(concat!(env!("OUT_DIR"), "/socket-icons.rs"));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Picker {
    Items,
    Stash,
    Socket(usize),
    Runeword,
    Affix,
    Forge,
    Augment,
    RandomSkill,
    RandomElement,
    Subskill,
    Class,
}
#[derive(Clone)]
struct Row {
    id: String,
    label: String,
    detail: String,
    icon: Option<ImageSource>,
    tone: Option<Hsla>,
    kind: &'static str,
    tier: Option<u32>,
    group: &'static str,
}

impl Row {
    fn new(id: impl Into<String>, label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            detail: detail.into(),
            icon: None,
            tone: None,
            kind: "",
            tier: None,
            group: "",
        }
    }

    fn modifier(mut self, kind: &'static str, tier: Option<u32>, group: &'static str) -> Self {
        self.kind = kind;
        self.tier = tier;
        self.group = group;
        self
    }

    fn icon(mut self, icon: Option<impl Into<ImageSource>>) -> Self {
        self.icon = icon.map(Into::into);
        self
    }

    fn tone(mut self, tone: Hsla) -> Self {
        self.tone = Some(tone);
        self
    }
}

fn gem_detail(description: Option<&str>) -> String {
    description
        .unwrap_or_default()
        .split('·')
        .map(str::trim)
        .filter(|part| !part.starts_with("Tier "))
        .collect::<Vec<_>>()
        .join(" · ")
}

pub(crate) fn element_label(element: &str) -> String {
    match element {
        "fire" => tr("gear.element_fire").to_owned(),
        "cold" => tr("gear.element_cold").to_owned(),
        "lightning" => tr("gear.element_lightning").to_owned(),
        "poison" => tr("gear.element_poison").to_owned(),
        "arcane" => tr("gear.element_arcane").to_owned(),
        other => other.to_owned(),
    }
}

fn tier_detail(tier: u32, description: Option<&str>) -> String {
    match description.filter(|d| !d.is_empty()) {
        Some(description) => trf(
            "gear.tier_description",
            &[
                ("tier", (tier).to_string()),
                ("description", (description).to_string()),
            ],
        ),
        None => trf("gear.tier", &[("tier", (tier).to_string())]),
    }
}

/// Short stat summary for list rows: "+12 Strength · +5% Attack Speed".
fn stat_summary(stats: &hsplanner_engine::calc::types::StatMap) -> String {
    let mut entries: Vec<_> = stats.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
        .iter()
        .take(3)
        .map(|(key, value)| format!("{value:+} {}", crate::skill_details::stat_name(key)))
        .collect::<Vec<_>>()
        .join(" · ")
}

#[derive(Clone)]
struct RollTarget {
    affix: Option<usize>,
    forged: bool,
    stat: String,
    skill: bool,
    relic_tier: bool,
}

struct RollSlider {
    state: Entity<SliderState>,
    focus: FocusHandle,
    observed_value: f32,
    target: RollTarget,
    _subscriptions: Vec<Subscription>,
}

#[derive(Default)]
struct CandidateBaseline {
    identity: (Option<String>, Option<String>),
    equipped: Option<EquippedItem>,
}

impl CandidateBaseline {
    /// Compare with the previously observed item, before an external edit changes it.
    /// Calculation revisions invalidate comparisons, but do not own a dirty item draft.
    fn synchronize(
        &mut self,
        session: &Session,
        slot: &str,
        mercenary: bool,
        editing: bool,
        candidate: &mut Option<EquippedItem>,
    ) -> bool {
        let draft = session.draft();
        let identity = (draft.build_id.clone(), draft.profile_id.clone());
        let retain = editing
            && self.identity == identity
            && !editor::same_item(candidate.as_ref(), self.equipped.as_ref());
        let inventory = if mercenary {
            &draft.snapshot.merc_inventory
        } else {
            &draft.snapshot.inventory
        };
        self.identity = identity;
        self.equipped = inventory.get(slot).cloned();
        if retain {
            false
        } else {
            candidate.clone_from(&self.equipped);
            true
        }
    }
}

pub struct GearView {
    session: Entity<Session>,
    document: DocumentKey,
    mercenary: bool,
    slot: String,
    candidate: Option<EquippedItem>,
    candidate_baseline: CandidateBaseline,
    icon: Option<Arc<RenderImage>>,
    search: Entity<InputState>,
    stash_search: Entity<InputState>,
    stash_rows: Vec<crate::gear_stash::StashRow>,
    stash_group: Option<String>,
    stash_list: ListState,
    stash_rem: Pixels,
    scroll: ScrollHandle,
    comparison_scroll: ScrollHandle,
    comparison_has_vertical_scroll: bool,
    editing: bool,
    confirming_close: bool,
    confirmation_focus: FocusHandle,
    confirmation_return_focus: Option<FocusHandle>,
    choosing: bool,
    charm_layout: gear::CharmLayout,
    picker: Picker,
    rows: Vec<Row>,
    item_rows: Vec<picker::ItemRow>,
    visible_items: Vec<usize>,
    picker_list: ListState,
    picker_rem: Pixels,
    sort_key: String,
    sort_options: Vec<(String, String)>,
    sort_select: Option<Entity<picker::SortSelect>>,
    sort_subscription: Option<Subscription>,
    dps_values: Option<HashMap<String, f64>>,
    dps_pending: bool,
    picker_context: Option<picker::PickerContext>,
    ranking_revision: u64,
    error: Option<String>,
    differences: Option<Vec<PerformanceDiff>>,
    verdict: Option<editor::Verdict>,
    sections_mode: Option<bool>,
    show_all_affixes: bool,
    section_overrides: HashMap<String, bool>,
    sliders: HashMap<String, RollSlider>,
    comparison_revision: u64,
    comparing: bool,
    active: bool,
    _subscriptions: Vec<Subscription>,
}
impl GearView {
    /// Relics are tier-only and never stashed, so the editor and picker hide stash controls.
    fn is_relic_slot(&self) -> bool {
        gear::slot_group(&self.slot) == "relic"
    }

    pub fn new(
        session: Entity<Session>,
        mercenary: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(tr("gear.search")));
        let stash_search =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr("gear.search_stash")));
        let document = DocumentKey::from_session(session.read(cx));
        let subscriptions = vec![
            cx.observe_global_in::<hsplanner_ui::i18n::Locale>(window, |this, window, cx| {
                this.search.update(cx, |input, cx| {
                    input.set_placeholder(tr("gear.search"), window, cx)
                });
                this.stash_search.update(cx, |input, cx| {
                    input.set_placeholder(tr("gear.search_stash"), window, cx)
                });
                this.refresh_item_language(window, cx);
                this.refresh_rows(cx);
                this.refresh_stash_rows(cx);
                cx.notify();
            }),
            cx.subscribe(&stash_search, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.refresh_stash_rows(cx);
                    cx.notify();
                }
            }),
            cx.observe(&session, |this, _, cx| {
                let document = DocumentKey::from_session(this.session.read(cx));
                if document != this.document {
                    this.document = document;
                    this.invalidate_item_picker();
                    let reloaded = this.candidate_baseline.synchronize(
                        this.session.read(cx),
                        &this.slot,
                        this.mercenary,
                        this.editing,
                        &mut this.candidate,
                    );
                    if reloaded {
                        this.sliders.clear();
                        this.load_icon();
                    }
                    this.refresh_rows(cx);
                    this.changed(cx);
                } else {
                    // Stash edits do not change the calculation revision.
                    if this.picker == Picker::Stash {
                        this.invalidate_item_picker();
                    }
                    this.refresh_rows(cx);
                }
                this.refresh_charm_layout(cx);
                this.refresh_stash_rows(cx);
                cx.notify();
            }),
            cx.subscribe(&search, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.refresh_rows(cx);
                    cx.notify();
                }
            }),
        ];
        let mut view = Self {
            session,
            document,
            mercenary,
            slot: "weapon".into(),
            candidate: None,
            candidate_baseline: CandidateBaseline::default(),
            icon: None,
            search,
            stash_search,
            stash_rows: vec![],
            stash_group: None,
            stash_list: ListState::new(0, ListAlignment::Top, px(200.)),
            stash_rem: window.rem_size(),
            scroll: ScrollHandle::new(),
            comparison_scroll: ScrollHandle::new(),
            comparison_has_vertical_scroll: false,
            editing: false,
            confirming_close: false,
            confirmation_focus: cx.focus_handle(),
            confirmation_return_focus: None,
            choosing: true,
            charm_layout: gear::charm_layout(&[], false),
            picker: Picker::Items,
            rows: vec![],
            item_rows: vec![],
            visible_items: vec![],
            picker_list: ListState::new(0, ListAlignment::Top, px(200.)),
            picker_rem: window.rem_size(),
            sort_key: "default".into(),
            sort_options: vec![],
            sort_select: None,
            sort_subscription: None,
            dps_values: None,
            dps_pending: false,
            picker_context: None,
            ranking_revision: 0,
            error: None,
            differences: None,
            verdict: None,
            comparison_revision: 0,
            comparing: false,
            sections_mode: None,
            show_all_affixes: false,
            section_overrides: HashMap::new(),
            sliders: HashMap::new(),
            active: false,
            _subscriptions: subscriptions,
        };
        view.revert(cx);
        view.refresh_charm_layout(cx);
        view.refresh_stash_rows(cx);
        view
    }
    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        if self.active != active {
            self.active = active;
            if active && self.editing {
                self.changed(cx);
            }
        }
    }
    fn revert(&mut self, cx: &mut Context<Self>) {
        self.sliders.clear();
        self.candidate_baseline.synchronize(
            self.session.read(cx),
            &self.slot,
            self.mercenary,
            false,
            &mut self.candidate,
        );
        self.load_icon();
        self.refresh_rows(cx);
        self.changed(cx);
    }
    fn refresh_charm_layout(&mut self, cx: &Context<Self>) {
        if self.mercenary {
            return;
        }
        let session = self.session.read(cx);
        let charms = session
            .snapshot()
            .inventory
            .iter()
            .filter(|(key, _)| key.starts_with("charm_"))
            .map(|(key, item)| {
                let base = data::get_item(&item.base_id);
                (
                    key.clone(),
                    base.and_then(|b| b.width).unwrap_or(1),
                    base.and_then(|b| b.height).unwrap_or(1),
                )
            })
            .collect::<Vec<_>>();
        self.charm_layout = gear::charm_layout(&charms, session.state().settings.extra_charm_slot);
    }
    fn load_icon(&mut self) {
        self.icon = self
            .candidate
            .as_ref()
            .and_then(|item| presentation::item_icon(&item.base_id));
    }
    fn choose_picker(&mut self, picker: Picker, window: &mut Window, cx: &mut Context<Self>) {
        self.choosing = true;
        self.picker = picker;
        self.picker_list.reset(0);
        self.rows.clear();
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.refresh_rows(cx);
        if matches!(picker, Picker::Items | Picker::Stash) {
            self.prepare_item_picker(window, cx);
        }
        cx.notify();
    }
    fn refresh_rows(&mut self, cx: &Context<Self>) {
        if matches!(self.picker, Picker::Items | Picker::Stash) {
            self.rows.clear();
            self.refresh_item_results(cx);
            return;
        }
        let session = self.session.read(cx);
        let snapshot = session.snapshot();
        let query = self.search.read(cx).value().to_lowercase();
        let base = self
            .candidate
            .as_ref()
            .and_then(|item| data::get_item(&item.base_id));
        let mut rows: Vec<Row> = match self.picker {
            Picker::Items | Picker::Stash => unreachable!(),
            Picker::Socket(_) => data::data()
                .gems
                .values()
                .map(|g| {
                    Row::new(&g.id, &g.name, gem_detail(g.description.as_deref()))
                        .icon(presentation::socketable_icon(&g.name))
                        .tone(theme::rarity_color("uncommon", cx))
                })
                .chain(data::data().runes.values().map(|r| {
                    Row::new(
                        &r.id,
                        &r.name,
                        tier_detail(r.tier, r.description.as_deref()),
                    )
                    .icon(presentation::socketable_icon(&r.name))
                    .tone(theme::rarity_color("rare", cx))
                }))
                .collect(),
            Picker::Runeword => data::data()
                .runewords
                .iter()
                .filter(|rw| {
                    base.is_some_and(|base| {
                        base.rarity == "common" && rw.allowed_base_types.contains(&base.base_type)
                    }) && self
                        .candidate
                        .as_ref()
                        .is_some_and(|item| gear::max_sockets(item) >= rw.runes.len() as u32)
                })
                .map(|rw| {
                    Row::new(&rw.id, &rw.name, rw.runes.join(" → "))
                        .tone(theme::rarity_color("rare", cx))
                })
                .collect(),
            Picker::Affix => data::data()
                .affixes
                .values()
                .filter(|a| {
                    base.is_some_and(|base| gear::affix_allowed(base, a, self.show_all_affixes))
                })
                .map(|a| {
                    use hsplanner_engine::calc::types::AffixKind;
                    let (kind, group) = match a.kind {
                        Some(AffixKind::Prefix) => ("PREFIX", "Prefixes"),
                        Some(AffixKind::Suffix) => ("SUFFIX", "Suffixes"),
                        None => ("AFFIX", ""),
                    };
                    Row::new(&a.id, &a.name, &a.description)
                        .modifier(kind, Some(a.tier), group)
                        .tone(if a.group_id == "random_unholy" {
                            theme::rarity_color("unholy", cx)
                        } else {
                            cx.global::<TooltipTheme>().text
                        })
                })
                .collect(),
            Picker::Forge => data::data()
                .crystals
                .values()
                .map(|a| {
                    Row::new(&a.id, &a.name, &a.description)
                        .modifier("CRYSTAL", Some(a.tier), "")
                        .tone(theme::rarity_color("satanic", cx))
                })
                .collect(),
            Picker::Augment => data::data()
                .augments
                .values()
                .map(|a| {
                    let detail = a
                        .levels
                        .last()
                        .map(|l| stat_summary(&l.stats))
                        .unwrap_or_default();
                    Row::new(&a.id, &a.name, detail)
                        .modifier("AUGMENT", None, "")
                        .icon(presentation::augment_icon(&a.id))
                        .tone(theme::rarity_color("angelic", cx))
                })
                .collect(),
            Picker::RandomSkill | Picker::Subskill => {
                let mut rows = data::data()
                    .skills_by_class
                    .values()
                    .flatten()
                    .filter(|skill| {
                        selector_accepts_skill(
                            self.picker,
                            base,
                            snapshot.class_id.as_deref(),
                            skill,
                        )
                    })
                    .map(|s| {
                        Row::new(&s.id, &s.name, &s.class_id)
                            .icon(crate::skills::skill_icon(&s.class_id, &s.id))
                    })
                    .collect::<Vec<_>>();
                rows.push(Row::new("", tr("gear.no_skill"), ""));
                rows
            }
            Picker::RandomElement => std::iter::once(Row::new("", tr("gear.no_element"), ""))
                .chain(
                    ["fire", "cold", "lightning", "poison", "arcane"]
                        .into_iter()
                        .map(|element| {
                            Row::new(
                                element,
                                trf(
                                    "gear.element_skills",
                                    &[("element", element_label(element))],
                                ),
                                "",
                            )
                        }),
                )
                .collect(),
            Picker::Class => data::data()
                .classes
                .values()
                .map(|c| Row::new(&c.id, &c.name, ""))
                .chain(std::iter::once(Row::new("", tr("gear.no_class"), "")))
                .collect(),
        };
        if self.picker == Picker::Affix {
            let mut grouped: HashMap<String, Vec<Row>> = HashMap::new();
            for row in rows {
                let tiers = gear::affix_tiers(&row.id);
                let key = if tiers.len() > 1 {
                    tiers[0].group_id.clone()
                } else {
                    row.id.clone()
                };
                grouped.entry(key).or_default().push(row);
            }
            rows = grouped
                .into_values()
                .map(|mut members| {
                    members.sort_by(|a, b| {
                        let strength = |r: &Row| {
                            data::get_affix(&r.id)
                                .and_then(|a| a.value_max)
                                .unwrap_or(f64::NEG_INFINITY)
                        };
                        strength(a).total_cmp(&strength(b)).then(a.id.cmp(&b.id))
                    });
                    let names = members
                        .iter()
                        .map(|r| r.label.clone())
                        .collect::<Vec<_>>()
                        .join(" · ");
                    let multiple = members.len() > 1;
                    let mut row = members.pop().unwrap();
                    if multiple {
                        let tiers = gear::affix_tiers(&row.id);
                        let affix = tiers
                            .iter()
                            .copied()
                            .find(|a| a.value_min.is_some() && a.value_max.is_some())
                            .unwrap_or_else(|| data::get_affix(&row.id).unwrap());
                        row.label = affix.description.clone();
                        if let (Some(min), Some(max), Some((lo, hi))) = (
                            affix.value_min,
                            affix.value_max,
                            gear::affix_roll_bounds(&row.id, None),
                        ) {
                            let token = |a: f64, b: f64| {
                                if a.abs() == b.abs() {
                                    format!("{}", a.abs())
                                } else {
                                    format!("[{}-{}]", a.abs(), b.abs())
                                }
                            };
                            row.label = row.label.replacen(&token(min, max), &token(lo, hi), 1);
                        }
                        row.detail = names;
                        row.tier = None;
                    }
                    row
                })
                .collect();
        }
        rows.retain(|row| {
            format!("{} {}", row.label, row.detail)
                .to_lowercase()
                .contains(&query)
        });
        if self.picker != Picker::Stash {
            rows.sort_by(|a, b| {
                a.group
                    .cmp(b.group)
                    .then(a.label.cmp(&b.label))
                    .then(a.id.cmp(&b.id))
            });
        }
        if rows.len() != self.rows.len()
            || rows.iter().zip(&self.rows).any(|(a, b)| {
                a.id != b.id || a.label != b.label || a.detail != b.detail || a.group != b.group
            })
        {
            self.picker_list.reset(rows.len());
        }
        self.rows = rows;
    }
    /// The candidate's current value for the open picker, for row highlighting.
    fn current_pick(&self) -> Option<String> {
        let item = self.candidate.as_ref()?;
        match self.picker {
            Picker::Socket(index) => item.socketed.get(index).cloned().flatten(),
            Picker::Runeword => item.runeword_id.clone(),
            Picker::Forge => item.forged_mods.first().map(|m| m.affix_id.clone()),
            Picker::Augment => item.augment.as_ref().map(|a| a.id.clone()),
            Picker::RandomSkill => item.random_skill_id.clone(),
            Picker::RandomElement => item.random_skill_element.clone(),
            Picker::Subskill => item.subskill_boost_skill_id.clone(),
            Picker::Class => item.all_skills_class_id.clone(),
            Picker::Items | Picker::Stash | Picker::Affix => None,
        }
    }

    fn pick(&mut self, id: &str, cx: &mut Context<Self>) {
        if matches!(self.picker, Picker::Items | Picker::Stash) {
            self.show_all_affixes = false;
            self.sections_mode = None;
            self.section_overrides.clear();
            self.sliders.clear();
        }
        let result = match self.picker {
            Picker::Items => gear::make_item(id).map(|item| self.candidate = Some(item)),
            Picker::Stash => self
                .session
                .read(cx)
                .draft()
                .stash
                .iter()
                .find(|entry| entry.id == id)
                .map(|entry| self.candidate = Some(entry.item.clone()))
                .ok_or_else(|| tr("gear.stash_missing").to_owned()),
            picker => {
                if let Some(item) = &mut self.candidate {
                    match picker {
                        Picker::Socket(index) => gear::set_socket(item, index, Some(id)),
                        Picker::Runeword => gear::apply_runeword(item, id),
                        Picker::Affix => {
                            gear::add_affix_with_pool_override(item, id, self.show_all_affixes)
                        }
                        Picker::Forge => gear::set_forge(item, Some(id)),
                        Picker::Augment => gear::set_augment(item, Some(id)),
                        Picker::RandomSkill => {
                            item.random_skill_id = (!id.is_empty()).then(|| id.into());
                            Ok(())
                        }
                        Picker::RandomElement => {
                            item.random_skill_element = (!id.is_empty()).then(|| id.into());
                            Ok(())
                        }
                        Picker::Subskill => {
                            item.subskill_boost_skill_id = (!id.is_empty()).then(|| id.into());
                            Ok(())
                        }
                        Picker::Class => {
                            item.all_skills_class_id = (!id.is_empty()).then(|| id.into());
                            Ok(())
                        }
                        _ => unreachable!(),
                    }
                } else {
                    Err(tr("gear.choose_first").into())
                }
            }
        };
        self.error = result.err();
        if self.error.is_none() {
            self.choosing = false;
        }
        self.load_icon();
        self.changed(cx);
    }
    fn edit(&mut self, cx: &mut Context<Self>, edit: impl FnOnce(&mut EquippedItem)) {
        if let Some(item) = &mut self.candidate {
            edit(item);
            self.error = None;
            self.changed(cx);
        }
    }
    fn changed(&mut self, cx: &mut Context<Self>) {
        self.comparison_revision += 1;
        self.differences = None;
        self.verdict = None;
        if self.active && self.editing && !self.comparing {
            self.compare(cx);
        }
        cx.notify();
    }
    fn compare(&mut self, cx: &mut Context<Self>) {
        self.comparing = true;
        let revision = self.comparison_revision;
        let input = self.session.read(cx).snapshot().clone();
        let extra = self.session.read(cx).state().settings.extra_charm_slot;
        let candidate = self.candidate.clone();
        let slot = self.slot.clone();
        let mercenary = self.mercenary;
        let task = cx.background_spawn(async move {
            let mut after = input.clone();
            gear::commit(&mut after, &slot, candidate, mercenary, extra)?;
            let before = evaluate(&input.planner_input());
            let after = evaluate(&after.planner_input());
            Ok::<_, String>((
                compare_planner(&before, &after),
                editor::comparison_verdict(&before.current, &after.current),
            ))
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.comparing = false;
                if revision == this.comparison_revision {
                    match result {
                        Ok((rows, verdict)) => {
                            this.differences = Some(rows);
                            this.verdict = Some(verdict);
                        }
                        Err(error) => this.error = Some(error),
                    }
                } else if this.active && this.editing {
                    this.compare(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn apply(&mut self, cx: &mut Context<Self>) {
        let candidate = self.candidate.clone();
        let slot = self.slot.clone();
        let mercenary = self.mercenary;
        let result = self.session.update(cx, |session, cx| {
            let extra = session.state().settings.extra_charm_slot;
            let mut result = Ok(());
            session.edit(|draft| {
                result = gear::commit(&mut draft.snapshot, &slot, candidate, mercenary, extra)
            });
            if result.is_ok() {
                cx.notify();
            }
            result
        });
        self.error = result.err();
        cx.notify();
    }
    pub(super) fn section_open(&self, key: &str, default_open: bool) -> bool {
        self.section_overrides
            .get(key)
            .copied()
            .or(self.sections_mode)
            .unwrap_or(default_open)
    }
    pub(super) fn toggle_section(&mut self, key: &str, default_open: bool, cx: &mut Context<Self>) {
        let open = self.section_open(key, default_open);
        self.section_overrides.insert(key.to_owned(), !open);
        cx.notify();
    }
    pub(super) fn set_sections_mode(&mut self, expanded: bool, cx: &mut Context<Self>) {
        self.sections_mode = Some(expanded);
        self.section_overrides.clear();
        cx.notify();
    }
    /// One slider entity per rollable stat, created on first render and kept while the item is edited.
    fn roll_slider(
        &mut self,
        key: String,
        bounds: (f64, f64),
        value: f64,
        target: RollTarget,
        cx: &mut Context<Self>,
    ) -> Entity<SliderState> {
        if let Some(binding) = self.sliders.get_mut(&key) {
            if binding
                .state
                .update(cx, |state, _| sync_roll_slider_bounds(state, bounds, value))
            {
                binding.observed_value = binding.state.read(cx).value().start();
            }
            return binding.state.clone();
        }
        let state = cx.new(|_| roll_slider_state(bounds, value));
        let observer_key = key.clone();
        let pointer_key = key.clone();
        let subscriptions = vec![
            cx.observe(&state, move |this, state, cx| {
                this.accept_roll_value(&observer_key, &state, false, cx);
            }),
            cx.subscribe(&state, move |this, state, event: &SliderEvent, cx| {
                if matches!(event, SliderEvent::Change(_)) {
                    // A click on the current endpoint can still pin an unpinned roll.
                    this.accept_roll_value(&pointer_key, &state, true, cx);
                }
            }),
        ];
        self.sliders.insert(
            key,
            RollSlider {
                state: state.clone(),
                focus: cx.focus_handle(),
                observed_value: state.read(cx).value().start(),
                target,
                _subscriptions: subscriptions,
            },
        );
        state
    }
    fn accept_roll_value(
        &mut self,
        key: &str,
        state: &Entity<SliderState>,
        pointer: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(binding) = self.sliders.get_mut(key) else {
            return;
        };
        if binding.state.entity_id() != state.entity_id() {
            return;
        }
        let Some(item) = self.candidate.as_mut() else {
            return;
        };
        if apply_roll_value(
            item,
            &binding.target,
            &mut binding.observed_value,
            state.read(cx).value().start(),
            pointer,
        ) {
            self.error = None;
            self.changed(cx);
        }
    }
    pub(super) fn set_slider(&mut self, key: &str, value: f64, window: &mut Window, cx: &mut App) {
        if let Some(binding) = self.sliders.get_mut(key) {
            binding.observed_value = value as f32;
            binding
                .state
                .update(cx, |state, cx| state.set_value(value as f32, window, cx));
        }
    }

    fn roll_control(
        &self,
        key: &str,
        label: &str,
        window: &Window,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let binding = &self.sliders[key];
        let key = key.to_owned();
        let mouse_key = key.clone();
        div()
            .id(SharedString::from(format!("roll-control-{key}")))
            .flex_1()
            .min_w_0()
            .track_focus(&binding.focus)
            .role(accesskit::Role::Group)
            .aria_label(trf("gear.roll_label", &[("label", (label).to_string())]))
            .rounded_sm()
            .when(binding.focus.is_focused(window), |v| {
                v.shadow(vec![BoxShadow {
                    color: cx.global::<TooltipTheme>().accent_deep,
                    offset: point(px(0.), px(0.)),
                    blur_radius: px(0.),
                    spread_radius: window.rem_size() / 13.,
                    inset: false,
                }])
            })
            .capture_any_mouse_down(
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    if event.button == MouseButton::Left
                        && let Some(binding) = this.sliders.get(&mouse_key)
                    {
                        window.focus(&binding.focus, cx);
                    }
                }),
            )
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                let Some(binding) = this.sliders.get(&key) else {
                    return;
                };
                if !binding.focus.is_focused(window) {
                    return;
                }
                let Some(value) = roll_key_value(binding.state.read(cx), &event.keystroke.key)
                else {
                    return;
                };
                binding
                    .state
                    .update(cx, |state, cx| state.set_value(value, window, cx));
                cx.stop_propagation();
            }))
            .child(hsplanner_ui::controls::planner_slider(
                &binding.state,
                window,
                cx,
            ))
    }
    fn picker_button(
        &self,
        id: &'static str,
        label: &'static str,
        picker: Picker,
        cx: &Context<Self>,
    ) -> Button {
        hsplanner_ui::controls::segment(id, label, self.picker == picker, cx).on_click(
            cx.listener(move |this, _, window, cx| this.choose_picker(picker, window, cx)),
        )
    }
}

impl Render for GearView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.overview(window, cx)
    }
}

fn roll_slider_state((lo, hi): (f64, f64), value: f64) -> SliderState {
    SliderState::new()
        .max(hi as f32)
        .min(lo as f32)
        .step(if lo.fract() == 0. && hi.fract() == 0. {
            1.
        } else {
            0.1
        })
        .default_value(value.clamp(lo, hi) as f32)
}

fn sync_roll_slider_bounds(state: &mut SliderState, bounds: (f64, f64), value: f64) -> bool {
    if state.min_value() != bounds.0 as f32 || state.max_value() != bounds.1 as f32 {
        *state = roll_slider_state(bounds, value);
        true
    } else {
        false
    }
}

fn roll_key_value(state: &SliderState, key: &str) -> Option<f32> {
    Some(match key {
        "left" | "down" => (state.value().start() - state.step_value()).max(state.min_value()),
        "right" | "up" => (state.value().start() + state.step_value()).min(state.max_value()),
        "home" => state.min_value(),
        "end" => state.max_value(),
        _ => return None,
    })
}

fn apply_roll_value(
    item: &mut EquippedItem,
    target: &RollTarget,
    observed: &mut f32,
    value: f32,
    pointer: bool,
) -> bool {
    if let Some(index) = target.affix {
        let changed = *observed != value || pointer;
        *observed = value;
        return changed
            && if target.forged {
                gear::set_forge_value(item, index, f64::from(value))
            } else {
                gear::set_affix_value(item, index, f64::from(value))
            };
    }
    if target.relic_tier {
        let changed = *observed != value || pointer;
        *observed = value;
        return changed && gear::set_relic_tier(item, value as u32);
    }
    let values = if target.skill {
        &mut item.skill_bonus_overrides
    } else {
        &mut item.implicit_overrides
    };
    let changed = *observed != value;
    *observed = value;
    if changed || (pointer && values.get(&target.stat) != Some(&f64::from(value))) {
        values.insert(target.stat.clone(), f64::from(value));
        true
    } else {
        false
    }
}

fn selector_accepts_skill(
    picker: Picker,
    base: Option<&hsplanner_engine::calc::types::ItemBase>,
    class_id: Option<&str>,
    skill: &hsplanner_engine::calc::types::SkillSpec,
) -> bool {
    match picker {
        Picker::RandomSkill => {
            base.and_then(|b| b.random_skill_pool.as_ref())
                .is_some_and(|pool| {
                    pool.class_id == skill.class_id
                        && skill.tree.as_deref() == Some(pool.tree.as_str())
                })
        }
        Picker::Subskill => {
            skill
                .subskills
                .as_ref()
                .is_some_and(|nodes| !nodes.is_empty())
                && class_id.is_none_or(|id| id == skill.class_id)
        }
        _ => false,
    }
}

#[cfg(test)]
mod control_tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn gems_have_no_tier_labels_but_keep_descriptive_text() {
        for gem in data::data().gems.values() {
            assert!(!gem_detail(gem.description.as_deref()).contains("Tier "));
            assert_eq!(editor::socketable_tier(&gem.id), None);
        }
        assert_eq!(gem_detail(Some("Tier S · Soulgem")), "Soulgem");
        assert_eq!(gem_detail(None), "");
        let rune = data::data().runes.values().next().unwrap();
        assert_eq!(editor::socketable_tier(&rune.id), Some(rune.tier));
    }

    fn draft_session() -> Session {
        use hsplanner_build::{
            BuildSnapshot,
            session::{Draft, WorkspaceState},
        };

        Session::new(WorkspaceState {
            draft: Draft {
                build_id: Some("build-a".into()),
                profile_id: Some("profile-a".into()),
                snapshot: BuildSnapshot {
                    level: 100,
                    inventory: HashMap::from([(
                        "weapon".into(),
                        EquippedItem {
                            base_id: "draft-reconciliation-fixture".into(),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
    }

    #[::core::prelude::v1::test]
    fn undo_and_redo_build_edits_preserve_an_open_item_draft() {
        let mut session = draft_session();
        session.edit(|draft| draft.snapshot.level = 99);
        let mut source = CandidateBaseline::default();
        let mut candidate = None;
        source.synchronize(&session, "weapon", false, false, &mut candidate);
        candidate
            .as_mut()
            .unwrap()
            .implicit_overrides
            .insert("all_skills".into(), 3.);
        let edited = serde_json::to_value(&candidate).unwrap();
        let old_document = DocumentKey::from_session(&session);

        session.undo();
        assert_eq!(session.snapshot().level, 100);
        assert_ne!(DocumentKey::from_session(&session), old_document);
        let saved = serde_json::to_value(session.draft()).unwrap();
        assert!(!source.synchronize(&session, "weapon", false, true, &mut candidate));
        assert_eq!(serde_json::to_value(&candidate).unwrap(), edited);
        assert_eq!(serde_json::to_value(session.draft()).unwrap(), saved);

        session.redo();
        assert_eq!(session.snapshot().level, 99);
        assert!(!source.synchronize(&session, "weapon", false, true, &mut candidate));
        assert_eq!(serde_json::to_value(&candidate).unwrap(), edited);
        assert!(
            session.snapshot().inventory["weapon"]
                .implicit_overrides
                .is_empty()
        );

        candidate = None;
        session.edit(|draft| draft.snapshot.level = 98);
        assert!(!source.synchronize(&session, "weapon", false, true, &mut candidate));
        assert!(candidate.is_none());
        assert!(session.snapshot().inventory.contains_key("weapon"));
    }

    #[::core::prelude::v1::test]
    fn clean_item_follows_external_edits_and_profile_changes_replace_dirty_drafts() {
        let mut session = draft_session();
        let mut source = CandidateBaseline::default();
        let mut candidate = None;
        source.synchronize(&session, "weapon", false, false, &mut candidate);

        session.edit(|draft| {
            draft.snapshot.inventory.get_mut("weapon").unwrap().stars = Some(2);
        });
        assert!(source.synchronize(&session, "weapon", false, true, &mut candidate));
        assert_eq!(candidate.as_ref().unwrap().stars, Some(2));

        candidate.as_mut().unwrap().stars = Some(5);
        session.edit(|draft| draft.profile_id = Some("profile-b".into()));
        assert!(source.synchronize(&session, "weapon", false, true, &mut candidate));
        assert_eq!(candidate.as_ref().unwrap().stars, Some(2));

        candidate.as_mut().unwrap().stars = Some(4);
        session.edit(|draft| draft.build_id = Some("build-b".into()));
        assert!(source.synchronize(&session, "weapon", false, true, &mut candidate));
        assert_eq!(candidate.as_ref().unwrap().stars, Some(2));

        candidate.as_mut().unwrap().stars = Some(4);
        assert!(source.synchronize(&session, "weapon", false, false, &mut candidate));
        assert_eq!(candidate.as_ref().unwrap().stars, Some(2));
    }

    #[::core::prelude::v1::test]
    fn slider_keyboard_and_native_value_changes_edit_the_same_roll_and_reset_does_not_repin() {
        let target = RollTarget {
            affix: None,
            forged: false,
            stat: "life".into(),
            skill: false,
            relic_tier: false,
        };
        let mut item = EquippedItem::default();
        let mut observed = 20.;
        let state = roll_slider_state((10., 20.), 20.);
        let from_key = roll_key_value(&state, "left").unwrap();
        assert_eq!(from_key, 19.);
        assert!(apply_roll_value(
            &mut item,
            &target,
            &mut observed,
            from_key,
            false
        ));
        assert_eq!(item.implicit_overrides.get("life"), Some(&19.));
        // Accessibility increments call set_value + notify, taking this same observer path.
        assert!(apply_roll_value(
            &mut item,
            &target,
            &mut observed,
            20.,
            false
        ));
        assert_eq!(item.implicit_overrides.get("life"), Some(&20.));
        assert!(!apply_roll_value(
            &mut item,
            &target,
            &mut observed,
            20.,
            true
        ));
        item.implicit_overrides.remove("life");
        observed = 20.; // The programmatic reset updates the observation before notifying.
        assert!(!apply_roll_value(
            &mut item,
            &target,
            &mut observed,
            20.,
            false
        ));
        assert!(item.implicit_overrides.is_empty());
        assert!(apply_roll_value(
            &mut item,
            &target,
            &mut observed,
            20.,
            true
        ));
        assert_eq!(item.implicit_overrides.get("life"), Some(&20.));
        assert_eq!(roll_key_value(&state, "end"), Some(20.));
        assert_eq!(roll_key_value(&state, "home"), Some(10.));
        assert_eq!(roll_key_value(&state, "right"), Some(20.));
        assert_eq!(roll_key_value(&state, "escape"), None);
    }

    #[::core::prelude::v1::test]
    fn random_pool_uses_the_items_class_and_tree_and_subskills_require_nodes() {
        let base = data::data()
            .items
            .values()
            .find(|base| base.random_skill_pool.is_some())
            .unwrap();
        let pool = base.random_skill_pool.as_ref().unwrap();
        let selected = data::get_skills_by_class(&pool.class_id)
            .iter()
            .filter(|skill| {
                selector_accepts_skill(Picker::RandomSkill, Some(base), Some("stormweaver"), skill)
            })
            .collect::<Vec<_>>();
        assert!(!selected.is_empty());
        assert!(
            selected
                .iter()
                .all(|skill| skill.tree.as_deref() == Some(pool.tree.as_str()))
        );
        for skill in data::data().skills_by_class.values().flatten() {
            assert_eq!(
                selector_accepts_skill(Picker::Subskill, None, None, skill),
                skill
                    .subskills
                    .as_ref()
                    .is_some_and(|nodes| !nodes.is_empty())
            );
        }
    }

    #[::core::prelude::v1::test]
    fn roll_slider_rebuild_uses_changed_star_bounds_without_changing_the_pin() {
        let mut state = roll_slider_state((10., 20.), 15.);
        sync_roll_slider_bounds(&mut state, (15., 30.), 15.);
        assert_eq!(state.min_value(), 15.);
        assert_eq!(state.max_value(), 30.);
        assert_eq!(state.value().start(), 15.);
        sync_roll_slider_bounds(&mut state, (5., 10.), 15.);
        assert_eq!(state.max_value(), 10.);
        assert_eq!(state.value().start(), 10.);
    }
}
