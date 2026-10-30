//! The skills panel PREVIEW: one use replayed frame by frame against the training dummy, in a loop.
use crate::area_diagram::{self, Colors, VIEW_ASPECT, View};
use crate::hitbox;
use crate::skill_details::{detail_block, units};
use crate::skill_sim::{self, Shape, Timeline};
use crate::skill_sprites::{self, Frames, Layer};
use gpui_kit::{prelude::*, *};
use hsplanner_engine::calc::types::SkillSpec;
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::theme::TooltipTheme;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

const FPS: f64 = 60.;
/// Game pixels shown before the player and past the dummy.
const MARGIN: f32 = 110.;
/// The view widens to fit what a skill draws, up to this many times its base width.
const MAX_ZOOM_OUT: f32 = 2.5;
/// Drawings farther from the dummy, such as a bolt flying on after a miss, leave the view as it is.
const FIT_REACH: f32 = 360.;
/// Game px always in view beside the player and the dummy, above them and below them.
const KEEP: [f32; 3] = [40., 70., 30.];
/// Frames of empty floor before the use plays again.
const LOOP_PAUSE: usize = 30;
/// Time between two drawn frames: every second game frame.
const REDRAW: Duration = Duration::from_millis(33);
/// Frames the dummy's collision ring stays lit after a hit.
const HIT_FLASH: usize = 8;
/// The dummy's collision circle, as the dps cards count it.
const DUMMY_RADIUS: f32 = 12.;
/// The skill on screen, when its loop started and the view that fits it.
static PLAYBACK: Mutex<Option<(String, Instant, Camera)>> = Mutex::new(None);

#[derive(Clone, Copy)]
struct Palette {
    active: Hsla,
    edge: Hsla,
    idle: Hsla,
    dummy: Hsla,
    hit: Hsla,
    player: Hsla,
}

/// The game-px rectangle the view shows, `VIEW_ASPECT` wide to tall.
#[derive(Clone, Copy)]
struct Camera {
    left: f32,
    top: f32,
    width: f32,
}

impl Camera {
    /// The base view widened to what the skill draws near the dummy; the player and the dummy stay in it.
    fn fit(skill_id: &str, timeline: &Timeline) -> Self {
        let dummy = timeline.dummy[0];
        let base = dummy + 2. * MARGIN;
        let half = base / VIEW_ASPECT / 2.;
        let [x0, y0, x1, y1] = timeline
            .frames
            .iter()
            .flatten()
            .flat_map(|shape| drawn(skill_id, shape))
            .filter(|[x0, y0, x1, y1]| ((x0 + x1) / 2. - dummy).hypot((y0 + y1) / 2.) <= FIT_REACH)
            .fold(
                [-MARGIN, -half, dummy + MARGIN, half],
                |[a, b, c, d], [x0, y0, x1, y1]| [a.min(x0), b.min(y0), c.max(x1), d.max(y1)],
            );
        let width = (x1 - x0)
            .max((y1 - y0) * VIEW_ASPECT)
            .min(base * MAX_ZOOM_OUT);
        let height = width / VIEW_ASPECT;
        let [side, above, below] = KEEP;
        Self {
            left: ((x0 + x1 - width) / 2.)
                .min(-side)
                .max(dummy + side - width),
            top: ((y0 + y1 - height) / 2.).min(-above).max(below - height),
            width,
        }
    }

    fn height(self) -> f32 {
        self.width / VIEW_ASPECT
    }

    /// An image covering a game-px rectangle, placed by shares of the view.
    fn place(self, image: Img, [x, y]: [f32; 2], [w, h]: [f32; 2]) -> Img {
        image
            .absolute()
            .left(relative((x - self.left) / self.width))
            .top(relative((y - self.top) / self.height()))
            .w(relative(w / self.width))
            .h(relative(h / self.height()))
            .object_fit(ObjectFit::Fill)
    }
}

/// When the loop of `skill_id` started and the view that fits it; picking another skill starts anew.
fn playback(skill_id: &str, timeline: &Timeline) -> (Instant, Camera) {
    let mut playback = PLAYBACK.lock().unwrap_or_else(PoisonError::into_inner);
    match &*playback {
        Some((id, start, camera)) if id == skill_id => (*start, *camera),
        _ => {
            let (start, camera) = (Instant::now(), Camera::fit(skill_id, timeline));
            *playback = Some((skill_id.to_owned(), start, camera));
            (start, camera)
        }
    }
}

/// The frame on screen (past the timeline during the pause) and how long it stays there.
fn tick(timeline: &Timeline, start: Instant) -> (usize, Duration) {
    let length = timeline.frames.len() + LOOP_PAUSE;
    let tick = (start.elapsed().as_secs_f64() * FPS) as usize % length;
    let wait = if tick < timeline.frames.len() {
        REDRAW
    } else {
        Duration::from_secs_f64((length - tick) as f64 / FPS)
    };
    (tick, wait)
}

