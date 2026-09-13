//! Period-synchronous synthesis into a 480-sample overlap buffer.
use crate::{
    acoustic::{AcousticData, Frame},
    dsp::{self, Effects, FramePolicy, Hold},
    prosody,
    utterance::{Segment, UnitPosition, group_segments},
    voice::VoiceSettings,
};

const BLOCK: usize = 240;
const RING: usize = 480;
const UNVOICED_PERIOD: usize = 55;

struct Output {
    ring: [i32; RING],
    write: usize,
    read: usize,
    written: usize,
    emitted: usize,
    samples: Vec<i16>,
    effects: Effects,
}

impl Output {
    fn new(settings: &VoiceSettings) -> Self {
        Self {
            ring: [0; RING],
            write: 0,
            read: 0,
            written: 0,
            emitted: 0,
            samples: Vec::new(),
            effects: Effects::new(settings),
        }
    }
    fn emit(&mut self, start: usize) {
        let mut block = [0; BLOCK];
        for (offset, value) in block.iter_mut().enumerate() {
            let index = (start + offset) % RING;
            *value = std::mem::take(&mut self.ring[index]);
        }
        self.effects.process(&mut block);
        self.samples.extend(block.map(|v| v as i16));
        self.emitted += BLOCK;
    }
    fn advance(&mut self, count: usize) {
        self.written += count;
        self.write += count;
        if self.read == 0 {
            if self.write >= BLOCK {
                self.emit(0);
                self.read = BLOCK;
            }
        } else if self.write < BLOCK || self.write > RING {
            self.emit(BLOCK);
            self.read = 0;
        }
        if self.write >= RING {
            self.write -= RING;
        }
    }
    fn add(&mut self, block: &[i32]) {
        for (offset, &sample) in block.iter().enumerate() {
            let slot = &mut self.ring[(self.write + offset) % RING];
            *slot = (*slot + sample).clamp(-32767, 32767);
        }
    }
    fn flush(&mut self) {
        if self.emitted % RING < self.written % RING {
            self.emit(self.read);
        }
    }
    fn reset_pointers(&mut self) {
        self.write = 0;
        self.read = 0;
        self.written = 0;
        self.emitted = 0;
    }
}

fn predictor(data: &AcousticData, frame: Frame, settings: &VoiceSettings) -> [i16; 10] {
    let mut spectrum = data.spectrum(frame);
    let order = if settings.spectral != 100 {
        dsp::shift_spectrum(
            &mut spectrum,
            (settings.spectral * 655) >> 6,
            settings.spectral,
        );
        dsp::rounded_div(1000, i64::from(settings.spectral)).clamp(1, 10) as usize
    } else {
        10
    };
    dsp::coefficients_with_order(&spectrum, order)
}

fn amplitude_index(value: i32) -> usize {
    ((value * 3277 + 16384) >> 15).clamp(0, 90) as usize
}

