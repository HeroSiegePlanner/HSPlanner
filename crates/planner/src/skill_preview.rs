//! The skills panel PREVIEW: one use replayed frame by frame against the training dummy, in a loop.
use crate::area_diagram::{self, Colors, TOWN, VIEW_ASPECT, VIEW_WIDTH, View};
use crate::hitbox;
use crate::skill_details::{DetailsContext, detail_block, units};
use crate::skill_mods::{self, Hit, Mods};
use crate::skill_overlay::{self, Kind, Rising};
use crate::skill_sim::{self, Shape, Timeline};
use crate::skill_sprites::{self, Frames, Layer};
use gpui_kit::{prelude::*, *};
use hsplanner_ui::i18n::{tr, trf};
use hsplanner_ui::theme::TooltipTheme;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

const FPS: f64 = 60.;
/// Frames of an empty town before the use plays again.
const LOOP_PAUSE: usize = 30;
/// Time between two drawn frames: every second game frame.
const REDRAW: Duration = Duration::from_millis(33);
/// Frames the dummy's collision ring stays lit after a hit.
const HIT_FLASH: usize = 8;
/// The dummy's collision circle, as the dps cards count it.
const DUMMY_RADIUS: f32 = 12.;
/// Ice the frostbitten dummy is tinted with.
const FROST_TINT: [u8; 3] = [150, 205, 255];
/// Game px above the player at which its buff shows.
const BUFF_ABOVE: f32 = 66.;
/// Fewest frames between two uses, however fast the game lets the skill be used.
const MIN_PERIOD: f64 = 6.;
/// Uses on screen and when their loop started.
type Playing = (Arc<[Timeline]>, Instant);
static PLAYBACK: Mutex<Option<Playing>> = Mutex::new(None);

#[derive(Clone, Copy)]
struct Palette {
    active: Hsla,
    edge: Hsla,
    idle: Hsla,
    dummy: Hsla,
    hit: Hsla,
    buffed: Hsla,
}

/// The game-px rectangle the view shows, `VIEW_ASPECT` wide to tall.
#[derive(Clone, Copy)]
struct Camera {
    left: f32,
    top: f32,
    width: f32,
}

