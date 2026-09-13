//! SHTTS speech synthesis with deterministic, fixed-point processing.
//!
//! ```
//! use shtts::{synthesize, Emotion, Voice, VoiceSettings};
//! let settings = VoiceSettings::from(Voice::Female).with_emotion(Emotion::Happy);
//! let pcm = synthesize("コンニチハ", &settings)?;
//! assert!(!pcm.is_empty());
//! # Ok::<(), shtts::InvalidParameter>(())
//! ```
#![forbid(unsafe_code)]

mod acoustic;
mod dsp;
mod prosody;
mod text;
mod utterance;
mod vocoder;
mod voice;

pub use voice::{Emotion, Voice, VoiceSettings};

/// Synthesize a kana string as mono, signed 16-bit PCM.
///
/// The engine's reference playback rate is 11,025 Hz.
/// Unknown characters are skipped; this engine does not provide a kanji reading dictionary.
///
/// # Errors
/// Returns an error for parameters outside the native signed 16-bit domain,
/// or nonpositive speed or spectral scaling.
pub fn synthesize(text: &str, settings: &VoiceSettings) -> Result<Vec<i16>, InvalidParameter> {
    settings.validate()?;
    let segments = text::analyze(text, settings);
    Ok(vocoder::synthesize(&segments, settings, 11000))
}

/// Playback rate of the reference PCM, in samples per second.
pub const SAMPLE_RATE: u32 = 11025;

/// A synthesis property cannot be represented by the engine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidParameter {
    pub name: &'static str,
    pub value: i32,
}

impl std::fmt::Display for InvalidParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid {} parameter: {}", self.name, self.value)
    }
}
impl std::error::Error for InvalidParameter {}
