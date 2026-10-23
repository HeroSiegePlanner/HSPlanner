//! Path of Building style area preview: the skill's hitboxes to scale on the game view.
use crate::hitbox::{self, AreaPreview, HitObject, Mask, SkillHitbox};
use crate::source_breakdown::{format_total, value_row};
use gpui_kit::{prelude::*, *};
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::theme::{self, TooltipTheme};
use std::sync::{Arc, LazyLock};

/// Game pixels across the camera view at field of view 1.0 (1024 + 256 × FOV).
const VIEW_WIDTH: f32 = 1280.;
const VIEW_ASPECT: f32 = 16. / 9.;
/// Act 1 town at field of view 1.0 and 1920 px wide (1.5 screen pixels per game pixel), with the
/// player at the centre: the same 1280×720 view the hitboxes are drawn on.
static TOWN: LazyLock<Arc<Image>> = LazyLock::new(|| {
    Arc::new(Image::from_bytes(
        ImageFormat::Webp,
        include_bytes!("../../../assets/hit-area/town.webp").to_vec(),
    ))
});
/// Grid step in game pixels.
const GRID_STEP: f32 = 64.;
/// Tags that decide which size stats reach a skill.
const SIZE_TAGS: [&str; 9] = [
    "Area of Effect",
    "Explosion",
    "Orbital",
    "Projectile",
    "Melee",
    "Ranged",
    "Spell",
    "Shield",
    "Relic",
];

fn units(value: f32) -> Rems {
    rems(value / 13.)
}

#[derive(Clone, Copy)]
struct Colors {
    view: Hsla,
    dim: Hsla,
    grid: Hsla,
    axis: Hsla,
    frame: Hsla,
    grown: Hsla,
    base: Hsla,
    reach: Hsla,
    check: Hsla,
    player: Hsla,
}

impl Colors {
    fn new(p: &TooltipTheme) -> Self {
        Self {
            view: p.background,
            dim: p.background.opacity(0.35),
            grid: p.border.opacity(0.3),
            axis: p.border.opacity(0.7),
            frame: p.border.opacity(0.6),
            grown: p.positive.opacity(0.35),
            base: p.text.opacity(0.22),
            reach: p.positive.opacity(0.8),
            check: p.accent.opacity(0.8),
            player: p.stat_blue.opacity(0.9),
        }
    }
}

struct Shape {
    object: &'static HitObject,
    base: [f64; 2],
    now: [f64; 2],
}

/// Game pixels to screen pixels around the skill's spawn point.
struct View {
    center: [f32; 2],
    zoom: f32,
    device: f32,
}

impl View {
    /// GPUI rounds quad edges to device pixels; a row thinner than that would vanish anyway.
    fn visible(&self, top: f32, bottom: f32) -> bool {
        (bottom * self.device).round() > (top * self.device).round()
    }
}

fn reach(object: &HitObject, scale: [f64; 2]) -> f64 {
    f64::from(object.reach) * scale[0].max(scale[1])
}

/// Two decimals, rounding halves up (0.075 → 0.08) as players read them.
fn hundredths(value: f64) -> f64 {
    (value * 100.).round() / 100.
}

fn scale_text(scale: f64) -> String {
    format!("×{:.2}", hundredths(scale))
}

/// Section value: reach of the headline object, base → with the build.
pub(crate) fn headline(preview: &AreaPreview) -> Option<String> {
    let object = preview.headline()?;
    let base = reach(object, object.scale);
    let now = reach(
        object,
        hitbox::object_scale(object, preview.own, preview.bonus),
    );
    Some(if (now - base).abs() < 0.5 {
        format!("{base:.0} px")
    } else {
        format!("{base:.0} → {now:.0} px")
    })
}

pub(crate) fn body(preview: &AreaPreview, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let colors = Colors::new(p);
    let tags: Vec<_> = preview
        .tags
        .iter()
        .filter(|tag| SIZE_TAGS.contains(&tag.as_str()))
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
    match preview.hitbox {
        Some(hitbox) => {
            content = content
                .child(diagram(preview, hitbox, colors))
                .child(legend(hitbox, colors, cx));
        }
        None => {
            content = content.child(note(
                trf(
                    "planner.sources.area_missing",
                    &[("0", preview.skill_name.clone())],
                ),
                cx,
            ));
        }
    }
    content.child(rows(preview, cx))
}

