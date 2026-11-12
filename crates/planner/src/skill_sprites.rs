//! The game's sprites for the skill preview: data/skill-sprites.json and assets/skill-fx.
use crate::scene::render_image;
use gpui_kit::RenderImage;
use image::{Rgba, RgbaImage};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

include!(concat!(env!("OUT_DIR"), "/skill-sprites.rs"));

/// Turned frames are baked and kept once per this many degrees.
const TURN_STEP: f32 = 5.;
/// Spun frames are baked one at a time, per half degree, at half size, and forgotten all at
/// once past `MAX_SPUN`: a spinning whip would otherwise keep hundreds of large frames.
const SPIN_STEP: f32 = 0.5;
const SPIN_RESOLUTION: f32 = 0.5;
const MAX_SPUN: usize = 1024;

#[derive(Deserialize)]
struct SpritesFile {
    dummy: DummyLook,
    origins: HashMap<String, [f32; 2]>,
    skills: HashMap<String, HashMap<String, Vec<Layer>>>,
}

#[derive(Deserialize)]
pub(crate) struct DummyLook {
    pub idle: String,
    pub hit: String,
    /// Sprite frames per game frame of the hit animation.
    pub speed: f32,
}

/// One sprite an object draws, as its Draw event does.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Layer {
    pub sprite: String,
    /// Sprite frames per game frame.
    #[serde(default)]
    pub speed: f32,
    /// Drawn this far above the object.
    #[serde(default)]
    pub lift: f32,
    /// Stays on the ground under a falling object: its shadow.
    #[serde(default)]
    pub ground: bool,
    /// Drawn right of the object by its height, for a diagonal fall.
    #[serde(default)]
    pub slant: bool,
    /// Added to the scale per px of height.
    #[serde(default)]
    pub height_scale: f32,
    /// Added to the alpha per px of height.
    #[serde(default)]
    pub height_alpha: f32,
    #[serde(default = "unscaled")]
    pub scale: [f32; 2],
    #[serde(default = "opaque")]
    pub alpha: f32,
    pub tint: Option<[u8; 3]>,
    /// Drawn with additive blending (bm_add).
    #[serde(default)]
    pub add: bool,
    /// Fades out together with the object.
    #[serde(default)]
    pub fades: bool,
    /// Object age at which this layer starts and plays from its first frame.
    #[serde(default)]
    pub from: u32,
    /// The frame it shows at `from`, for an animation the object drives itself.
    #[serde(default)]
    pub first: f32,
    /// Object age at which another layer takes over.
    pub until: Option<u32>,
    /// Drawn turned with its object, as a flying flare points where it flies.
    #[serde(default)]
    pub turns: bool,
    /// Mirrored top to bottom while its object points left, between 90° and 270°.
    #[serde(default)]
    pub flips: bool,
    /// Bent round a circle about its object, as draw_sprite_circle bends it.
    pub circle: Option<Circle>,
    /// Plays with the loop instead of its object, whose game instance never starts again.
    #[serde(default)]
    pub steady: bool,
    /// Keeps its own size whatever its object's scale.
    #[serde(default)]
    pub fixed: bool,
}

impl Layer {
    /// The frame, of `count`, an object `age` frames old shows `clock` frames into the loop.
    pub(crate) fn frame(&self, age: u32, clock: usize, count: usize) -> usize {
        let shown = if self.steady {
            clock
        } else {
            age.saturating_sub(self.from.max(1)) as usize
        };
        (self.first + shown as f32 * self.speed) as usize % count.max(1)
    }

    /// Whether its object's scale goes in before the turn, as GameMaker scales a sprite along
    /// its own axes: an uneven or mirrored turned sprite.
    pub(crate) fn spins(&self, scale: [f32; 2]) -> bool {
        self.turns && (self.flips || scale[0] != scale[1])
    }

    /// The scale it spins an object of `scale` with while the object points at `degrees`.
    pub(crate) fn spun_scale(&self, [x, y]: [f32; 2], degrees: f32) -> [f32; 2] {
        let angle = degrees.rem_euclid(360.);
        let left = angle > 90. && angle < 270.;
        [x, if self.flips && left { -y } else { y }]
    }

    /// The scale it draws an object of `scale` at; a bent or spun layer carries that scale in
    /// its frames.
    pub(crate) fn object_scale(&self, scale: [f32; 2]) -> [f32; 2] {
        if self.fixed || self.circle.is_some() || self.spins(scale) {
            [1.; 2]
        } else {
            scale
        }
    }
}