impl Camera {
    /// The game's view at field of view 1.0 around the player, as the town screenshot shows it.
    const GAME: Self = Self {
        left: -VIEW_WIDTH / 2.,
        top: -VIEW_WIDTH / VIEW_ASPECT / 2.,
        width: VIEW_WIDTH,
    };

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

/// When the loop of these uses started; another skill or other modifiers start anew.
fn playback(uses: &Arc<[Timeline]>) -> Instant {
    let mut playback = PLAYBACK.lock().unwrap_or_else(PoisonError::into_inner);
    match &*playback {
        Some((shown, start)) if Arc::ptr_eq(shown, uses) => *start,
        _ => {
            let start = Instant::now();
            *playback = Some((uses.clone(), start));
            start
        }
    }
}

/// Frames between two uses: an aura's pulse timer, whose shadow lasts one pulse; the game's use
/// rate for a skill whose modifiers are modelled (its cooldown and cast speed), never under
/// `MIN_PERIOD`; else a use and a short pause.
fn period(details: &DetailsContext, mods: &Mods, uses: &[Timeline]) -> usize {
    if let Some(frames) = skill_mods::pulse_frames(&details.skill.id, mods) {
        return frames;
    }
    let longest = uses.iter().map(|use_| use_.frames.len()).max().unwrap_or(0);
    let ranks = &details.snapshot.subskill_ranks;
    let skill = &*hsplanner_engine::calc::subskill::learned_use_period(details.skill, ranks);
    let rate = details
        .performance
        .filter(|_| skill_mods::models(&skill.id))
        .and_then(|result| {
            let scoped = result.computed.skill_scoped.get(&skill.id)?;
            skill_overlay::use_rate(skill, &result.computed.stats, scoped)
        });
    match rate {
        Some((low, high)) if low + high > 0. => {
            (2. * FPS / (low + high)).round().max(MIN_PERIOD) as usize
        }
        _ => longest + LOOP_PAUSE,
    }
}

/// A modelled skill's average hit, an attack's full one, without the planner damage the preview
/// draws itself.
fn hit(details: &DetailsContext) -> Option<Hit> {
    let skill = &details.skill.id;
    let result = details.performance.filter(|_| skill_mods::models(skill))?;
    let entry = result
        .per_skill
        .iter()
        .find(|entry| entry.skill_id == *skill)?;
    let drawn = skill_mods::drawn_damage(skill, &Mods::of(skill, &details.snapshot.subskill_ranks));
    let performance = &entry.performance;
    Hit::of(
        performance.damage.as_ref(),
        performance.attack_damage.as_ref(),
        drawn,
    )
}

/// The skill's average hit and the player's maximum mana, for the numbers over a modelled skill.
fn magnitudes(details: &DetailsContext) -> Option<(f64, f64)> {
    let hit = hit(details)?;
    let mana = details.performance?.computed.stats.get("mana");
    Some((hit.damage, mana.map_or(0., |mana| mana.1)))
}

/// What plays now: the uses, which of them are on screen at which frame, the frames since the
/// loop started, the view and when to draw again.
struct Now {
    uses: Arc<[Timeline]>,
    shown: Vec<(usize, usize)>,
    clock: usize,
    camera: Camera,
    wait: Duration,
}

/// Classes whose skills show a Preview: every subskill of theirs is read from the game.
const CLASSES: [&str; 2] = ["exo", "pyromancer"];

/// Whether skills of `class_id` show a Preview.
fn previews(class_id: &str) -> bool {
    CLASSES.contains(&class_id)
}

impl Now {
    fn of(details: &DetailsContext) -> Option<Self> {
        let skill = details.skill;
        previews(&skill.class_id).then_some(())?;
        let computed = details.performance.map(|result| &result.computed);
        let mut mods = Mods::for_build(&skill.id, details.snapshot, computed);
        // A NaN would miss the timeline cache on every frame and restart the loop.
        let finite = |hit: &Hit| hit.elemental.is_finite() && hit.weapon_bonus.is_finite();
        if let Some(hit) = hit(details).filter(finite) {
            mods.elemental = hit.elemental;
            mods.weapon_bonus = hit.weapon_bonus;
        }
        let uses = skill_sim::uses(&skill.id, &mods)?;
        let start = playback(&uses);
        let period = period(details, &mods, &uses);
        let lengths: Vec<usize> = uses.iter().map(|use_| use_.frames.len()).collect();
        let t = (start.elapsed().as_secs_f64() * FPS) as usize;
        let shown = skill_overlay::visible(&lengths, period, t);
        let wait = if shown.is_empty() {
            Duration::from_secs_f64(skill_overlay::until_next(period, t) as f64 / FPS)
        } else {
            REDRAW
        };
        Some(Self {
            uses,
            shown,
            clock: t,
            camera: Camera::GAME,
            wait,
        })
    }

