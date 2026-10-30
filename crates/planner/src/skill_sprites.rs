//! The game's sprites for the skill preview: data/skill-sprites.json and assets/skill-fx.
use crate::scene::render_image;
use gpui_kit::RenderImage;
use image::{Rgba, RgbaImage};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

include!(concat!(env!("OUT_DIR"), "/skill-sprites.rs"));

/// Floor tiles across and down: wide enough for a dummy 400 px away.
const GROUND_TILES: [u32; 2] = [10, 6];

#[derive(Deserialize)]
struct SpritesFile {
    ground: String,
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
    /// Object age at which another layer takes over.
    pub until: Option<u32>,
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

type Look = (&'static str, Option<[u8; 3]>, bool);

static FILE: LazyLock<SpritesFile> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../data/skill-sprites.json"))
        .expect("embedded skill sprites must be valid")
});

static FRAMES: LazyLock<Mutex<HashMap<Look, Arc<Frames>>>> = LazyLock::new(Default::default);

static GROUND: LazyLock<(Arc<RenderImage>, [f32; 2])> = LazyLock::new(|| {
    let tile = decode(bundled(&FILE.ground)[0], None, false);
    let mut ground = RgbaImage::new(
        tile.width() * GROUND_TILES[0],
        tile.height() * GROUND_TILES[1],
    );
    image::imageops::tile(&mut ground, &tile);
    let size = [ground.width() as f32, ground.height() as f32];
    (render_image(ground), size)
});

/// How a skill's object is drawn, bottom layer first.
pub(crate) fn layers(skill_id: &str, object: &str) -> Option<&'static [Layer]> {
    FILE.skills.get(skill_id)?.get(object).map(Vec::as_slice)
}

pub(crate) fn dummy() -> &'static DummyLook {
    &FILE.dummy
}

/// The town floor tiled into one image, and its size in game px.
pub(crate) fn ground() -> (Arc<RenderImage>, [f32; 2]) {
    GROUND.clone()
}

/// A sprite's frames with a tint and blend, decoded on first use.
pub(crate) fn frames(sprite: &'static str, tint: Option<[u8; 3]>, add: bool) -> Arc<Frames> {
    let mut cache = FRAMES.lock().unwrap_or_else(PoisonError::into_inner);
    cache
        .entry((sprite, tint, add))
        .or_insert_with(|| {
            let decoded: Vec<_> = bundled(sprite)
                .iter()
                .map(|png| decode(png, tint, add))
                .collect();
            let size = [decoded[0].width() as f32, decoded[0].height() as f32];
            Arc::new(Frames {
                images: decoded.into_iter().map(render_image).collect(),
                size,
                origin: FILE.origins[sprite],
            })
        })
        .clone()
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
        assert_eq!(ground().1, [640., 384.]);
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
