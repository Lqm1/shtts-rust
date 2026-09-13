//! Typed browser bindings for SHTTS.
#![forbid(unsafe_code)]

mod presets;
mod settings;

pub use presets::{Emotion, Voice};
pub use settings::Settings;
use wasm_bindgen::prelude::*;

/// Synthesize kana synchronously into a mono Int16Array.
///
/// Initialize the WASM module first. Use a Worker for longer input to keep the
/// interface responsive. The returned array owns its samples and remains valid
/// after settings.free(). Invalid parameters throw a JavaScript Error.
#[wasm_bindgen]
pub fn synthesize(text: &str, settings: &Settings) -> Result<Vec<i16>, JsError> {
    shtts::synthesize(text, &settings.inner).map_err(|error| JsError::new(&error.to_string()))
}

/// Playback sample rate in Hz for synthesized PCM.
#[wasm_bindgen(js_name = sampleRate)]
pub fn sample_rate() -> u32 {
    shtts::SAMPLE_RATE
}
