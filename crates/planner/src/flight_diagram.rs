//! Projectile flight preview: the skill's projectile at base speed and with the build, on the game view.
use crate::area_diagram::{self, Colors, TOWN, VIEW_ASPECT, VIEW_WIDTH, View};
use crate::hitbox;
use crate::projectile::{Bonus, FPS, FlightPreview, Flyer, SkillFlight};
use crate::source_breakdown::{format_total, value_row};
use gpui_kit::{prelude::*, *};
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::theme::{self, TooltipTheme};
use std::sync::LazyLock;
use std::time::Instant;

/// Frames between trail dots (0.1 s).
const TRAIL_STEP: f64 = 6.;
/// Frames the finished flight stays on screen before it starts again.
const REPLAY_PAUSE: f64 = 45.;
/// Game pixels past the view's edge before a flight counts as finished.
const EDGE_MARGIN: f64 = 80.;
/// Shared clock, so a repaint continues the flight instead of restarting it.
static CLOCK: LazyLock<Instant> = LazyLock::new(Instant::now);
/// Game pixels each lane sits off the aim line.
const LANE_OFFSET: f32 = 14.;

fn units(value: f32) -> Rems {
    rems(value / 13.)
}

#[derive(Clone, Copy)]
struct Lanes {
    base: Hsla,
    fast: Hsla,
    end: Hsla,
}

impl Lanes {
    fn new(p: &TooltipTheme) -> Self {
        Self {
            base: p.text.opacity(0.6),
            fast: p.positive.opacity(0.9),
            end: p.negative.opacity(0.85),
        }
    }
}

fn per_second(speed: f64) -> String {
    format!("{:.0} px/s", speed * FPS)
}

fn change(base: String, now: String) -> String {
    if base == now {
        base
    } else {
        format!("{base} → {now}")
    }
}

/// GameMaker object names read as words: Demon_Slayer_Bullet_Hell_obj → Demon Slayer Bullet Hell.
fn object_name(name: &str) -> String {
    name.trim_end_matches("_obj").replace('_', " ")
}

fn range_text(flyer: &Flyer, speed: f64) -> String {
    match flyer.range(speed) {
        Some(range) => format!("{range:.0} px"),
        None if flyer.drops => tr("planner.sources.flight_drops").to_owned(),
        None => tr("planner.sources.flight_hits").to_owned(),
    }
}

/// Section value: speed of the headline object, base → with the build.
pub(crate) fn headline(preview: &FlightPreview) -> Option<String> {
    let flyer = preview.headline()?;
    Some(change(
        per_second(preview.base(flyer)),
        per_second(preview.speed(flyer)),
    ))
}

pub(crate) fn body(preview: &FlightPreview, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let colors = Colors::new(p);
    let lanes = Lanes::new(p);
    let tags: Vec<_> = preview
        .tags
        .iter()
        .filter(|tag| ["Projectile", "Melee", "Ranged"].contains(&tag.as_str()))
        .map(|tag| crate::skill_details::tag_name(tag).to_uppercase())
        .collect();
    let mut content = div().px_3().py_2().flex().flex_col().gap_2().child(
        div()
            .flex()
            .items_baseline()
            .justify_between()
            .gap_2()
            .text_size(units(11.))
            .child(div().min_w_0().child(preview.skill_name.clone()))
            .child(
                div()
                    .flex_shrink_0()
                    .font_family(theme::MONO_FONT_FAMILY)
                    .text_size(units(9.))
                    .text_color(p.faint)
                    .child(tags.join(" · ")),
            ),
    );
    content = match (preview.flight, preview.headline()) {
        (Some(_), Some(flyer)) => content
            .child(diagram(preview, flyer, colors, lanes))
            .child(legend(lanes, colors, cx)),
        (Some(SkillFlight { note, .. }), None) => {
            content.child(note_text(note.clone().unwrap_or_default(), cx))
        }
        (None, _) => content.child(note_text(
            trf(
                "planner.sources.flight_missing",
                &[("0", preview.skill_name.clone())],
            ),
            cx,
        )),
    };
    content.child(rows(preview, cx))
}

