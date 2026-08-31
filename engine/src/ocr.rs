use base64::Engine as _;
use image::RgbImage;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use once_cell::sync::Lazy;
use rten::Model;

mod models {
    include!(concat!(env!("OUT_DIR"), "/ocr_includes.rs"));
}

static ENGINE: Lazy<Result<OcrEngine, String>> = Lazy::new(|| {
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

fn preprocess(img: RgbImage) -> RgbImage {
    let (w, h) = img.dimensions();
    let sum: u64 = img.pixels().map(|p| p.0.iter().map(|&c| c as u64).sum::<u64>()).sum();
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
    let engine = ENGINE.as_ref().map_err(|e| e.clone())?;
    let img = image::load_from_memory(bytes)
        .map_err(|e| format!("decode image: {e}"))?
        .into_rgb8();
    let img = preprocess(img);
    let source = ImageSource::from_bytes(img.as_raw(), img.dimensions())
        .map_err(|e| format!("prepare image: {e}"))?;
    let input = engine
        .prepare_input(source)
        .map_err(|e| format!("prepare OCR input: {e}"))?;
    let text = engine.get_text(&input).map_err(|e| format!("OCR: {e}"))?;
    Ok(text
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

#[tauri::command]
pub fn ocr_tooltip_lines(image_base64: String) -> Result<Vec<String>, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64.trim())
        .map_err(|e| format!("decode base64: {e}"))?;
    ocr_image_bytes(&bytes)
}
