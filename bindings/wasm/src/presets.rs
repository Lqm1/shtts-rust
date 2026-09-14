//! Typed speaker and emotion presets for JavaScript.
use wasm_bindgen::prelude::*;

/// Speaker presets. Use Settings.preset() to load one.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Voice {
    Male = 0x101,
    Female,
    AdultMale,
    AdultFemale,
    ElderlyMale,
    ElderlyFemale,
    Boy,
    Girl,
    Sisters,
    Meimu,
    Space,
    Giant,
}

impl From<Voice> for shtts::Voice {
    fn from(value: Voice) -> Self {
        match value {
            Voice::Male => Self::Male,
            Voice::Female => Self::Female,
            Voice::AdultMale => Self::AdultMale,
            Voice::AdultFemale => Self::AdultFemale,
            Voice::ElderlyMale => Self::ElderlyMale,
            Voice::ElderlyFemale => Self::ElderlyFemale,
            Voice::Boy => Self::Boy,
            Voice::Girl => Self::Girl,
            Voice::Sisters => Self::Sisters,
            Voice::Meimu => Self::Meimu,
            Voice::Space => Self::Space,
            Voice::Giant => Self::Giant,
        }
    }
}

/// Emotion presets applied before manual parameter edits.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Emotion {
    Angry = 0x201,
    Business,
    Calm,
    Depressed,
    Excited,
    Falsetto,
    Happy,
    Loud,
    Monotone,
    Perky,
    Quiet,
    Sarcastic,
    Scared,
    Shout,
    Tense,
    Whisper,
}

impl From<Emotion> for shtts::Emotion {
    fn from(value: Emotion) -> Self {
        match value {
            Emotion::Angry => Self::Angry,
            Emotion::Business => Self::Business,
            Emotion::Calm => Self::Calm,
            Emotion::Depressed => Self::Depressed,
            Emotion::Excited => Self::Excited,
            Emotion::Falsetto => Self::Falsetto,
            Emotion::Happy => Self::Happy,
            Emotion::Loud => Self::Loud,
            Emotion::Monotone => Self::Monotone,
            Emotion::Perky => Self::Perky,
            Emotion::Quiet => Self::Quiet,
            Emotion::Sarcastic => Self::Sarcastic,
            Emotion::Scared => Self::Scared,
            Emotion::Shout => Self::Shout,
            Emotion::Tense => Self::Tense,
            Emotion::Whisper => Self::Whisper,
        }
    }
}