pub(crate) fn synthesize(
    segments: &[Segment],
    settings: &VoiceSettings,
    pitch_scale: i32,
) -> Vec<i16> {
    if segments.is_empty() {
        return Vec::new();
    }
    let data = AcousticData::embedded();
    let groups = group_segments(segments);
    let pitches = prosody::pitches(segments, &groups, settings);
    let amplitudes: Vec<_> = groups
        .iter()
        .map(|g| prosody::amplitudes(segments, g, data))
        .collect();
    let mut group_of = vec![0; segments.len()];
    let mut pitch_offset = vec![0; segments.len()];
    for (group_index, group) in groups.iter().enumerate() {
        let mut offset = 0i32;
        for index in group.segments.clone() {
            group_of[index] = group_index;
            pitch_offset[index] = offset;
            offset += segments[index].periods;
        }
    }
    let mut frame_phonemes = vec![None; data.frames.len()];
    for segment in segments {
        if segment.phoneme != 0 && segment.periods > 0 {
            for frame in segment.frames.clone() {
                frame_phonemes[segment.unit_start + frame] = Some(segment.phoneme);
            }
        }
    }
    let mut work = segments.to_vec();
    let last = work.last().unwrap();
    if last.phoneme != 0 {
        let mut periods = last.tail_periods;
        if settings.speed != 100 {
            periods = (periods * settings.speed + 50) / 100;
        }
        work.push(Segment {
            phoneme: 0,
            periods,
            original_periods: periods,
            unit_start: 0,
            frames: 0..1,
            pitch: 0,
            amplitude: 0,
            devoiced: false,
            tail_periods: 200,
        });
    }
    let mut output = Output::new(settings);
    let mut phase = 0i32;
    let mut noise_count = 0;
    for (index, segment) in work.iter().enumerate() {
        if segment.periods <= 0 {
            continue;
        }
        if segment.phoneme == 0 {
            let hold = segment.unit_start != 0 && segment.amplitude != 0;
            for period in 0..segment.periods as usize {
                if hold {
                    let frame = data.frames[segment.unit_start
                        + segment.frames.start
                        + period.min(segment.frames.len() - 1)];
                    let block = frame.excitable.then(|| {
                        dsp::excitation(
                            &data.impulse,
                            &predictor(data, frame, settings),
                            amplitude_index(segment.amplitude),
                            BLOCK,
                            1564,
                        )
                    });
                    phase += UNVOICED_PERIOD as i32;
                    while phase > 0 {
                        if let Some(block) = &block {
                            output.add(block);
                        }
                        output.advance(UNVOICED_PERIOD);
                        phase -= UNVOICED_PERIOD as i32;
                    }
                    if phase < -32768 {
                        phase += 65536;
                    }
                } else {
                    output.advance(UNVOICED_PERIOD);
                }
            }
            if index + 1 < work.len() {
                output.flush();
                output.reset_pointers();
            }
            continue;
        }
        let group_index = group_of[index];
        let group = &groups[group_index];
        let first_of_group = index == group.segments.start;
        let phrase_first = index == 0 || segments[index - 1].phoneme == 0;
        let mut hold = if first_of_group && group.position != UnitPosition::Initial {
            Hold::Head
        } else {
            Hold::Tail
        };
        let flags = segment.flags();
        if segment.periods > segment.frames.len() as i32 && flags & 64 != 0 && flags & 128 != 0 {
            hold = if first_of_group || phrase_first {
                Hold::Tail
            } else {
                Hold::None
            };
        }
        let voicing: Vec<_> = segment
            .frames
            .clone()
            .map(|frame| {
                !segment.devoiced
                    && segment.pitch >= 0
                    && data.frames[segment.unit_start + frame].voiced
            })
            .collect();
        let scheduled = dsp::schedule(
            &voicing,
            segment.periods as usize,
            FramePolicy {
                phoneme_flags: flags,
                hold,
                phrase_initial: group.position == UnitPosition::Initial,
            },
        );
        for (period, frame_offset) in scheduled.into_iter().enumerate() {
            let mut step = UNVOICED_PERIOD;
            let mut block = None;
            if let Some(frame_offset) = frame_offset {
                let absolute = segment.unit_start + segment.frames.start + frame_offset;
                let frame = data.frames[absolute];
                let envelope = &amplitudes[group_index];
                let amplitude_offset = (segment.frames.start - group.frames.start + frame_offset)
                    .min(envelope.len() - 1);
                let amplitude = amplitude_index(envelope[amplitude_offset]);
                if !frame.voiced || segment.devoiced {
                    if frame.excitable {
                        block = Some(dsp::noise(
                            &data.noise,
                            noise_count % 6,
                            &predictor(data, frame, settings),
                            amplitude,
                            UNVOICED_PERIOD,
                            2048,
                        ));
                        noise_count += 1;
                    }
                } else {
                    let contour = &pitches[group_index];
                    let pitch = if contour.is_empty() {
                        0
                    } else {
                        contour[(pitch_offset[index] as usize + period).min(contour.len() - 1)]
                    };
                    let pitch_period = data.period(pitch, pitch_scale);
                    if pitch_period > 0 {
                        let phoneme = frame_phonemes[absolute].unwrap_or(segment.phoneme);
                        let excitation = if crate::utterance::PHONEME_FLAGS[phoneme] & 132 != 0 {
                            data.pulse.as_slice()
                        } else {
                            data.impulse.as_slice()
                        };
                        block = Some(dsp::excitation(
                            excitation,
                            &predictor(data, frame, settings),
                            amplitude,
                            BLOCK,
                            1564,
                        ));
                        step = pitch_period;
                    }
                }
            }
            phase += UNVOICED_PERIOD as i32;
            while phase > 0 {
                if let Some(block) = &block {
                    output.add(block);
                }
                output.advance(step);
                phase -= step as i32;
            }
            if phase < -32768 {
                phase += 65536;
            }
        }
    }
    output.flush();
    output.samples
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Input<'a>(&'a [u8]);
    impl Input<'_> {
        fn integer(&mut self) -> i32 {
            let value = i32::from_le_bytes(self.0[..4].try_into().unwrap());
            self.0 = &self.0[4..];
            value
        }
    }

    #[test]
    fn all_reference_pcm_from_analyzed_segments() {
        let mut input = Input(include_bytes!("../tests/data/kernel/utterances.bin"));
        let mut count = 0;
        let mut failures = Vec::new();
        while !input.0.is_empty() {
            let name_len = input.integer() as usize;
            let name = std::str::from_utf8(&input.0[..name_len])
                .unwrap()
                .to_owned();
            input.0 = &input.0[name_len..];
            let settings = VoiceSettings {
                pitch: input.integer(),
                accent: input.integer(),
                phrase_accent: input.integer(),
                volume: input.integer(),
                speed: input.integer(),
                fluctuation_depth: input.integer(),
                fluctuation_delay: input.integer(),
                echo_delay: input.integer(),
                spectral: input.integer(),
                echo_depth: input.integer(),
                ring_rate: input.integer(),
            };
            let length = input.integer() as usize;
            let segments: Vec<_> = (0..length)
                .map(|_| Segment {
                    phoneme: input.integer() as usize,
                    periods: input.integer(),
                    original_periods: input.integer(),
                    unit_start: input.integer() as usize,
                    frames: input.integer() as usize..input.integer() as usize,
                    pitch: input.integer(),
                    amplitude: input.integer(),
                    devoiced: input.integer() != 0,
                    tail_periods: input.integer(),
                })
                .collect();
            let expected = std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/data/ref")
                    .join(format!("{name}.pcm")),
            )
            .unwrap();
            let actual = synthesize(&segments, &settings, 11000);
            let expected: Vec<i16> = expected
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes(b.try_into().unwrap()))
                .collect();
            let differences = actual.iter().zip(&expected).filter(|(a, b)| a != b).count()
                + actual.len().abs_diff(expected.len());
            if differences > 0 {
                failures.push(format!(
                    "{name}: {differences} differences; lengths {} / {}",
                    actual.len(),
                    expected.len()
                ));
            }
            count += 1;
        }
        assert_eq!(count, 1248);
        assert!(
            failures.is_empty(),
            "{} of {count} cases failed:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}