fn note_text(text: String, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .text_size(units(10.))
        .text_color(p.muted)
        .whitespace_normal()
        .child(text)
}

fn rows(preview: &FlightPreview, cx: &App) -> Div {
    let mut rows = div().flex().flex_col().gap_1();
    for term in &preview.terms {
        let name = crate::stat_labels::stat_name(term.key, term.key);
        rows = rows.child(value_row(
            &name,
            format_total((term.value, term.value), !term.flat),
            false,
            cx,
        ));
    }
    if preview.own != 0. {
        rows = rows.child(value_row(
            &trf(
                "planner.sources.area_own",
                &[("0", preview.skill_name.clone())],
            ),
            format_total((preview.own, preview.own), true),
            false,
            cx,
        ));
    }
    let objects = preview.flight.map_or(&[][..], |f| &f.objects[..]);
    for flyer in objects {
        let (base, now) = (preview.base(flyer), preview.speed(flyer));
        let label = if objects.len() > 1 {
            object_name(&flyer.name)
        } else {
            tr("planner.sources.flight_speed").to_owned()
        };
        rows = rows.child(value_row(
            &label,
            change(per_second(base), per_second(now)),
            true,
            cx,
        ));
        if flyer.result != Bonus::No {
            rows = rows.child(value_row(
                tr("planner.sources.flight_range"),
                change(range_text(flyer, base), range_text(flyer, now)),
                false,
                cx,
            ));
        }
        if let Some(frames) = flyer.lifetime {
            rows = rows.child(value_row(
                tr("planner.sources.flight_lifetime"),
                format!("{:.2} s", frames / FPS),
                false,
                cx,
            ));
        }
        if let Some(why) = flyer.why.as_ref().filter(|_| flyer.result != Bonus::Yes) {
            rows = rows.child(note_text(why.clone(), cx));
        }
        if flyer.percent_only {
            rows = rows.child(note_text(
                tr("planner.sources.flight_percent_only").to_owned(),
                cx,
            ));
        }
    }
    if preview.percent() + preview.own < 0. {
        rows = rows.child(note_text(
            tr("planner.sources.flight_negative").to_owned(),
            cx,
        ));
    }
    if !preview.ignored.is_empty() {
        let list = preview
            .ignored
            .iter()
            .map(|term| {
                format!(
                    "{} {}",
                    crate::stat_labels::stat_name(term.key, term.key),
                    format_total((term.value, term.value), !term.flat)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        rows = rows.child(note_text(
            trf(
                "planner.sources.flight_ignored",
                &[("0", preview.skill_name.clone()), ("1", list)],
            ),
            cx,
        ));
    }
    if preview.titan != 0. {
        rows = rows.child(note_text(
            trf(
                "planner.sources.flight_titan",
                &[("0", format_total((preview.titan, preview.titan), true))],
            ),
            cx,
        ));
    }
    rows
}

fn legend(lanes: Lanes, colors: Colors, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let swatch = |color: Hsla| div().flex_shrink_0().size(units(8.)).bg(color);
    let item = |mark: Div, label: &str| {
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(mark)
            .child(label.to_owned())
    };
    div()
        .flex()
        .flex_col()
        .gap_1()
        .text_size(units(9.))
        .text_color(p.muted)
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_x_3()
                .gap_y_1()
                .child(item(swatch(lanes.fast), tr("planner.sources.area_now")))
                .child(item(swatch(lanes.base), tr("planner.sources.flight_base")))
                .child(item(
                    swatch(lanes.end).w(units(2.)),
                    tr("planner.sources.flight_end"),
                ))
                .child(item(
                    swatch(colors.player).rounded_full(),
                    tr("planner.sources.area_player"),
                )),
        )
        .child(
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_color(p.faint)
                .child(tr("planner.sources.flight_view")),
        )
}

fn diagram(
    preview: &FlightPreview,
    flyer: &'static Flyer,
    colors: Colors,
    lanes: Lanes,
) -> impl IntoElement {
    let runs = [
        (preview.base(flyer), -LANE_OFFSET, lanes.base),
        (preview.speed(flyer), LANE_OFFSET, lanes.fast),
    ];
    div()
        .relative()
        .w_full()
        .aspect_ratio(VIEW_ASPECT)
        .overflow_hidden()
        .bg(colors.view)
        .child(
            img(TOWN.clone())
                .absolute()
                .inset_0()
                .size_full()
                .object_fit(ObjectFit::Cover),
        )
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, (), window, _| paint(bounds, flyer, runs, colors, lanes, window),
            )
            .absolute()
            .inset_0()
            .size_full(),
        )
}