/// The preview block, for skills whose motion was read from the game.
pub(crate) fn block(skill: &SkillSpec, cx: &App) -> Option<Div> {
    let timeline = skill_sim::timeline(&skill.id)?;
    let (start, camera) = playback(&skill.id, &timeline);
    let (tick, wait) = tick(&timeline, start);
    let p = cx.global::<TooltipTheme>();
    let colors = Colors::new(p);
    let palette = Palette {
        active: p.positive.opacity(0.35),
        edge: p.positive.opacity(0.8),
        idle: p.text.opacity(0.2),
        dummy: p.stat_orange.opacity(0.7),
        hit: p.accent_hot,
        player: p.stat_blue.opacity(0.9),
    };
    let summary = trf(
        "planner.skills.preview_hits",
        &[
            ("hits", timeline.hits.len().to_string()),
            ("distance", format!("{:.0}", timeline.dummy[0])),
        ],
    );
    let view = div()
        .relative()
        .w_full()
        .aspect_ratio(VIEW_ASPECT)
        .overflow_hidden()
        .bg(colors.view)
        .children(floor(camera))
        .child(div().absolute().inset_0().bg(colors.dim))
        .child(dummy_sprite(&timeline, tick, camera))
        .children(sprites(&skill.id, &timeline, tick, camera));
    let skill_id = skill.id.clone();
    Some(
        detail_block(
            "skill-preview",
            tr("planner.skills.preview"),
            None,
            None,
            cx,
        )
        .child(
            view.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, (), window, cx| {
                        let frame = (tick, camera);
                        paint(bounds, &timeline, &skill_id, frame, colors, palette, window);
                        // In the background the loop rests; activating the window redraws it.
                        if window.is_window_active() {
                            redraw_after(wait, window, cx);
                        }
                    },
                )
                .absolute()
                .inset_0()
                .size_full(),
            ),
        )
        .child(
            div()
                .mt_1p5()
                .text_size(units(11.))
                .text_color(p.muted)
                .child(summary),
        ),
    )
}

/// The Act 1 town floor tiled under the whole view, one tile placed as in the base view.
fn floor(camera: Camera) -> impl Iterator<Item = Img> {
    let (image, [w, h]) = skill_sprites::ground();
    let [x0, y0] = [-MARGIN, -h / 2.];
    let span = |from: f32, length: f32, origin: f32, size: f32| {
        ((from - origin) / size).floor() as i32..=((from + length - origin) / size).floor() as i32
    };
    let rows = span(camera.top, camera.height(), y0, h);
    span(camera.left, camera.width, x0, w)
        .flat_map(move |i| rows.clone().map(move |j| (i, j)))
        .map(move |(i, j)| {
            let corner = [x0 + i as f32 * w, y0 + j as f32 * h];
            // One px of overlap hides the seam between tiles.
            camera.place(img(image.clone()), corner, [w + 1., h + 1.])
        })
}

/// When the dummy's hit animation playing at `tick` started: a hit during it changes nothing.
fn reaction(hits: &[u32], tick: usize, length: usize) -> Option<usize> {
    let mut start: Option<usize> = None;
    for hit in hits
        .iter()
        .map(|&hit| hit as usize)
        .take_while(|&hit| hit <= tick)
    {
        if start.is_none_or(|start| hit >= start + length) {
            start = Some(hit);
        }
    }
    start.filter(|&start| tick < start + length)
}

// ponytail: the game picks one of five hit animations at random; the preview always plays one.
fn dummy_sprite(timeline: &Timeline, tick: usize, camera: Camera) -> Img {
    let look = skill_sprites::dummy();
    let hit = skill_sprites::frames(&look.hit, None, false);
    let length = (hit.images.len() as f32 / look.speed).round() as usize;
    let (frames, frame) = match reaction(&timeline.hits, tick, length) {
        Some(start) => (hit, ((tick - start) as f32 * look.speed) as usize),
        None => (skill_sprites::frames(&look.idle, None, false), 0),
    };
    let image = frames.images[frame.min(frames.images.len() - 1)].clone();
    let [x, y] = timeline.dummy;
    camera.place(
        img(image),
        [x - frames.origin[0], y - frames.origin[1]],
        frames.size,
    )
}

/// The game's sprites of every object on screen in this frame.
fn sprites<'a>(
    skill_id: &'a str,
    timeline: &'a Timeline,
    tick: usize,
    camera: Camera,
) -> impl Iterator<Item = Img> + 'a {
    timeline
        .frames
        .get(tick)
        .into_iter()
        .flatten()
        .flat_map(move |shape| {
            skill_sprites::layers(skill_id, &shape.object.name)
                .into_iter()
                .flatten()
                .filter_map(move |layer| sprite(shape, layer, camera))
        })
}