    fn moment(&self) -> Vec<(&Timeline, usize)> {
        self.shown
            .iter()
            .map(|&(k, frame)| (&self.uses[k % self.uses.len()], frame))
            .collect()
    }
}

fn palette(p: &TooltipTheme) -> Palette {
    Palette {
        active: p.positive.opacity(0.35),
        edge: p.positive.opacity(0.8),
        idle: p.text.opacity(0.2),
        dummy: p.stat_orange.opacity(0.7),
        hit: p.accent_hot,
        buffed: p.positive,
    }
}

/// The numbers rising over the dummy and the player, and the player's buff.
fn overlays(
    details: &DetailsContext,
    moment: &[(&Timeline, usize)],
    camera: Camera,
    p: &TooltipTheme,
) -> Vec<Div> {
    let color = |kind: Kind| match kind {
        Kind::Hit => p.text,
        Kind::Frost => p.stat_blue,
        Kind::Mana => p.accent,
    };
    let numbers = magnitudes(details)
        .map(|(hit, mana)| skill_overlay::rising(moment, hit, mana))
        .unwrap_or_default();
    let buff = skill_overlay::buff(moment).map(|percent| Rising {
        at: [0., -BUFF_ABOVE],
        text: format!("+{percent:.0}%"),
        gone: 0.,
        kind: Kind::Hit,
    });
    numbers
        .iter()
        .map(|number| label(number, camera, color(number.kind)))
        .chain(buff.map(|buff| label(&buff, camera, p.positive)))
        .collect()
}

/// The preview block, for skills whose motion was read from the game, with the build's subskills.
pub(crate) fn block(details: &DetailsContext, cx: &App) -> Option<Div> {
    let now = Now::of(details)?;
    let camera = now.camera;
    let p = cx.global::<TooltipTheme>();
    let (colors, palette) = (Colors::new(p), palette(p));
    let moment = now.moment();
    let summary = trf(
        "planner.skills.preview_hits",
        &[
            ("hits", now.uses[0].hits.len().to_string()),
            ("distance", format!("{:.0}", now.uses[0].dummy[0])),
        ],
    );
    let view = div()
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
        .child(div().absolute().inset_0().bg(colors.dim))
        .children(dummy_sprite(&moment, camera))
        .children(sprites(&details.skill.id, &moment, camera, now.clock))
        .children(overlays(details, &moment, camera, p));
    let buffed = skill_overlay::buff(&moment).is_some();
    let pulls = skill_overlay::pulls(&moment);
    let skill_id = details.skill.id.clone();
    let canvas = canvas(
        |_, _, _| (),
        move |bounds, (), window, cx| {
            let scene = Scene {
                moment: &now.moment(),
                skill_id: &skill_id,
                camera,
                buffed,
                pulls: &pulls,
            };
            paint(bounds, &scene, colors, palette, window);
            // In the background the loop rests; activating the window redraws it.
            if window.is_window_active() {
                redraw_after(now.wait, window, cx);
            }
        },
    );
    Some(
        detail_block(
            "skill-preview",
            tr("planner.skills.preview"),
            None,
            None,
            cx,
        )
        .child(view.child(canvas.absolute().inset_0().size_full()))
        .child(
            div()
                .mt_1p5()
                .text_size(units(11.))
                .text_color(p.muted)
                .child(summary),
        ),
    )
}

/// A number placed on the view at its game-px point, fading as it rises.
fn label(number: &Rising, camera: Camera, color: Hsla) -> Div {
    let [x, y] = number.at;
    div()
        .absolute()
        .left(relative((x - camera.left) / camera.width))
        .top(relative((y - camera.top) / camera.height()))
        .ml(units(-8.))
        .text_size(units(10.))
        .font_weight(FontWeight::BOLD)
        .text_color(color.opacity(1. - number.gone))
        .child(number.text.clone())
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
fn dummy_sprite(moment: &[(&Timeline, usize)], camera: Camera) -> Option<Img> {
    let dummy = moment.first().map(|(use_, _)| use_.dummy)?;
    let look = skill_sprites::dummy();
    let tint = skill_overlay::frostbitten(moment).then_some(FROST_TINT);
    let hit = skill_sprites::frames(&look.hit, tint, false);
    let length = (hit.images.len() as f32 / look.speed).round() as usize;
    let reacting = moment
        .iter()
        .filter_map(|&(use_, frame)| Some(frame - reaction(&use_.hits, frame, length)?))
        .min();
    let (frames, frame) = match reacting {
        Some(since) => (hit, (since as f32 * look.speed) as usize),
        None => (skill_sprites::frames(&look.idle, tint, false), 0),
    };
    let image = frames.images[frame.min(frames.images.len() - 1)].clone();
    let [x, y] = dummy;
    Some(camera.place(
        img(image),
        [x - frames.origin[0], y - frames.origin[1]],
        frames.size,
    ))
}

/// The game's sprites of every object on screen in this frame, `clock` frames into the loop.
fn sprites<'a>(
    skill_id: &'a str,
    moment: &'a [(&'a Timeline, usize)],
    camera: Camera,
    clock: usize,
) -> impl Iterator<Item = Img> + 'a {
    moment
        .iter()
        .flat_map(|&(use_, frame)| use_.frames.get(frame).into_iter().flatten())
        .flat_map(move |shape| {
            skill_sprites::layers(skill_id, &shape.object.name)
                .into_iter()
                .flatten()
                .filter_map(move |layer| sprite(shape, layer, camera, clock))
        })
}