/// A sprite's width once round a circle and its height across the band.
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Circle {
    /// Game px from the object to the middle of the band.
    pub radius: f32,
    /// Game px across the band.
    pub width: f32,
}

impl Circle {
    /// Radius and width round an object of `scale`, floored as LoadAuraTimer floors them.
    pub(crate) fn at(self, scale: f32) -> [f32; 2] {
        [(self.radius * scale).floor(), (self.width * scale).floor()]
    }
}

fn unscaled() -> [f32; 2] {
    [1., 1.]
}

fn opaque() -> f32 {
    1.
}

/// A sprite's frames as one layer draws them.
pub(crate) struct Frames {
    pub images: Vec<Arc<RenderImage>>,
    /// Frame size in game px.
    pub size: [f32; 2],
    pub origin: [f32; 2],
}

/// Sprite, tint, additive blend and the whole degrees it is turned.
type Look = (&'static str, Option<[u8; 3]>, bool, i32);

static FILE: LazyLock<SpritesFile> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../data/skill-sprites.json"))
        .expect("embedded skill sprites must be valid")
});

static FRAMES: LazyLock<Mutex<HashMap<Look, Arc<Frames>>>> = LazyLock::new(Default::default);

/// Sprite, tint, additive blend and the whole radius and width it is bent to.
type Bent = (&'static str, Option<[u8; 3]>, bool, [u32; 2]);

static RINGS: LazyLock<Mutex<HashMap<Bent, Arc<Frames>>>> = LazyLock::new(Default::default);

/// Sprite, tint, additive blend, half degrees, thousandths of scale and frame.
type Spin = (&'static str, Option<[u8; 3]>, bool, i32, [i32; 2], usize);

static SPUN: LazyLock<Mutex<HashMap<Spin, Arc<Frames>>>> = LazyLock::new(Default::default);

/// How a skill's object is drawn, bottom layer first.
pub(crate) fn layers(skill_id: &str, object: &str) -> Option<&'static [Layer]> {
    FILE.skills.get(skill_id)?.get(object).map(Vec::as_slice)
}

pub(crate) fn dummy() -> &'static DummyLook {
    &FILE.dummy
}

/// A sprite's frames with a tint and blend, decoded on first use.
pub(crate) fn frames(sprite: &'static str, tint: Option<[u8; 3]>, add: bool) -> Arc<Frames> {
    turned(sprite, tint, add, 0.)
}

/// The frames turned `degrees` counter-clockwise about their origin, as image_angle turns a sprite.
pub(crate) fn turned(
    sprite: &'static str,
    tint: Option<[u8; 3]>,
    add: bool,
    degrees: f32,
) -> Arc<Frames> {
    let whole = (((degrees / TURN_STEP).round() * TURN_STEP) as i32).rem_euclid(360);
    let mut cache = FRAMES.lock().unwrap_or_else(PoisonError::into_inner);
    cache
        .entry((sprite, tint, add, whole))
        .or_insert_with(|| {
            let origin = FILE.origins[sprite];
            let turn = |image: RgbaImage| match whole {
                0 => (image, origin),
                _ => {
                    let (turned, origin, _) = spin(&image, origin, whole as f32, [1.; 2], 1.);
                    (turned, origin)
                }
            };
            let decoded: Vec<_> = (bundled(sprite).iter())
                .map(|png| turn(decode(png, tint, add)))
                .collect();
            let (first, origin) = &decoded[0];
            let size = [first.width() as f32, first.height() as f32];
            let origin = *origin;
            Arc::new(Frames {
                images: decoded
                    .into_iter()
                    .map(|(image, _)| render_image(image))
                    .collect(),
                size,
                origin,
            })
        })
        .clone()
}

/// The frames bent round a circle of `[radius, width]` game px, centred on their object.
pub(crate) fn ring(
    sprite: &'static str,
    tint: Option<[u8; 3]>,
    add: bool,
    band: [f32; 2],
) -> Arc<Frames> {
    let mut cache = RINGS.lock().unwrap_or_else(PoisonError::into_inner);
    cache
        .entry((sprite, tint, add, band.map(|v| v as u32)))
        .or_insert_with(|| {
            let bent: Vec<_> = (bundled(sprite).iter())
                .map(|png| bend(&decode(png, tint, add), band))
                .collect();
            let side = bent[0].width() as f32;
            Arc::new(Frames {
                images: bent.into_iter().map(render_image).collect(),
                size: [side; 2],
                origin: [side / 2.; 2],
            })
        })
        .clone()
}

