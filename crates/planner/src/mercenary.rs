use crate::{TreeView, build_panel::format_range, gear::GearView};
use gpui_kit::base::Disableable;
use gpui_kit::component::{
    Sizable,
    button::{Button, ButtonCustomVariant, ButtonVariants},
    checkbox::Checkbox,
};
use gpui_kit::{prelude::*, *};
use hsplanner_build::{BuildSnapshot, session::Session};
use hsplanner_engine::calc::{data, mercenary};
use hsplanner_ui::{
    components::{panel_with_trailing, section_heading},
    controls::{PlannerControl, icon_button},
    theme::{self, TooltipTheme},
};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};

include!("mercenary_icons.rs");
static IMAGES: LazyLock<HashMap<&'static str, Arc<RenderImage>>> = LazyLock::new(|| {
    MERC_ICONS
        .iter()
        .filter_map(|(key, bytes)| {
            let mut rgba = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
                .ok()?
                .into_rgba8();
            for p in rgba.pixels_mut() {
                p.0.swap(0, 2);
            }
            Some((
                *key,
                Arc::new(RenderImage::new(vec![image::Frame::new(rgba)])),
            ))
        })
        .collect()
});

pub struct MercenaryView {
    session: Entity<Session>,
    gear: Entity<GearView>,
    tree: Entity<TreeView>,
    show_equipment_stats: bool,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}