// ponytail: sprites are drawn unturned, as the masks are; the replay aims along angle 0.
fn sprite(shape: &Shape, layer: &'static Layer, camera: Camera) -> Option<Img> {
    if shape.age < layer.from || layer.until.is_some_and(|until| shape.age >= until) {
        return None;
    }
    let frames = skill_sprites::frames(&layer.sprite, layer.tint, layer.add);
    let (corner, size, alpha) = placed(shape, layer, &frames, shape.height)?;
    let shown = shape.age.saturating_sub(layer.from.max(1));
    let frame = (shown as f32 * layer.speed) as usize % frames.images.len();
    Some(camera.place(
        img(frames.images[frame].clone()).opacity(alpha.min(1.)),
        corner,
        size,
    ))
}

/// Where a layer draws an object `height` px above the ground: top-left corner, size and alpha.
fn placed(
    shape: &Shape,
    layer: &Layer,
    frames: &Frames,
    height: f32,
) -> Option<([f32; 2], [f32; 2], f32)> {
    let scale: [f32; 2] =
        std::array::from_fn(|i| (shape.scale[i] + height * layer.height_scale) * layer.scale[i]);
    let fade = if layer.fades { 1. - shape.fade } else { 1. };
    let alpha = (layer.alpha + height * layer.height_alpha) * fade;
    if scale[0] <= 0. || scale[1] <= 0. || alpha <= 0. {
        return None;
    }
    let lift = if layer.ground {
        0.
    } else {
        height + layer.lift
    };
    let x = shape.pos[0] + if layer.slant { height } else { 0. };
    let y = shape.pos[1] - lift;
    Some((
        [
            x - frames.origin[0] * scale[0],
            y - frames.origin[1] * scale[1],
        ],
        [frames.size[0] * scale[0], frames.size[1] * scale[1]],
        alpha,
    ))
}

/// Game-px rectangles an object covers on the ground: its hitbox and its sprites, height left out.
fn drawn<'a>(skill_id: &'a str, shape: &'a Shape) -> impl Iterator<Item = [f32; 4]> + 'a {
    let sprites = skill_sprites::layers(skill_id, &shape.object.name)
        .into_iter()
        .flatten()
        .filter_map(move |layer| {
            let frames = skill_sprites::frames(&layer.sprite, layer.tint, layer.add);
            let ([x0, y0], [w, h], _) = placed(shape, layer, &frames, 0.)?;
            Some([x0, y0, x0 + w, y0 + h])
        });
    hitbox_rect(shape).into_iter().chain(sprites)
}

/// Game-px edges of an object's mask or box, unturned as the preview draws it.
fn hitbox_rect(shape: &Shape) -> Option<[f32; 4]> {
    let object = shape.object;
    let [x0, y0, x1, y1] = match (object.mask.as_deref().and_then(hitbox::mask), object.bounds) {
        (Some(mask), _) => {
            let width = mask.rows.iter().flatten().copied().max().unwrap_or(0) as f32;
            let [x, y] = object.at;
            [x, y, x + width, y + mask.rows.len() as f32]
        }
        (None, Some([x0, y0, x1, y1])) => [x0, y0, x1 + 1., y1 + 1.],
        (None, None) => return None,
    };
    let [x, y] = shape.pos;
    let edge = |v: f32, axis: usize| [x, y][axis] + (v - object.origin[axis]) * shape.scale[axis];
    Some([edge(x0, 0), edge(y0, 1), edge(x1, 0), edge(y1, 1)])
}