fn paint(
    bounds: Bounds<Pixels>,
    flyer: &Flyer,
    runs: [(f64, f32, Hsla); 2],
    colors: Colors,
    lanes: Lanes,
    window: &mut Window,
) {
    let center = bounds.center();
    let view = View {
        center: [f32::from(center.x), f32::from(center.y)],
        zoom: f32::from(bounds.size.width) / VIEW_WIDTH,
        device: window.scale_factor(),
    };
    // Game pixels from the player to the right edge, where the trail leaves the view.
    let edge = f64::from(VIEW_WIDTH / 2.);
    let stop = |speed: f64| {
        flyer
            .range(speed)
            .unwrap_or(f64::INFINITY)
            .min(edge + EDGE_MARGIN)
    };
    // Both lanes replay together once the slower one has finished.
    let period = runs
        .iter()
        .map(|&(speed, _, _)| stop(speed) / speed)
        .fold(0., f64::max)
        + REPLAY_PAUSE;
    let time = (CLOCK.elapsed().as_secs_f64() * FPS) % period;
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        window.paint_quad(fill(bounds, colors.dim));
        area_diagram::paint_grid(window, bounds, &view, colors);
        for (speed, offset, color) in runs {
            let end = flyer.range(speed).unwrap_or(f64::INFINITY);
            let stop = stop(speed);
            let at = |distance: f64| {
                [
                    view.center[0] + distance as f32 * view.zoom,
                    view.center[1] + offset * view.zoom,
                ]
            };
            let dot = 1.5 * view.zoom.max(1.);
            let mut frame = TRAIL_STEP;
            while frame <= time && speed * frame <= stop {
                let [x, y] = at(speed * frame);
                area_diagram::paint_rect(window, [x - dot, y - dot, x + dot, y + dot], color);
                frame += TRAIL_STEP;
            }
            if end <= edge {
                let [x, y] = at(end);
                let half = 9. * view.zoom;
                area_diagram::paint_rect(window, [x - 1., y - half, x + 1., y + half], lanes.end);
            }
            let done = speed * time >= stop;
            let [x, y] = at((speed * time).min(stop));
            let shape = View {
                center: [x, y],
                zoom: view.zoom,
                device: view.device,
            };
            let alpha = if done { 0.2 } else { 0.55 };
            paint_flyer(window, &shape, flyer, color.opacity(alpha));
        }
        let player = hitbox::player();
        if let Some(mask) = hitbox::mask(&player.mask) {
            area_diagram::paint_mask(
                window,
                &view,
                mask,
                [0., 0.],
                player.origin,
                [1., 1.],
                colors.player,
            );
        }
    });
    window.paint_quad(outline(bounds, colors.frame, BorderStyle::Solid));
    window.request_animation_frame();
}

fn paint_flyer(window: &mut Window, view: &View, flyer: &Flyer, color: Hsla) {
    let scale = [flyer.scale, flyer.scale];
    if let Some(mask) = flyer.mask.as_deref().and_then(hitbox::mask) {
        area_diagram::paint_mask(window, view, mask, flyer.at, flyer.origin, scale, color);
    } else if let Some([x0, y0, x1, y1]) = flyer.bounds {
        let x = |v: f32| view.center[0] + (v - flyer.origin[0]) * flyer.scale * view.zoom;
        let y = |v: f32| view.center[1] + (v - flyer.origin[1]) * flyer.scale * view.zoom;
        area_diagram::paint_rect(window, [x(x0), y(y0), x(x1 + 1.), y(y1 + 1.)], color);
    }
}
