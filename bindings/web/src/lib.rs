//! Browser bindings for SHTTS.
#![forbid(unsafe_code)]

use shtts::{Emotion, Voice, VoiceSettings};
use wasm_bindgen::prelude::*;

/// Editable synthesis parameters in the engine's native integer units.
#[wasm_bindgen]
pub struct Settings {
    pub pitch: i32,
    pub accent: i32,
    pub phrase_accent: i32,
    pub volume: i32,
    pub speed: i32,
    pub fluctuation_depth: i32,
    pub fluctuation_delay: i32,
    pub echo_delay: i32,
    pub spectral: i32,
    pub echo_depth: i32,
    pub ring_rate: i32,
}

impl From<VoiceSettings> for Settings {
    fn from(value: VoiceSettings) -> Self {
        Self {
            pitch: value.pitch,
            accent: value.accent,
            phrase_accent: value.phrase_accent,
            volume: value.volume,
            speed: value.speed,
            fluctuation_depth: value.fluctuation_depth,
            fluctuation_delay: value.fluctuation_delay,
            echo_delay: value.echo_delay,
            spectral: value.spectral,
            echo_depth: value.echo_depth,
            ring_rate: value.ring_rate,
        }
    }
}

impl From<&Settings> for VoiceSettings {
    fn from(value: &Settings) -> Self {
        Self {
            pitch: value.pitch,
            accent: value.accent,
            phrase_accent: value.phrase_accent,
            volume: value.volume,
            speed: value.speed,
            fluctuation_depth: value.fluctuation_depth,
            fluctuation_delay: value.fluctuation_delay,
            echo_delay: value.echo_delay,
            spectral: value.spectral,
            echo_depth: value.echo_depth,
            ring_rate: value.ring_rate,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        VoiceSettings::default().into()
    }
}

#[wasm_bindgen]
impl Settings {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a speaker, then apply an optional emotion before manual edits.
    pub fn preset(voice: &str, emotion: &str) -> Result<Settings, JsValue> {
        let mut settings = match voice {
            "Default" => VoiceSettings::default(),
            "Male" => Voice::Male.into(),
            "Female" => Voice::Female.into(),
            "AdultMale" => Voice::AdultMale.into(),
            "AdultFemale" => Voice::AdultFemale.into(),
            "ElderlyMale" => Voice::ElderlyMale.into(),
            "ElderlyFemale" => Voice::ElderlyFemale.into(),
            "Boy" => Voice::Boy.into(),
            "Girl" => Voice::Girl.into(),
            "Sisters" => Voice::Sisters.into(),
            "Meimu" => Voice::Meimu.into(),
            "Space" => Voice::Space.into(),
            "Giant" => Voice::Giant.into(),
            _ => return Err(JsValue::from_str("Unknown voice preset")),
        };
        let emotion = match emotion {
            "None" => None,
            "Angry" => Some(Emotion::Angry),
            "Business" => Some(Emotion::Business),
            "Calm" => Some(Emotion::Calm),
            "Depressed" => Some(Emotion::Depressed),
            "Excited" => Some(Emotion::Excited),
            "Falsetto" => Some(Emotion::Falsetto),
            "Happy" => Some(Emotion::Happy),
            "Loud" => Some(Emotion::Loud),
            "Monotone" => Some(Emotion::Monotone),
            "Perky" => Some(Emotion::Perky),
            "Quiet" => Some(Emotion::Quiet),
            "Sarcastic" => Some(Emotion::Sarcastic),
            "Scared" => Some(Emotion::Scared),
            "Shout" => Some(Emotion::Shout),
            "Tense" => Some(Emotion::Tense),
            "Whisper" => Some(Emotion::Whisper),
            _ => return Err(JsValue::from_str("Unknown emotion preset")),
        };
        if let Some(emotion) = emotion {
            settings = settings.with_emotion(emotion);
        }
        Ok(settings.into())
    }
}

/// Generate mono signed 16-bit samples as an Int16Array.
#[wasm_bindgen]
pub fn synthesize(text: &str, settings: &Settings) -> Result<Vec<i16>, JsValue> {
    shtts::synthesize(text, &settings.into()).map_err(|error| JsValue::from_str(&error.to_string()))
}

#[wasm_bindgen]
pub fn sample_rate() -> u32 {
    shtts::SAMPLE_RATE
}
