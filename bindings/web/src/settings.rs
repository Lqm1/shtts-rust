//! Validated settings at the JavaScript boundary.
use crate::{Emotion, Voice};
use shtts::VoiceSettings;
use wasm_bindgen::prelude::*;

/// Editable synthesis parameters.
///
/// Every property accepts finite integers only. Invalid assignments throw Error
/// and leave the previous value unchanged. Free this object after use; returned
/// PCM arrays remain valid independently of its lifetime.
#[wasm_bindgen]
#[derive(Default)]
pub struct Settings {
    pub(crate) inner: VoiceSettings,
}

#[wasm_bindgen]
impl Settings {
    /// Create neutral engine settings without applying a speaker preset.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a speaker and optionally apply an emotion before manual edits.
    pub fn preset(voice: Voice, emotion: Option<Emotion>) -> Self {
        let mut inner = VoiceSettings::from(shtts::Voice::from(voice));
        if let Some(emotion) = emotion {
            inner = inner.with_emotion(emotion.into());
        }
        Self { inner }
    }

    /// Base pitch code, not Hz. Larger values raise the pitch. Default: 5058.
    ///
    /// Accepts integers from -32768 through 32767.
    #[wasm_bindgen(getter = pitch)]
    pub fn pitch(&self) -> i32 {
        self.inner.pitch
    }

    /// Set pitch; throws Error for a non-finite, fractional or out-of-range value.
    #[wasm_bindgen(setter = pitch)]
    pub fn set_pitch(&mut self, value: f64) -> Result<(), JsError> {
        self.inner.pitch = checked_integer("pitch", value, i16::MIN)?;
        Ok(())
    }

    /// Initial accent modulation. 100 is neutral.
    ///
    /// Accepts integers from -32768 through 32767.
    #[wasm_bindgen(getter = accent)]
    pub fn accent(&self) -> i32 {
        self.inner.accent
    }

    /// Set accent; throws Error for a non-finite, fractional or out-of-range value.
    #[wasm_bindgen(setter = accent)]
    pub fn set_accent(&mut self, value: f64) -> Result<(), JsError> {
        self.inner.accent = checked_integer("accent", value, i16::MIN)?;
        Ok(())
    }

    /// Following accent modulation. 100 is neutral.
    ///
    /// Accepts integers from -32768 through 32767.
    #[wasm_bindgen(getter = phraseAccent)]
    pub fn phrase_accent(&self) -> i32 {
        self.inner.phrase_accent
    }

    /// Set phraseAccent; throws Error for a non-finite, fractional or out-of-range value.
    #[wasm_bindgen(setter = phraseAccent)]
    pub fn set_phrase_accent(&mut self, value: f64) -> Result<(), JsError> {
        self.inner.phrase_accent = checked_integer("phraseAccent", value, i16::MIN)?;
        Ok(())
    }

    /// Additive amplitude offset in logarithmic units. 0 is neutral.
    ///
    /// Accepts integers from -32768 through 32767.
    #[wasm_bindgen(getter = volume)]
    pub fn volume(&self) -> i32 {
        self.inner.volume
    }

    /// Set volume; throws Error for a non-finite, fractional or out-of-range value.
    #[wasm_bindgen(setter = volume)]
    pub fn set_volume(&mut self, value: f64) -> Result<(), JsError> {
        self.inner.volume = checked_integer("volume", value, i16::MIN)?;
        Ok(())
    }

    /// Duration percentage. 100 is neutral; larger values speak more slowly.
    ///
    /// Accepts integers from 1 through 32767.
    #[wasm_bindgen(getter = speed)]
    pub fn speed(&self) -> i32 {
        self.inner.speed
    }

    /// Set speed; throws Error for a non-finite, fractional or out-of-range value.
    #[wasm_bindgen(setter = speed)]
    pub fn set_speed(&mut self, value: f64) -> Result<(), JsError> {
        self.inner.speed = checked_integer("speed", value, 1)?;
        Ok(())
    }

