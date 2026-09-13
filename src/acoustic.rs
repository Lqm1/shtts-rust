//! Statically stored acoustic frames, units and fixed-point synthesis tables.
mod frames;
mod tables;
mod units;

/// Index into the shared, expanded LSP vectors.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SpectrumId(u16);

pub(crate) const ORDER: usize = 10;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Frame {
    pub voiced: bool,
    pub amplitude: i32,
    pub spectrum: SpectrumId,
    /// Whether this frame supplies excitation, independently of its LSP values.
    pub excitable: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Unit {
    pub start: usize,
    pub boundaries: [u8; 4],
}

pub(crate) struct AcousticData {
    pub frames: &'static [Frame],
    pub coarse_pitch: [i16; 50],
    pub fine_pitch: [i16; 128],
    pub pulse: [i16; 35],
    pub impulse: [i16; 8],
    pub noise: [i16; 360],
    pub ring_sine: [i16; 256],
}

impl AcousticData {
    pub fn embedded() -> &'static Self {
        static DATA: AcousticData = AcousticData {
            frames: &frames::FRAMES,
            coarse_pitch: tables::COARSE_PITCH,
            fine_pitch: tables::FINE_PITCH,
            pulse: tables::PULSE,
            impulse: tables::IMPULSE,
            noise: tables::NOISE,
            ring_sine: tables::RING_SINE,
        };
        &DATA
    }

    pub fn unit(&self, left: usize, center: usize, right: usize) -> Option<Unit> {
        *units::UNITS.get(left)?.get(center)?.get(right)?
    }

    pub fn period(&self, pitch: i32, scale: i32) -> usize {
        if pitch <= 0 {
            return 0;
        }
        let coarse = ((pitch >> 7) - 28).clamp(0, 49) as usize;
        let fine = (pitch & 127) as usize;
        let value = (i64::from(self.coarse_pitch[coarse]) * i64::from(self.fine_pitch[fine])) >> 16;
        let value = (value * i64::from(scale)) >> 16;
        ((value * 16098) >> 16).clamp(0, 239) as usize
    }

    pub fn spectrum(&self, frame: Frame) -> [i32; ORDER] {
        frames::SPECTRA[usize::from(frame.spectrum.0)]
    }
}
