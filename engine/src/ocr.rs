use image::RgbImage;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams, TextItem};
use rten::Model;
use std::sync::LazyLock;

mod models {
    include!(concat!(env!("OUT_DIR"), "/ocr_includes.rs"));
}

static ENGINE: LazyLock<Result<OcrEngine, String>> = LazyLock::new(|| {
    let detection = Model::load_static_slice(models::TEXT_DETECTION)
        .map_err(|e| format!("load detection model: {e}"))?;
    let recognition = Model::load_static_slice(models::TEXT_RECOGNITION)
        .map_err(|e| format!("load recognition model: {e}"))?;
    OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection),
        recognition_model: Some(recognition),
        ..Default::default()
    })
    .map_err(|e| format!("init OCR engine: {e}"))
});

const UPSCALE_BELOW_WIDTH: u32 = 1200;
/// The brightest sixth of a line's box is its glyphs; the rest is dark tooltip background.
const TEXT_PIXEL_SHARE: usize = 6;

/// A recognized tooltip line and the color of its text.
#[derive(Clone, Debug, PartialEq)]
pub struct OcrLine {
    pub text: String,
    pub color: [u8; 3],
}

fn preprocess(img: RgbImage) -> RgbImage {
    let (w, h) = img.dimensions();
    let sum: u64 = img
        .pixels()
        .map(|p| p.0.iter().map(|&c| c as u64).sum::<u64>())
        .sum();
    let mean = sum / (w as u64 * h as u64 * 3).max(1);
    let img = if mean < 128 {
        let mut inv = img;
        for p in inv.pixels_mut() {
            p.0 = [255 - p.0[0], 255 - p.0[1], 255 - p.0[2]];
        }
        inv
    } else {
        img
    };
    if w < UPSCALE_BELOW_WIDTH {
        image::imageops::resize(&img, w * 2, h * 2, image::imageops::FilterType::CatmullRom)
    } else {
        img
    }
}

pub fn ocr_image_bytes(bytes: &[u8]) -> Result<Vec<String>, String> {
    ocr_image_lines(bytes).map(|lines| lines.into_iter().map(|line| line.text).collect())
}

pub fn ocr_image_lines(bytes: &[u8]) -> Result<Vec<OcrLine>, String> {
    ocr_image_lines_controlled(
        bytes,
        &crate::task_control::Cancellation::default(),
        |_, _| {},
    )
}

fn text_color(img: &RgbImage, [left, top, right, bottom]: [i32; 4], scale: i32) -> [u8; 3] {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let (x0, x1) = ((left / scale).clamp(0, w), (right / scale).clamp(0, w));
    let (y0, y1) = ((top / scale).clamp(0, h), (bottom / scale).clamp(0, h));
    let mut pixels: Vec<[u8; 3]> = (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| img.get_pixel(x as u32, y as u32).0))
        .collect();
    if pixels.is_empty() {
        return [0; 3];
    }
    pixels.sort_unstable_by_key(|p| std::cmp::Reverse(p.iter().copied().max().unwrap_or(0)));
    let core = &pixels[..(pixels.len() / TEXT_PIXEL_SHARE).max(1)];
    let mut sum = [0u32; 3];
    for pixel in core {
        for (total, channel) in sum.iter_mut().zip(pixel) {
            *total += u32::from(*channel);
        }
    }
    sum.map(|total| (total / core.len() as u32) as u8)
}

pub fn ocr_image_lines_controlled(
    bytes: &[u8],
    cancellation: &crate::task_control::Cancellation,
    progress: impl Fn(u32, u32),
) -> Result<Vec<OcrLine>, String> {
    cancellation.check()?;
    progress(0, 4);
    let engine = ENGINE.as_ref().map_err(|e| e.clone())?;
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("decode image: {e}"))?
        .into_rgb8();
    cancellation.check()?;
    progress(1, 4);
    let original = img.clone();
    let img = preprocess(img);
    let scale = (img.width() / original.width().max(1)).max(1) as i32;
    let source = ImageSource::from_bytes(img.as_raw(), img.dimensions())
        .map_err(|e| format!("prepare image: {e}"))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| format!("prepare OCR input: {e}"))?;
    cancellation.check()?;
    progress(2, 4);
    let words = engine
        .detect_words(&input)
        .map_err(|e| format!("OCR detection: {e}"))?;
    cancellation.check()?;
    let lines = engine.find_text_lines(&input, &words);
    progress(3, 4);
    let lines = engine
        .recognize_text(&input, &lines)
        .map_err(|e| format!("OCR recognition: {e}"))?;
    cancellation.check()?;
    progress(4, 4);
    Ok(lines
        .iter()
        .flatten()
        .filter_map(|line| {
            let text = line.to_string().trim().to_string();
            let rect = line.bounding_rect();
            let bounds = [rect.left(), rect.top(), rect.right(), rect.bottom()];
            (!text.is_empty()).then(|| OcrLine {
                color: text_color(&original, bounds, scale),
                text,
            })
        })
        .collect())
}