    /// Depth of delayed pitch variation. 0 disables variation.
    ///
    /// Accepts integers from -32768 through 32767.
    #[wasm_bindgen(getter = fluctuationDepth)]
    pub fn fluctuation_depth(&self) -> i32 {
        self.inner.fluctuation_depth
    }

    /// Set fluctuationDepth; throws Error for a non-finite, fractional or out-of-range value.
    #[wasm_bindgen(setter = fluctuationDepth)]
    pub fn set_fluctuation_depth(&mut self, value: f64) -> Result<(), JsError> {
        self.inner.fluctuation_depth = checked_integer("fluctuationDepth", value, i16::MIN)?;
        Ok(())
    }

    /// Pitch variation delay in samples. 0 disables variation.
    ///
    /// Accepts integers from -32768 through 32767.
    #[wasm_bindgen(getter = fluctuationDelay)]
    pub fn fluctuation_delay(&self) -> i32 {
        self.inner.fluctuation_delay
    }

    /// Set fluctuationDelay; throws Error for a non-finite, fractional or out-of-range value.
    #[wasm_bindgen(setter = fluctuationDelay)]
    pub fn set_fluctuation_delay(&mut self, value: f64) -> Result<(), JsError> {
        self.inner.fluctuation_delay = checked_integer("fluctuationDelay", value, i16::MIN)?;
        Ok(())
    }

    /// Echo delay in samples at the playback sample rate.
    ///
    /// Accepts integers from -32768 through 32767.
    #[wasm_bindgen(getter = echoDelay)]
    pub fn echo_delay(&self) -> i32 {
        self.inner.echo_delay
    }

    /// Set echoDelay; throws Error for a non-finite, fractional or out-of-range value.
    #[wasm_bindgen(setter = echoDelay)]
    pub fn set_echo_delay(&mut self, value: f64) -> Result<(), JsError> {
        self.inner.echo_delay = checked_integer("echoDelay", value, i16::MIN)?;
        Ok(())
    }

    /// Spectral scale percentage. 100 is neutral.
    ///
    /// Accepts integers from 1 through 32767.
    #[wasm_bindgen(getter = spectral)]
    pub fn spectral(&self) -> i32 {
        self.inner.spectral
    }

    /// Set spectral; throws Error for a non-finite, fractional or out-of-range value.
    #[wasm_bindgen(setter = spectral)]
    pub fn set_spectral(&mut self, value: f64) -> Result<(), JsError> {
        self.inner.spectral = checked_integer("spectral", value, 1)?;
        Ok(())
    }

    /// Echo mix percentage. 0 disables echo.
    ///
    /// Accepts integers from -32768 through 32767.
    #[wasm_bindgen(getter = echoDepth)]
    pub fn echo_depth(&self) -> i32 {
        self.inner.echo_depth
    }

    /// Set echoDepth; throws Error for a non-finite, fractional or out-of-range value.
    #[wasm_bindgen(setter = echoDepth)]
    pub fn set_echo_depth(&mut self, value: f64) -> Result<(), JsError> {
        self.inner.echo_depth = checked_integer("echoDepth", value, i16::MIN)?;
        Ok(())
    }

    /// Ring modulation rate in native units. 0 disables the effect.
    ///
    /// Accepts integers from -32768 through 32767.
    #[wasm_bindgen(getter = ringRate)]
    pub fn ring_rate(&self) -> i32 {
        self.inner.ring_rate
    }

    /// Set ringRate; throws Error for a non-finite, fractional or out-of-range value.
    #[wasm_bindgen(setter = ringRate)]
    pub fn set_ring_rate(&mut self, value: f64) -> Result<(), JsError> {
        self.inner.ring_rate = checked_integer("ringRate", value, i16::MIN)?;
        Ok(())
    }
}

fn checked_integer(name: &str, value: f64, minimum: i16) -> Result<i32, JsError> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value < f64::from(minimum)
        || value > f64::from(i16::MAX)
    {
        return Err(JsError::new(&format!(
            "{name} must be a finite integer between {minimum} and {}",
            i16::MAX
        )));
    }
    Ok(value as i32)
}
