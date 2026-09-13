//! Voice presets and their parameter overrides.

/// A speaker preset from the SHTTS voice table.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
#[repr(u16)]
pub enum Voice {
    #[default]
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

/// An emotion modifies the selected speaker before explicit overrides are applied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
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

/// Synthesis parameters in the engine's native integer units.
///
/// The default uses the engine's neutral initialization, without a speaker preset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoiceSettings {
    /// Base pitch code. Larger values produce shorter periods.
    pub pitch: i32,
    /// Initial accent modulation, with 100 as the neutral setting.
    pub accent: i32,
    /// Subsequent accent modulation, with 100 as the neutral setting.
    pub phrase_accent: i32,
    /// Additive amplitude offset in the engine's logarithmic scale.
    pub volume: i32,
    /// Duration percentage. A larger value makes speech slower; must be positive.
    pub speed: i32,
    /// Depth of the delayed pitch-fluctuation effect, zero to disable.
    pub fluctuation_depth: i32,
    /// Delay of the pitch-fluctuation effect in samples, zero to disable.
    pub fluctuation_delay: i32,
    /// Echo delay in samples.
    pub echo_delay: i32,
    /// Spectral scale percentage, with 100 as neutral; must be positive.
    pub spectral: i32,
    /// Echo mix percentage, zero to disable.
    pub echo_depth: i32,
    /// Ring-modulator rate in the engine's native units, zero to disable.
    pub ring_rate: i32,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            pitch: 5058,
            accent: 100,
            phrase_accent: 100,
            volume: 0,
            speed: 100,
            fluctuation_depth: 0,
            fluctuation_delay: 0,
            echo_delay: 0,
            spectral: 100,
            echo_depth: 0,
            ring_rate: 0,
        }
    }
}

impl From<Voice> for VoiceSettings {
    fn from(voice: Voice) -> Self {
        let (pitch, speed, spectral) = match voice {
            Voice::Male => (4800, 90, 90),
            Voice::Female => (5250, 90, 103),
            Voice::AdultMale => (4800, 100, 97),
            Voice::AdultFemale => (5100, 100, 100),
            Voice::ElderlyMale => (4300, 120, 85),
            Voice::ElderlyFemale => (5250, 130, 95),
            Voice::Boy => (5400, 80, 107),
            Voice::Girl => (5650, 75, 115),
            Voice::Sisters => (5250, 90, 103),
            Voice::Meimu => (5700, 100, 120),
            Voice::Space => (5300, 100, 100),
            Voice::Giant => (4200, 130, 84),
        };
        let mut settings = Self {
            pitch,
            speed,
            spectral,
            fluctuation_depth: 20,
            fluctuation_delay: 500,
            ..Self::default()
        };
        match voice {
            Voice::Sisters => {
                settings.echo_delay = 700;
                settings.echo_depth = 80;
            }
            Voice::Space => {
                settings.fluctuation_depth = 30;
                settings.fluctuation_delay = 2000;
                settings.ring_rate = 1000;
            }
            Voice::Giant => {
                settings.fluctuation_depth = 60;
                settings.fluctuation_delay = 1500;
            }
            _ => {}
        }
        settings
    }
}

impl VoiceSettings {
    pub(crate) fn validate(&self) -> Result<(), crate::InvalidParameter> {
        for (name, value) in [
            ("pitch", self.pitch),
            ("accent", self.accent),
            ("phrase_accent", self.phrase_accent),
            ("volume", self.volume),
            ("speed", self.speed),
            ("fluctuation_depth", self.fluctuation_depth),
            ("fluctuation_delay", self.fluctuation_delay),
            ("echo_delay", self.echo_delay),
            ("spectral", self.spectral),
            ("echo_depth", self.echo_depth),
            ("ring_rate", self.ring_rate),
        ] {
            if i16::try_from(value).is_err() || (matches!(name, "speed" | "spectral") && value <= 0)
            {
                return Err(crate::InvalidParameter { name, value });
            }
        }
        Ok(())
    }

    pub fn with_emotion(mut self, emotion: Emotion) -> Self {
        let (pitch, accent, phrase, volume, speed) = match emotion {
            Emotion::Angry => (200, 130, 60, 30, 80),
            Emotion::Business => (0, 90, 90, 0, 90),
            Emotion::Calm => (0, 90, 90, 0, 110),
            Emotion::Depressed => (-200, 90, 90, 0, 110),
            Emotion::Excited => (200, 120, 120, 0, 90),
            Emotion::Falsetto => (700, 100, 100, 0, 100),
            Emotion::Happy => (100, 140, 100, 0, 100),
            Emotion::Loud => (0, 100, 100, 60, 100),
            Emotion::Monotone => (500, 0, 0, 0, 100),
            Emotion::Perky => (0, 140, 140, 0, 100),
            Emotion::Quiet => (0, 100, 100, -60, 100),
            Emotion::Sarcastic => (-100, 100, 70, 0, 80),
            Emotion::Scared => (500, 60, 90, 0, 80),
            Emotion::Shout => (1000, 40, 40, 60, 100),
            Emotion::Tense => (400, 60, 60, 0, 100),
            Emotion::Whisper => (0, 100, 100, -200, 100),
        };
        self.pitch += pitch;
        self.accent = accent;
        self.phrase_accent = phrase;
        self.volume = volume;
        if speed != 100 {
            self.speed = speed;
        }
        if matches!(emotion, Emotion::Happy | Emotion::Scared) {
            self.fluctuation_depth = 50;
            self.fluctuation_delay = 500;
        }
        if emotion == Emotion::Happy {
            self.spectral = 105;
        }
        self
    }
}