// ponytail: sprites are drawn unturned unless their layer turns (a flying flare); masks stay unturned.
fn sprite(shape: &Shape, layer: &'static Layer, camera: Camera, clock: usize) -> Option<Img> {
    if shape.age < layer.from || layer.until.is_some_and(|until| shape.age >= until) {
        return None;
    }
    let (frames, frame) = layer_frames(shape, layer, clock);
    let (corner, size, alpha) = placed(shape, layer, &frames, shape.height)?;
    Some(camera.place(
        img(frames.images[frame].clone()).opacity(alpha.min(1.)),
        corner,
        size,
    ))
}

/// A layer's frames and the one it shows `clock` frames into the loop: bent round its circle at
/// the object's scale, spun with the object's uneven scale, or turned with the object when the
/// layer turns.
fn layer_frames(shape: &Shape, layer: &'static Layer, clock: usize) -> (Arc<Frames>, usize) {
    let (sprite, tint, add) = (layer.sprite.as_str(), layer.tint, layer.add);
    let frame = layer.frame(shape.age, clock, skill_sprites::count(sprite));
    if let Some(circle) = layer.circle {
        let band = circle.at(shape.scale[0]);
        return (skill_sprites::ring(sprite, tint, add, band), frame);
    }
    if layer.spins(shape.scale) {
        let scale = layer.spun_scale(shape.scale, shape.angle);
        let spun = skill_sprites::spun(sprite, tint, add, shape.angle, scale, frame);
        return (spun, 0);
    }
    let angle = if layer.turns { shape.angle } else { 0. };
    (skill_sprites::turned(sprite, tint, add, angle), frame)
}

/// Where a layer draws an object `height` px above the ground: top-left corner, size and alpha.
fn placed(
    shape: &Shape,
    layer: &Layer,
    frames: &Frames,
    height: f32,
) -> Option<([f32; 2], [f32; 2], f32)> {
    let object = layer.object_scale(shape.scale);
    let scale: [f32; 2] =
        std::array::from_fn(|i| (object[i] + height * layer.height_scale) * layer.scale[i]);
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

/// What one painted frame shows.
struct Scene<'a> {
    moment: &'a [(&'a Timeline, usize)],
    skill_id: &'a str,
    camera: Camera,
    buffed: bool,
    pulls: &'a [[f32; 2]],
}