fn paint(
    bounds: Bounds<Pixels>,
    timeline: &Timeline,
    skill_id: &str,
    (tick, camera): (usize, Camera),
    colors: Colors,
    palette: Palette,
    window: &mut Window,
) {
    let zoom = f32::from(bounds.size.width) / camera.width;
    let left = f32::from(bounds.origin.x) - camera.left * zoom;
    let top = f32::from(bounds.origin.y) - camera.top * zoom;
    let device = window.scale_factor();
    let at = |[x, y]: [f32; 2]| View {
        center: [left + x * zoom, top + y * zoom],
        zoom,
        device,
    };
    let lit = timeline
        .hits
        .iter()
        .any(|&hit| (hit as usize..hit as usize + HIT_FLASH).contains(&tick));
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        area_diagram::paint_grid(window, bounds, &at([0., 0.]), colors);
        let player = hitbox::player();
        if let Some(mask) = hitbox::mask(&player.mask) {
            area_diagram::paint_mask(
                window,
                &at([0., 0.]),
                mask,
                [0., 0.],
                player.origin,
                [1., 1.],
                palette.player,
            );
        }
        let (ring, width) = if lit {
            (palette.hit, 2.)
        } else {
            (palette.dummy, 1.)
        };
        paint_ring(window, &at(timeline.dummy), ring, width);
        for shape in timeline.frames.get(tick).into_iter().flatten() {
            if skill_sprites::layers(skill_id, &shape.object.name).is_some() {
                // The sprite shows the object; the hitbox shows only while it can hit.
                if shape.active {
                    paint_hitbox(window, &at(shape.pos), shape, palette);
                }
                continue;
            }
            let color = if shape.active {
                palette.active
            } else {
                palette.idle
            };
            if shape.height > 0. {
                // The airborne object; collisions stay on the ground below it.
                let lifted = [shape.pos[0], shape.pos[1] - shape.height];
                paint_shape(window, &at(lifted), shape, palette.idle);
            }
            paint_shape(window, &at(shape.pos), shape, color);
        }
    });
    window.paint_quad(outline(bounds, colors.frame, BorderStyle::Solid));
}

/// Draws the window again after `delay`; frames painted meanwhile ride on the same redraw.
fn redraw_after(delay: Duration, window: &Window, cx: &App) {
    static PENDING: AtomicBool = AtomicBool::new(false);
    if PENDING.swap(true, Ordering::AcqRel) {
        return;
    }
    window
        .spawn(cx, async move |cx| {
            cx.background_executor().timer(delay).await;
            PENDING.store(false, Ordering::Release);
            let _ = cx.update(|window, _| window.refresh());
        })
        .detach();
}

/// Over a sprite: a mask lightly filled, a box only edged, since it would cover the whole sprite.
fn paint_hitbox(window: &mut Window, view: &View, shape: &Shape, palette: Palette) {
    match box_rect(view, shape) {
        Some([x0, y0, x1, y1]) if shape.object.mask.is_none() => window.paint_quad(outline(
            Bounds::from_corners(point(px(x0), px(y0)), point(px(x1), px(y1))),
            palette.edge,
            BorderStyle::Solid,
        )),
        _ => paint_shape(window, view, shape, palette.active),
    }
}

fn paint_ring(window: &mut Window, view: &View, color: Hsla, width: f32) {
    let r = DUMMY_RADIUS * view.zoom;
    let [x, y] = view.center;
    window.paint_quad(quad(
        Bounds::from_corners(point(px(x - r), px(y - r)), point(px(x + r), px(y + r))),
        px(r),
        transparent_black(),
        px(width),
        color,
        BorderStyle::Solid,
    ));
}

// ponytail: masks are drawn unturned; aimed objects fly at angle 0, fans tilt only a few degrees.
fn paint_shape(window: &mut Window, view: &View, shape: &Shape, color: Hsla) {
    let object = shape.object;
    if let Some(mask) = object.mask.as_deref().and_then(hitbox::mask) {
        area_diagram::paint_mask(
            window,
            view,
            mask,
            object.at,
            object.origin,
            shape.scale,
            color,
        );
    } else if let Some(rect) = box_rect(view, shape) {
        area_diagram::paint_rect(window, rect, color);
    }
}

/// Screen edges of an object's box hitbox.
fn box_rect(view: &View, shape: &Shape) -> Option<[f32; 4]> {
    let object = shape.object;
    let [x0, y0, x1, y1] = object.bounds?;
    let x = |v: f32| view.center[0] + (v - object.origin[0]) * shape.scale[0] * view.zoom;
    let y = |v: f32| view.center[1] + (v - object.origin[1]) * shape.scale[1] * view.zoom;
    Some([x(x0), y(y0), x(x1 + 1.), y(y1 + 1.)])
}

#[cfg(test)]
mod tests {
    use super::{Camera, KEEP, MARGIN, MAX_ZOOM_OUT, skill_sim};

    #[test]
    fn the_view_widens_to_the_skill_and_keeps_the_player_and_the_dummy() {
        let widths = ["volcano", "armageddon"].map(|skill| {
            let timeline = skill_sim::timeline(skill).expect(skill);
            let camera = Camera::fit(skill, &timeline);
            let dummy = timeline.dummy[0];
            let base = dummy + 2. * MARGIN;
            assert!(
                (base..=base * MAX_ZOOM_OUT).contains(&camera.width),
                "{skill}"
            );
            assert!(camera.left <= -KEEP[0], "{skill}");
            assert!(camera.left + camera.width >= dummy + KEEP[0], "{skill}");
            camera.width / base
        });
        // Volcano fits the base view; Armageddon's meteors land up to 340 px from the cursor.
        assert_eq!(widths[0], 1.);
        assert!(widths[1] > 1.5, "{widths:?}");
    }
}