fn note(text: String, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    div()
        .text_size(units(10.))
        .text_color(p.muted)
        .whitespace_normal()
        .child(text)
}

fn rows(preview: &AreaPreview, cx: &App) -> Div {
    let mut rows = div().flex().flex_col().gap_1();
    if preview.own != 0. {
        rows = rows.child(value_row(
            &trf(
                "planner.sources.area_own",
                &[("0", preview.skill_name.clone())],
            ),
            format!(
                "{} → {}",
                format_total((preview.own, preview.own), true),
                scale_text((1. + preview.own / 100.).max(0.))
            ),
            false,
            cx,
        ));
    }
    for term in &preview.terms {
        let name = crate::stat_labels::stat_name(term.key, term.key);
        rows = rows.child(value_row(
            &name,
            format!(
                "{} → {:+.2}",
                format_total((term.percent, term.percent), true),
                hundredths(term.scale)
            ),
            false,
            cx,
        ));
    }
    if let Some(object) = preview.headline() {
        let now = hitbox::object_scale(object, preview.own, preview.bonus);
        let value = if now == object.scale {
            scale_text(now[0])
        } else {
            format!("{} → {}", scale_text(object.scale[0]), scale_text(now[0]))
        };
        rows = rows.child(value_row(tr("planner.sources.area_scale"), value, true, cx));
    }
    if !preview.ignored.is_empty() {
        let list = preview
            .ignored
            .iter()
            .map(|term| {
                format!(
                    "{} {}",
                    crate::stat_labels::stat_name(term.key, term.key),
                    format_total((term.percent, term.percent), true)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        rows = rows.child(note(
            trf(
                "planner.sources.area_ignored",
                &[("0", preview.skill_name.clone()), ("1", list)],
            ),
            cx,
        ));
    }
    if preview.hitbox.is_some_and(|h| h.triggered) {
        rows = rows.child(note(tr("planner.sources.area_triggered").to_owned(), cx));
    }
    rows
}

fn legend(hitbox: &SkillHitbox, colors: Colors, cx: &App) -> Div {
    let p = cx.global::<TooltipTheme>();
    let swatch = |color: Hsla| div().flex_shrink_0().size(units(8.)).bg(color);
    let ring = |color: Hsla| {
        div()
            .flex_shrink_0()
            .size(units(8.))
            .rounded_full()
            .border_1()
            .border_dashed()
            .border_color(color)
    };
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
                .child(item(swatch(colors.grown), tr("planner.sources.area_now")))
                .child(item(swatch(colors.base), tr("planner.sources.area_base")))
                .when(hitbox.objects.iter().any(|o| o.spins), |v| {
                    v.child(item(ring(colors.reach), tr("planner.sources.area_reach")))
                })
                .when(!hitbox.circles.is_empty(), |v| {
                    v.child(item(ring(colors.check), tr("planner.sources.area_radius")))
                })
                .child(item(
                    swatch(colors.player).rounded_full(),
                    tr("planner.sources.area_player"),
                )),
        )
        .child(
            div()
                .font_family(theme::MONO_FONT_FAMILY)
                .text_color(p.faint)
                .child(tr("planner.sources.area_view")),
        )
}

fn diagram(
    preview: &AreaPreview,
    hitbox: &'static SkillHitbox,
    colors: Colors,
) -> impl IntoElement {
    let shapes: Vec<Shape> = hitbox
        .objects
        .iter()
        .map(|object| Shape {
            object,
            base: object.scale,
            now: hitbox::object_scale(object, preview.own, preview.bonus),
        })
        .collect();
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
                move |bounds, (), window, _| paint(bounds, &shapes, hitbox, colors, window),
            )
            .absolute()
            .inset_0()
            .size_full(),
        )
}