fn paint(
    bounds: Bounds<Pixels>,
    scene: &Scene,
    colors: Colors,
    palette: Palette,
    window: &mut Window,
) {
    let camera = scene.camera;
    let zoom = f32::from(bounds.size.width) / camera.width;
    let left = f32::from(bounds.origin.x) - camera.left * zoom;
    let top = f32::from(bounds.origin.y) - camera.top * zoom;
    let device = window.scale_factor();
    let at = |[x, y]: [f32; 2]| View {
        center: [left + x * zoom, top + y * zoom],
        zoom,
        device,
    };
    let lit = scene.moment.iter().any(|&(use_, frame)| {
        use_.hits
            .iter()
            .any(|&hit| (hit as usize..hit as usize + HIT_FLASH).contains(&frame))
    });
    let dummy = scene
        .moment
        .first()
        .map_or([0., 0.], |(use_, _)| use_.dummy);
    window.with_content_mask(Some(ContentMask { bounds }), |window| {
        area_diagram::paint_grid(window, bounds, &at([0., 0.]), colors);
        // The town screenshot shows the player; its outline marks a buff.
        let player = hitbox::player();
        if let Some(mask) = hitbox::mask(&player.mask).filter(|_| scene.buffed) {
            area_diagram::paint_mask(
                window,
                &at([0., 0.]),
                mask,
                [0., 0.],
                player.origin,
                [1., 1.],
                palette.buffed,
            );
        }
        let (ring, width) = if lit {
            (palette.hit, 2.)
        } else {
            (palette.dummy, 1.)
        };
        paint_ring(window, &at(dummy), ring, width);
        for &toward in scene.pulls {
            paint_line(window, at(dummy).center, at(toward).center, palette.edge);
        }
        let shapes = scene
            .moment
            .iter()
            .flat_map(|&(use_, frame)| use_.frames.get(frame).into_iter().flatten());
        for shape in shapes {
            paint_object(window, &at(shape.pos), shape, scene.skill_id, palette, &at);
        }
    });
    window.paint_quad(outline(bounds, colors.frame, BorderStyle::Solid));
}

/// One object: the hitbox over its sprite while it can hit, or its shape where it has no sprite.
fn paint_object(
    window: &mut Window,
    view: &View,
    shape: &Shape,
    skill_id: &str,
    palette: Palette,
    at: &impl Fn([f32; 2]) -> View,
) {
    if skill_sprites::layers(skill_id, &shape.object.name).is_some() {
        if shape.active {
            paint_hitbox(window, view, shape, palette);
        }
        return;
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
    paint_shape(window, view, shape, color);
}

/// A thin line between two screen points, such as a pull toward the hole.
fn paint_line(window: &mut Window, from: [f32; 2], to: [f32; 2], color: Hsla) {
    let mut path = PathBuilder::stroke(px(1.));
    path.move_to(point(px(from[0]), px(from[1])));
    path.line_to(point(px(to[0]), px(to[1])));
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
    }
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
    if shape.angle != 0. && shape.object.mask.is_none() {
        return paint_turned_box(window, view, shape, palette.edge);
    }
    match box_rect(view, shape) {
        Some([x0, y0, x1, y1]) if shape.object.mask.is_none() => window.paint_quad(outline(
            Bounds::from_corners(point(px(x0), px(y0)), point(px(x1), px(y1))),
            palette.edge,
            BorderStyle::Solid,
        )),
        _ => paint_shape(window, view, shape, palette.active),
    }
}

/// A turned box's edge: its corners turned about the object's origin, as the collision turns them.
fn paint_turned_box(window: &mut Window, view: &View, shape: &Shape, color: Hsla) {
    let object = shape.object;
    let Some([x0, y0, x1, y1]) = object.bounds else {
        return;
    };
    let (sin, cos) = shape.angle.to_radians().sin_cos();
    let [cx, cy] = view.center;
    let corner = |[x, y]: [f32; 2]| {
        let dx = (x - object.origin[0]) * shape.scale[0];
        let dy = (y - object.origin[1]) * shape.scale[1];
        point(
            px(cx + (dx * cos + dy * sin) * view.zoom),
            px(cy + (dy * cos - dx * sin) * view.zoom),
        )
    };
    let mut path = PathBuilder::stroke(px(1.));
    path.move_to(corner([x0, y0]));
    for at in [[x1 + 1., y0], [x1 + 1., y1 + 1.], [x0, y1 + 1.]] {
        path.line_to(corner(at));
    }
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, color);
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

// ponytail: masks and unsprited boxes are drawn unturned; only a sprited box (the flare) turns.
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
    #[test]
    fn only_exo_and_pyromancer_skills_show_a_preview() {
        assert!(super::previews("exo") && super::previews("pyromancer"));
        assert!(!super::previews("stormweaver"));
    }
}