impl MercenaryView {
    pub fn new(
        session: Entity<Session>,
        tree: Entity<TreeView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let gear = cx.new(|cx| GearView::new(session.clone(), true, window, cx));
        let subscriptions = vec![
            cx.observe(&session, |_, _, cx| cx.notify()),
            cx.observe(&tree, |_, _, cx| cx.notify()),
        ];
        Self {
            session,
            gear,
            tree,
            show_equipment_stats: false,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }
    pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
        self.gear.update(cx, |gear, cx| gear.set_active(active, cx));
    }
    fn edit(&mut self, cx: &mut Context<Self>, edit: impl FnOnce(&mut BuildSnapshot)) {
        self.session.update(cx, |session, cx| {
            session.edit(|draft| edit(&mut draft.snapshot));
            cx.notify();
        });
    }
    fn skill_panel(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let Some(class) = mercenary::data()
            .classes
            .iter()
            .find(|c| Some(c.id.as_str()) == snapshot.merc_class_id.as_deref())
        else {
            return div();
        };
        panel_with_trailing(
            "merc-skills-panel",
            format!("{} Skills", class.name),
            div()
                .flex()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(10. / 13.))
                .text_color(p.faint)
                .child(
                    div().text_color(p.accent_hot).child(
                        snapshot
                            .merc_skill_ranks
                            .values()
                            .copied()
                            .sum::<u32>()
                            .to_string(),
                    ),
                )
                .child(" POINTS"),
            cx,
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1p5()
                .children(class.skills.iter().map(|skill| {
                    let id = skill.id.clone();
                    let minus = id.clone();
                    let rank = snapshot.merc_skill_ranks.get(&id).copied().unwrap_or(0);
                    let max = mercenary::data().max_skill_rank;
                    let icon = IMAGES.get(format!("{}/{}", class.id, id).as_str());
                    div()
                        .id(SharedString::from(id.clone()))
                        .rounded_sm()
                        .border_1()
                        .border_color(if rank > 0 {
                            p.accent_deep.opacity(0.5)
                        } else {
                            p.border
                        })
                        .bg(if rank > 0 {
                            p.accent_hot.opacity(0.04)
                        } else {
                            p.background.opacity(0.)
                        })
                        .px_2p5()
                        .py_2()
                        .flex()
                        .items_start()
                        .gap_2p5()
                        .child(
                            div()
                                .size(rems(34. / 13.))
                                .flex_shrink_0()
                                .border_1()
                                .rounded_sm()
                                .border_color(p.border_strong)
                                .bg(p.background)
                                .children(icon.map(|i| {
                                    img(i.clone()).size_full().object_fit(ObjectFit::Contain)
                                })),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap_0p5()
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .gap_2()
                                        .items_baseline()
                                        .child(
                                            div()
                                                .text_size(rems(12. / 13.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(if rank > 0 {
                                                    p.accent_hot
                                                } else {
                                                    p.text
                                                })
                                                .child(skill.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .font_family(theme::MONO_FONT_FAMILY)
                                                .text_size(rems(8.5 / 13.))
                                                .text_color(p.faint)
                                                .child(format!(
                                                    "{}{}",
                                                    skill.kind.to_uppercase(),
                                                    skill
                                                        .damage_type
                                                        .as_ref()
                                                        .map(|kind| format!(
                                                            " · {}",
                                                            kind.to_uppercase()
                                                        ))
                                                        .unwrap_or_default()
                                                )),
                                        )
                                        .when(skill.shared, |v| {
                                            v.child(
                                                div()
                                                    .border_1()
                                                    .rounded_sm()
                                                    .px_1()
                                                    .border_color(p.positive.opacity(0.4))
                                                    .text_size(rems(8.5 / 13.))
                                                    .text_color(p.positive)
                                                    .child("HERO"),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .text_size(rems(10.5 / 13.))
                                        .text_color(p.muted)
                                        .child(skill.description.clone()),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_1()
                                .items_center()
                                .self_center()
                                .child(
                                    icon_button("minus", "−", false, cx)
                                        .disabled(rank == 0)
                                        .accessibility_label(format!(
                                            "Decrease {} rank",
                                            skill.name
                                        ))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.edit(cx, |s| {
                                                s.set_mercenary_skill(
                                                    &minus,
                                                    rank.saturating_sub(1),
                                                )
                                            })
                                        })),
                                )
                                .child(
                                    div()
                                        .w_12()
                                        .text_center()
                                        .font_family(theme::MONO_FONT_FAMILY)
                                        .text_size(rems(11. / 13.))
                                        .text_color(if rank > 0 { p.accent_hot } else { p.faint })
                                        .child(format!("{rank}/{max}")),
                                )
                                .child(
                                    icon_button("plus", "+", false, cx)
                                        .disabled(rank >= max)
                                        .accessibility_label(format!(
                                            "Increase {} rank",
                                            skill.name
                                        ))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.edit(cx, |s| s.set_mercenary_skill(&id, rank + 1))
                                        })),
                                ),
                        )
                })),
        )
    }
    fn shared_panel(&self, cx: &Context<Self>) -> Div {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let performance = self.tree.read(cx).performance();
        let magic = performance.as_ref().map(|p| {
            p.mercenary
                .stats
                .get("magic_find")
                .copied()
                .unwrap_or((0., 0.))
        });
        let mut effects = Vec::new();
        let mut auras = std::collections::BTreeMap::new();
        for item in snapshot.merc_inventory.values() {
            if let Some(base) = data::get_item(&item.base_id) {
                for (name, _) in data::skill_bonus_entries(base, item) {
                    if data::get_item_granted_skill_by_name(name).is_some_and(|s| s.aura) {
                        auras.insert(
                            name.trim().to_lowercase(),
                            (name.clone(), base.name.clone()),
                        );
                    }
                }
                for effect in base.unique_effects.iter().flatten() {
                    effects.push((base.name.clone(), effect.clone()));
                }
            }
        }
        let no_buffs = auras.is_empty() && effects.is_empty();
        let shared = mercenary::data()
            .classes
            .iter()
            .find(|c| Some(c.id.as_str()) == snapshot.merc_class_id.as_deref())
            .map(|c| {
                c.skills
                    .iter()
                    .filter(|s| {
                        s.shared && snapshot.merc_skill_ranks.get(&s.id).copied().unwrap_or(0) > 0
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut view = div()
            .rounded_sm()
            .border_1()
            .border_color(p.border)
            .bg(p.panel)
            .child(
                div()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(p.border)
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(rems(10. / 13.))
                    .text_color(p.positive)
                    .child("◆  SHARED WITH HERO"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(p.border)
                    .text_size(rems(12. / 13.))
                    .child("Magic Find")
                    .child(
                        div()
                            .font_family(theme::MONO_FONT_FAMILY)
                            .text_color(p.accent_hot)
                            .child(
                                magic
                                    .filter(|v| *v != (0., 0.))
                                    .map(|v| format_range(v, true))
                                    .unwrap_or_else(|| "—".into()),
                            ),
                    ),
            );
        let heading = |title: &str| {
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_size(rems(9. / 13.))
                .text_color(p.faint)
                .child(title.to_owned())
        };
        let mut buffs = div()
            .px_3()
            .py_2p5()
            .flex()
            .flex_col()
            .gap_1p5()
            .child(heading("ITEM BUFFS"));
        if no_buffs {
            buffs = buffs.child(div().text_size(rems(11. / 13.)).text_color(p.muted).child(
                "No shared item buffs — equip uniques with effects (e.g. Pearlescent Dream).",
            ));
        }
        for (key, (name, source)) in auras {
            let checked = !snapshot
                .merc_disabled_auras
                .get(&key)
                .copied()
                .unwrap_or(false);
            buffs = buffs.child(
                div()
                    .id(SharedString::from(format!("aura-row-{key}")))
                    .child(
                        Checkbox::new(SharedString::from(format!("aura-{key}")))
                            .label(name)
                            .checked(checked)
                            .on_click(cx.listener(move |this, enabled: &bool, _, cx| {
                                this.edit(cx, |s| {
                                    if *enabled {
                                        s.merc_disabled_auras.remove(&key);
                                    } else {
                                        s.merc_disabled_auras.insert(key.clone(), true);
                                    }
                                })
                            })),
                    )
                    .child(
                        div()
                            .text_size(rems(10. / 13.))
                            .text_color(p.faint)
                            .child(source),
                    ),
            );
        }
        for (source, effect) in effects {
            buffs = buffs.child(
                div()
                    .text_size(rems(11.5 / 13.))
                    .text_color(p.positive)
                    .child(effect)
                    .child(
                        div()
                            .text_size(rems(10. / 13.))
                            .text_color(p.faint)
                            .child(source),
                    ),
            );
        }
        buffs = buffs.child(
            div()
                .mt_2()
                .pt_2()
                .border_t_1()
                .border_color(p.border)
                .child(heading("SKILL EFFECTS")),
        );
        if shared.is_empty() {
            buffs = buffs.child(
                div()
                    .text_size(rems(11. / 13.))
                    .text_color(p.muted)
                    .child("No hero-affecting skills leveled yet."),
            );
        }
        for skill in shared {
            buffs = buffs.child(
                div()
                    .text_size(rems(11.5 / 13.))
                    .text_color(p.positive)
                    .child(format!(
                        "{}  {}/{}",
                        skill.name,
                        snapshot.merc_skill_ranks[&skill.id],
                        mercenary::data().max_skill_rank
                    ))
                    .child(
                        div()
                            .text_size(rems(10. / 13.))
                            .text_color(p.faint)
                            .child(skill.description.clone()),
                    ),
            );
        }
        view = view.child(buffs).child(
            Button::new("merc-stat-details")
                .planner_style(cx)
                .small()
                .m_3()
                .label(if self.show_equipment_stats {
                    "Hide equipment stats"
                } else {
                    "Equipment stats"
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.show_equipment_stats = !this.show_equipment_stats;
                    cx.notify();
                })),
        );
        if self.show_equipment_stats
            && let Some(performance) = performance
        {
            let mut rows = performance
                .mercenary
                .stats
                .iter()
                .filter(|(_, v)| **v != (0., 0.))
                .collect::<Vec<_>>();
            rows.sort_by_key(|(key, _)| *key);
            view = view.child(
                div()
                    .px_3()
                    .pb_3()
                    .children(rows.into_iter().map(|(key, v)| {
                        let def = data::game_config().stats.iter().find(|s| &s.key == key);
                        div()
                            .flex()
                            .justify_between()
                            .gap_2()
                            .text_size(rems(11. / 13.))
                            .child(def.map_or(key.clone(), |d| d.name.clone()))
                            .child(format_range(
                                *v,
                                def.is_some_and(|d| d.format.as_deref() == Some("percent")),
                            ))
                    })),
            );
        }
        view
    }
}
impl Render for MercenaryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.global::<TooltipTheme>();
        let snapshot = self.session.read(cx).snapshot();
        let selected = snapshot.merc_class_id.as_deref();
        let used = snapshot.merc_inventory.len();
        let spent: u32 = snapshot.merc_skill_ranks.values().copied().sum();
        let wide = window.viewport_size().width >= window.rem_size() * (1024. / 13.);
        div().size_full().bg(p.background).child(div().id("mercenary-overview").size_full().track_scroll(&self.scroll).overflow_y_scroll().child(div().p_6().flex().flex_col().gap_4()
            .child(div().flex().items_end().justify_between().gap_3().child(section_heading("merc-heading","Loadout","Mercenary",cx))
                .when(selected.is_some(),|v|v.child(div().flex().gap_3().items_center().font_family(theme::MONO_FONT_FAMILY).text_size(rems(10./13.)).text_color(p.faint)
                    .child(format!("{used} / {} equipped  ·  {spent} skill points",mercenary::data().slots.len()))
                    .child(Button::new("reset-mercenary").planner_style(cx).small().label("Dismiss")
                        .on_click(cx.listener(|this,_,_,cx|this.edit(cx,|s|{s.merc_class_id=None;s.merc_skill_ranks.clear();s.merc_inventory.clear();s.merc_disabled_auras.clear();})))))))
            .child(div().flex().flex_wrap().gap_2p5().children(mercenary::data().classes.iter().map(|class|{
                let id=class.id.clone();let chosen=selected==Some(id.as_str());
                Button::new(SharedString::from(format!("merc-class-{id}"))).planner_style(cx).custom(ButtonCustomVariant::new(cx).color(if chosen { p.accent_hot.opacity(0.08) } else { p.panel }).hover(p.panel_secondary)).min_w(rems(200./13.)).flex_1().h_auto().min_h(rems(56./13.)).px_3().py_2p5().border_color(if chosen{p.accent_hot}else{p.border})
                    .accessibility_label(format!("Hire {}",class.name))
                    .child(div().w_full().flex().items_center().gap_3()
                        .child(div().size_12().flex_shrink_0().border_1().border_color(p.border_strong).rounded_sm().bg(p.background)
                            .children(IMAGES.get(id.as_str()).map(|i|img(i.clone()).size_full().object_fit(ObjectFit::Contain))))
                        .child(div().flex_1().min_w_0().flex().flex_col().items_start()
                            .child(div().font_family(theme::FONT_FAMILY).text_size(rems(1.)).font_weight(FontWeight::SEMIBOLD).text_color(if chosen{p.accent_hot}else{p.text}).child(class.name.clone()))
                            .child(div().text_size(rems(9./13.)).text_color(p.muted).child(class.role.to_uppercase()))
                            .child(div().truncate().font_family(theme::FONT_FAMILY).text_size(rems(10./13.)).text_color(p.faint).child(class.location.clone())))
                        .when(chosen,|v|v.child(div().self_start().text_color(p.accent_hot).child("◆"))))
                    .on_click(cx.listener(move|this,_,_,cx|this.edit(cx,|s|s.set_mercenary_class(Some(&id)))))
            })))
            .when(selected.is_none(),|v|v.child(div().border_1().rounded_sm().border_color(p.border).px_4().py_10().text_center().text_color(p.muted)
                .child("No mercenary hired — pick a class above.").child(div().mt_1p5().text_size(rems(11./13.)).text_color(p.faint).child("Mercenaries fight beside your hero. Their Magic Find counts for your drops, and buffs from their items are shared with you."))))
            .when(selected.is_some(),|v|v.child(div().flex().when(!wide, |v| v.flex_col()).items_start().gap_4()
                .child(div().min_w_0().when(wide, |v| v.flex_grow(2.).flex_shrink(1.).flex_basis(px(0.))).when(!wide, |v| v.w_full()).flex().flex_col().gap_4().child(self.gear.clone()).child(self.skill_panel(cx)))
                .child(div().min_w_0().when(wide, |v| v.flex_grow(1.).flex_shrink(1.).flex_basis(px(0.))).when(!wide, |v| v.w_full()).child(self.shared_panel(cx)))))))
    }
}