/// A frame bent by nearest pixel as draw_sprite_circle bends it: its width once round
/// counter-clockwise from the right, its top row on the inside of a band `width` px across.
fn bend(frame: &RgbaImage, [radius, width]: [f32; 2]) -> RgbaImage {
    let side = 2 * (radius + width / 2.).ceil() as u32 + 2;
    let centre = side as f32 / 2.;
    let (w, h) = frame.dimensions();
    let texel = |share: f32, size: u32| ((share * size as f32) as u32).min(size - 1);
    RgbaImage::from_fn(side, side, |i, j| {
        let (x, y) = (i as f32 + 0.5 - centre, j as f32 + 0.5 - centre);
        let v = (x.hypot(y) - radius) / width + 0.5;
        if !(0. ..1.).contains(&v) {
            return Rgba([0; 4]);
        }
        let u = (-y).atan2(x).to_degrees().rem_euclid(360.) / 360.;
        *frame.get_pixel(texel(u, w), texel(v, h))
    })
}

/// One frame scaled by its object's `scale` along its own axes, then turned `degrees`, cut to
/// what it shows and baked when first drawn.
pub(crate) fn spun(
    sprite: &'static str,
    tint: Option<[u8; 3]>,
    add: bool,
    degrees: f32,
    scale: [f32; 2],
    frame: usize,
) -> Arc<Frames> {
    let steps = (degrees.rem_euclid(360.) / SPIN_STEP).round() as i32;
    let thousandths = scale.map(|s| (s * 1000.).round() as i32);
    let key = (sprite, tint, add, steps, thousandths, frame);
    let mut cache = SPUN.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(spun) = cache.get(&key) {
        return spun.clone();
    }
    if cache.len() >= MAX_SPUN {
        cache.clear();
    }
    let look = (tint, add);
    let spun = Arc::new(bake(sprite, look, steps as f32 * SPIN_STEP, scale, frame));
    cache.insert(key, spun.clone());
    spun
}

/// One frame of `spun`, with its tint and blend.
fn bake(
    sprite: &str,
    (tint, add): (Option<[u8; 3]>, bool),
    degrees: f32,
    scale: [f32; 2],
    frame: usize,
) -> Frames {
    let decoded = decode(bundled(sprite)[frame], tint, add);
    let (image, origin) = cropped(decoded, FILE.origins[sprite]);
    let (image, origin, size) = spin(&image, origin, degrees, scale, SPIN_RESOLUTION);
    let images = vec![render_image(image)];
    Frames {
        images,
        size,
        origin,
    }
}

/// Frames the sprite has.
pub(crate) fn count(sprite: &str) -> usize {
    bundled(sprite).len()
}

/// An image cut to its visible pixels, and where the origin lands in the cut.
fn cropped(image: RgbaImage, origin: [f32; 2]) -> (RgbaImage, [f32; 2]) {
    let seen = image.enumerate_pixels().filter(|(.., pixel)| pixel[3] > 0);
    let Some([x0, y0, x1, y1]) = seen.fold(None, |bounds: Option<[u32; 4]>, (x, y, _)| {
        let [x0, y0, x1, y1] = bounds.unwrap_or([x, y, x, y]);
        Some([x0.min(x), y0.min(y), x1.max(x), y1.max(y)])
    }) else {
        return (image, origin);
    };
    let cut = image::imageops::crop_imm(&image, x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image();
    (cut, [origin[0] - x0 as f32, origin[1] - y0 as f32])
}

/// An image scaled by `scale` along its own axes and turned `degrees` about `origin`
/// (counter-clockwise, y down) by nearest pixel, `resolution` pixels a game px, as GameMaker
/// draws a sprite: the image, where the origin lands in it and its size, in game px.
fn spin(
    image: &RgbaImage,
    origin: [f32; 2],
    degrees: f32,
    scale: [f32; 2],
    resolution: f32,
) -> (RgbaImage, [f32; 2], [f32; 2]) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (w, h) = (image.width() as f32, image.height() as f32);
    let corners = [[0., 0.], [w, 0.], [0., h], [w, h]].map(|[x, y]| {
        let (dx, dy) = ((x - origin[0]) * scale[0], (y - origin[1]) * scale[1]);
        [dx * cos + dy * sin, dy * cos - dx * sin]
    });
    let bound = |axis: usize, pick: fn(f32, f32) -> f32| {
        let mut values = corners.iter().map(|corner| corner[axis]);
        let first = values.next().unwrap_or(0.);
        values.fold(first, pick).round()
    };
    let [x0, y0] = [bound(0, f32::min), bound(1, f32::min)];
    let pixels = [bound(0, f32::max) - x0, bound(1, f32::max) - y0]
        .map(|span| ((span * resolution).ceil() as u32).max(1));
    let spun = RgbaImage::from_fn(pixels[0], pixels[1], |i, j| {
        let rx = (i as f32 + 0.5) / resolution + x0;
        let ry = (j as f32 + 0.5) / resolution + y0;
        let sx = (rx * cos - ry * sin) / scale[0] + origin[0];
        let sy = (rx * sin + ry * cos) / scale[1] + origin[1];
        if (0. ..w).contains(&sx) && (0. ..h).contains(&sy) {
            *image.get_pixel(sx as u32, sy as u32)
        } else {
            Rgba([0; 4])
        }
    });
    let size = pixels.map(|px| px as f32 / resolution);
    (spun, [-x0, -y0], size)
}