fn paint(
    bounds: Bounds<Pixels>,
    shapes: &[Shape],
    hitbox: &SkillHitbox,
    colors: Colors,
    window: &mut Window,
) {
    let center = bounds.center();
    let view = View {
        center: [f32::from(center.x), f32::from(center.y)],
        zoom: f32::from(bounds.size.width) / VIEW_WIDTH,
        device: window.scale_factor(),
    };
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        window.paint_quad(fill(bounds, colors.dim));
        paint_grid(window, bounds, &view, colors);
        for shape in shapes {
            paint_object(window, &view, shape.object, shape.now, colors.grown);
        }
        for shape in shapes.iter().filter(|s| s.now != s.base) {
            paint_object(window, &view, shape.object, shape.base, colors.base);
        }
        for shape in shapes.iter().filter(|s| s.object.spins) {
            paint_ring(
                window,
                &view,
                reach(shape.object, shape.now) as f32,
                colors.reach,
            );
        }
        for &radius in &hitbox.circles {
            paint_ring(window, &view, radius, colors.check);
        }
        let player = hitbox::player();
        if let Some(mask) = hitbox::mask(&player.mask) {
            paint_mask(
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
}

fn paint_rect(window: &mut Window, [x0, y0, x1, y1]: [f32; 4], color: Hsla) {
    window.paint_quad(fill(
        Bounds::from_corners(point(px(x0), px(y0)), point(px(x1), px(y1))),
        color,
    ));
}

fn paint_grid(window: &mut Window, bounds: Bounds<Pixels>, view: &View, colors: Colors) {
    let step = GRID_STEP * view.zoom;
    let hairline = 1. / view.device;
    let [left, top] = [f32::from(bounds.origin.x), f32::from(bounds.origin.y)];
    let [right, bottom] = [f32::from(bounds.right()), f32::from(bounds.bottom())];
    let columns = ((right - left) / 2. / step).ceil() as i32;
    for k in -columns..=columns {
        let x = view.center[0] + k as f32 * step;
        let color = if k == 0 { colors.axis } else { colors.grid };
        paint_rect(window, [x, top, x + hairline, bottom], color);
    }
    let rows = ((bottom - top) / 2. / step).ceil() as i32;
    for k in -rows..=rows {
        let y = view.center[1] + k as f32 * step;
        let color = if k == 0 { colors.axis } else { colors.grid };
        paint_rect(window, [left, y, right, y + hairline], color);
    }
}

fn paint_object(
    window: &mut Window,
    view: &View,
    object: &HitObject,
    scale: [f64; 2],
    color: Hsla,
) {
    let scale = [scale[0] as f32, scale[1] as f32];
    if let Some(mask) = object.mask.as_deref().and_then(hitbox::mask) {
        paint_mask(window, view, mask, object.at, object.origin, scale, color);
    } else if let Some([x0, y0, x1, y1]) = object.bounds {
        let x = |v: f32| view.center[0] + (v - object.origin[0]) * scale[0] * view.zoom;
        let y = |v: f32| view.center[1] + (v - object.origin[1]) * scale[1] * view.zoom;
        paint_rect(window, [x(x0), y(y0), x(x1 + 1.), y(y1 + 1.)], color);
    }
}

/// Mask rows as runs of quads; objects face the aim, drawn here aiming right.
fn paint_mask(
    window: &mut Window,
    view: &View,
    mask: &Mask,
    at: [f32; 2],
    origin: [f32; 2],
    scale: [f32; 2],
    color: Hsla,
) {
    let x = |v: f32| view.center[0] + (at[0] + v - origin[0]) * scale[0] * view.zoom;
    let y = |v: f32| view.center[1] + (at[1] + v - origin[1]) * scale[1] * view.zoom;
    for (row, runs) in mask.rows.iter().enumerate() {
        let (top, bottom) = (y(row as f32), y(row as f32 + 1.));
        if !view.visible(top, bottom) {
            continue;
        }
        for run in runs.chunks_exact(2) {
            paint_rect(
                window,
                [x(f32::from(run[0])), top, x(f32::from(run[1])), bottom],
                color,
            );
        }
    }
}

fn paint_ring(window: &mut Window, view: &View, radius: f32, color: Hsla) {
    let r = radius * view.zoom;
    let [cx, cy] = view.center;
    window.paint_quad(quad(
        Bounds::from_corners(point(px(cx - r), px(cy - r)), point(px(cx + r), px(cy + r))),
        px(r),
        transparent_black(),
        px(1.),
        color,
        BorderStyle::Dashed,
    ));
}