fn bundled(sprite: &str) -> &'static [&'static [u8]] {
    SKILL_SPRITES
        .iter()
        .find(|(name, _)| *name == sprite)
        .unwrap_or_else(|| panic!("sprite {sprite} is not in assets/skill-fx"))
        .1
}

fn decode(png: &[u8], tint: Option<[u8; 3]>, add: bool) -> RgbaImage {
    let mut rgba = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .expect("embedded skill sprite must be a PNG")
        .into_rgba8();
    for pixel in rgba.pixels_mut() {
        *pixel = look(*pixel, tint, add);
    }
    rgba
}

/// GPUI blends only by alpha, so an added sprite becomes the colour it adds, opaque where it adds most.
fn look(Rgba([r, g, b, a]): Rgba<u8>, tint: Option<[u8; 3]>, add: bool) -> Rgba<u8> {
    let tint = tint.unwrap_or([255; 3]);
    let rgb: [f32; 3] =
        std::array::from_fn(|i| f32::from([r, g, b][i]) * f32::from(tint[i]) / 255.);
    if !add {
        return Rgba([rgb[0] as u8, rgb[1] as u8, rgb[2] as u8, a]);
    }
    let added = rgb.map(|c| c * f32::from(a) / 255.);
    let peak = added.into_iter().fold(0., f32::max);
    if peak < 1. {
        return Rgba([0; 4]);
    }
    let [r, g, b] = added.map(|c| (c * 255. / peak).round() as u8);
    Rgba([r, g, b, peak.round() as u8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_layer_draws_a_bundled_sprite_of_a_simulated_skill() {
        for (skill, objects) in &FILE.skills {
            assert!(
                crate::skill_sim::skill(skill).is_some(),
                "{skill} has sprites but no motion in skill-fx.json"
            );
            for layer in objects.values().flatten() {
                let frames = frames(&layer.sprite, layer.tint, layer.add);
                assert!(!frames.images.is_empty(), "{} has no frames", layer.sprite);
            }
        }
        for sprite in [&FILE.dummy.idle, &FILE.dummy.hit] {
            assert!(!frames(sprite, None, false).images.is_empty());
        }
    }

    #[test]
    fn a_turned_flare_points_down_with_its_origin_carried_along() {
        let flare = frames("Exo_Black_Hole_Cosmic_Flare_spr", None, true);
        let down = turned("Exo_Black_Hole_Cosmic_Flare_spr", None, true, 270.);
        assert_eq!(flare.size, [86., 56.]);
        assert_eq!(down.size, [56., 86.]);
        assert_eq!(down.origin, [26., 71.]);
        assert_eq!(down.images.len(), flare.images.len());
    }

    #[test]
    fn turned_frames_are_shared_within_a_few_degrees() {
        let sprite = "Exo_Black_Hole_Cosmic_Flare_spr";
        let near = turned(sprite, None, true, 268.);
        assert!(Arc::ptr_eq(&near, &turned(sprite, None, true, 270.)));
    }

    fn layer(json: &str) -> Layer {
        serde_json::from_str(json).expect("a layer")
    }

    #[test]
    fn a_bent_sprite_wraps_once_round_the_circle_with_its_bottom_row_outside() {
        let colours = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]];
        let band = RgbaImage::from_fn(4, 2, |x, y| {
            let [r, g, b] = colours[x as usize];
            Rgba([r, g, b, if y == 0 { 100 } else { 200 }])
        });
        let ring = bend(&band, [20., 4.]);
        assert_eq!(ring.dimensions(), (46, 46));
        let at = |degrees: f32, reach: f32| {
            let (sin, cos) = degrees.to_radians().sin_cos();
            *ring.get_pixel((23. + cos * reach) as u32, (23. - sin * reach) as u32)
        };
        // Counter-clockwise from the right: a quarter of the sprite's width per quarter turn.
        for (column, degrees) in [45., 135., 225., 315.].into_iter().enumerate() {
            let [r, g, b] = colours[column];
            assert_eq!(at(degrees, 19.), Rgba([r, g, b, 100]), "{degrees}° inside");
            assert_eq!(at(degrees, 21.), Rgba([r, g, b, 200]), "{degrees}° outside");
        }
        assert_eq!(at(45., 15.), Rgba([0; 4]));
        assert_eq!(at(45., 23.5), Rgba([0; 4]));
    }

    #[test]
    fn a_widened_ring_floors_its_radius_and_width_as_the_game() {
        let circle = Circle {
            radius: 128.,
            width: 12.,
        };
        assert_eq!(circle.at(1.), [128., 12.]);
        assert_eq!(circle.at(1.15), [147., 13.]);
        assert_eq!(circle.at(1.75), [224., 21.]);
    }

    #[test]
    fn a_ring_is_baked_once_per_size_round_its_centre() {
        let sprite = "Pyromancer_Scorching_Aura_Circle_spr";
        let base = ring(sprite, None, true, [128., 12.]);
        assert_eq!((base.size, base.origin), ([270., 270.], [135., 135.]));
        assert_eq!(base.images.len(), 6);
        assert!(Arc::ptr_eq(&base, &ring(sprite, None, true, [128., 12.])));
        assert_eq!(ring(sprite, None, true, [224., 21.]).size, [472., 472.]);
    }

    #[test]
    fn a_steady_layer_runs_with_the_loop_and_others_with_their_object() {
        let aged = layer(r#"{"sprite": "s", "speed": 0.2}"#);
        let steady = layer(r#"{"sprite": "s", "speed": 0.2, "steady": true}"#);
        assert_eq!(aged.frame(11, 37, 6), 2);
        assert_eq!(steady.frame(11, 37, 6), 1);
    }

    #[test]
    fn a_fixed_or_bent_layer_keeps_its_size_whatever_its_object_s_scale() {
        let ring = r#"{"sprite": "s", "circle": {"radius": 128, "width": 12}}"#;
        assert_eq!(
            layer(r#"{"sprite": "s"}"#).object_scale([1.75; 2]),
            [1.75; 2]
        );
        let fixed = layer(r#"{"sprite": "s", "fixed": true}"#);
        assert_eq!(fixed.object_scale([1.75; 2]), [1.; 2]);
        assert_eq!(layer(ring).object_scale([1.75; 2]), [1.; 2]);
    }

    #[test]
    fn a_layer_can_start_on_a_later_frame_and_hold_it() {
        let hold = layer(r#"{"sprite": "s", "first": 6, "from": 20}"#);
        assert_eq!((hold.frame(20, 0, 10), hold.frame(29, 0, 10)), (6, 6));
        let decay = layer(r#"{"sprite": "s", "speed": 0.5, "first": 6.5, "from": 30}"#);
        assert_eq!(decay.frame(31, 0, 10), 7);
    }

    #[test]
    fn the_lash_plays_the_hand_driven_swing_of_the_game() {
        let lash = layers("scorching_whip", "Exo_Scorching_Whip_obj").expect("the lash");
        let shown: Vec<Option<usize>> = (1..=37)
            .map(|age| {
                let on = |l: &&Layer| age >= l.from && l.until.is_none_or(|until| age < until);
                lash.iter().find(on).map(|l| l.frame(age, 0, 10))
            })
            .collect();
        // Alarm_0 shows it 3 frames in; Step_0 climbs 10 / 26.4 a frame to 6, holds there while
        // its lifetime lasts, then climbs on and destroys it on reaching 9.
        let climb = [0, 0, 1, 1, 1, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 6];
        let frames = climb
            .into_iter()
            .chain([6; 10])
            .chain([6, 6, 7, 7, 7, 8, 8]);
        let expected: Vec<_> = [None; 3].into_iter().chain(frames.map(Some)).collect();
        assert_eq!(shown, [expected, vec![None]].concat());
        for proc in [
            "Exo_Solar_Burst_obj",
            "Exo_Solar_Dash_Fireball_obj",
            "Exo_Solar_Dash_Flame_obj",
        ] {
            let first = layers("scorching_whip", proc).expect(proc)[0].from;
            assert_eq!(first, 2, "{proc} shows from the alarm 1 frame in");
        }
    }

    #[test]
    fn a_flipping_layer_mirrors_its_sprite_only_while_it_points_left() {
        let lash = layer(r#"{"sprite": "s", "turns": true, "flips": true}"#);
        let plain = layer(r#"{"sprite": "s", "turns": true}"#);
        assert!(lash.spins([1., 1.]) && plain.spins([2., 1.]) && !plain.spins([1.5, 1.5]));
        let mirrored = [
            (0., 1.),
            (90., 1.),
            (91., -1.),
            (180., -1.),
            (269., -1.),
            (270., 1.),
        ];
        for (degrees, y) in mirrored.into_iter().chain([(-90., 1.), (-135., -1.)]) {
            assert_eq!(lash.spun_scale([2., 1.], degrees), [2., y], "{degrees}°");
        }
        assert_eq!(plain.spun_scale([2., 1.], 180.), [2., 1.]);
        assert_eq!(lash.object_scale([2., 1.]), [1., 1.]);
    }

    #[test]
    fn an_uneven_sprite_is_scaled_along_its_own_axes_before_it_turns() {
        let red = |y: u32| Rgba([255, 0, 0, 100 + 100 * y as u8]);
        let blue = |y: u32| Rgba([0, 0, 255, 100 + 100 * y as u8]);
        let band = RgbaImage::from_fn(2, 2, |x, y| if x == 0 { red(y) } else { blue(y) });
        // Twice as long, then turned to point up: its far blue end on top, the origin at its foot.
        let (up, origin, size) = spin(&band, [0., 1.], 90., [2., 1.], 1.);
        assert_eq!(
            (up.dimensions(), origin, size),
            ((2, 4), [1., 4.], [2., 4.])
        );
        assert_eq!(
            [up[(0, 0)], up[(1, 1)], up[(0, 3)]],
            [blue(0), blue(1), red(0)]
        );
        let (mirrored, ..) = spin(&band, [0., 1.], 90., [2., -1.], 1.);
        assert_eq!([mirrored[(0, 0)], mirrored[(1, 3)]], [blue(1), red(0)]);
        let (half, origin, size) = spin(&band, [0., 1.], 90., [2., 1.], 0.5);
        assert_eq!(
            (half.dimensions(), origin, size),
            ((1, 2), [1., 4.], [2., 4.])
        );
    }

    #[test]
    fn a_spun_frame_is_baked_once_and_cut_to_what_it_shows() {
        let sprite = "Exo_Scorching_Whip_Right_spr";
        let spun = spun(sprite, None, true, 7.5, [2., 1.], 6);
        assert!(Arc::ptr_eq(
            &spun,
            &super::spun(sprite, None, true, 7.5, [2., 1.], 6)
        ));
        assert_eq!(spun.images.len(), 1);
        let pixels = spun.images[0].size(0);
        let measured = [pixels.width.0, pixels.height.0].map(|px| px as f32 / SPIN_RESOLUTION);
        assert_eq!(measured, spun.size);
        // The 200 px frame doubled is cut to its lash, well short of the 400 px it would span.
        assert!(
            spun.size[0] < 380. && spun.size[1] < 120.,
            "{:?}",
            spun.size
        );
    }

    #[test]
    fn an_added_sprite_keeps_the_colour_it_adds() {
        assert_eq!(
            look(Rgba([255, 128, 0, 128]), None, true),
            Rgba([255, 128, 0, 128])
        );
        assert_eq!(look(Rgba([255, 255, 255, 0]), None, true), Rgba([0; 4]));
        assert_eq!(
            look(Rgba([200, 100, 50, 255]), Some([0, 0, 0]), false),
            Rgba([0, 0, 0, 255])
        );
    }
}
