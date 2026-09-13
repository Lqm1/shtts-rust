use crate::{acoustic::AcousticData, voice::VoiceSettings};

/// Persistent output state, recreated for each utterance.
pub(crate) struct Effects {
    echo: Option<Echo>,
    ring: Option<RingModulation>,
    fluctuation: Option<Fluctuation>,
}

impl Effects {
    pub fn new(settings: &VoiceSettings) -> Self {
        Self {
            echo: (settings.echo_depth > 0 && settings.echo_delay > 0).then_some(Echo {
                gain: (settings.echo_depth << 8) / 100,
                delay: settings.echo_delay as usize,
                history: [0; 1001],
                count: 0,
            }),
            ring: (settings.ring_rate != 0).then(|| RingModulation {
                increment: (i64::from(settings.ring_rate) << 16).div_euclid(11000),
                count: 0,
            }),
            fluctuation: (settings.fluctuation_delay > 0).then(|| Fluctuation {
                depth: (settings.fluctuation_depth as i16).wrapping_shl(8),
                delay: settings.fluctuation_delay as i16,
                state: [0; 4],
                history: [0; 3001],
                count: 0,
            }),
        }
    }

    pub fn process(&mut self, block: &mut [i32]) {
        for sample in block {
            let doubled = *sample * 2;
            *sample = if doubled > 27000 {
                (27000 + ((doubled - 27000) >> 1)).min(32767)
            } else if doubled < -27000 {
                (-27000 + ((doubled + 27000) >> 1)).max(-32767)
            } else {
                doubled
            };
            if let Some(echo) = &mut self.echo {
                *sample = echo.process(*sample as i16);
            }
            if let Some(ring) = &mut self.ring {
                *sample = ring.process(*sample as i16);
            }
            if let Some(fluctuation) = &mut self.fluctuation {
                *sample = fluctuation.process(*sample as i16);
            }
        }
    }
}

struct Echo {
    gain: i32,
    delay: usize,
    history: [i16; 1001],
    count: usize,
}
impl Echo {
    fn process(&mut self, sample: i16) -> i32 {
        let output = if self.count < self.delay {
            i32::from(sample)
        } else {
            let old = i32::from(self.history[(self.count - self.delay) % self.history.len()]);
            (i32::from(sample) + ((self.gain * old) >> 8)).clamp(-32768, 32764)
        };
        self.history[self.count % 1001] = sample;
        self.count += 1;
        output
    }
}

struct RingModulation {
    increment: i64,
    count: i64,
}
impl RingModulation {
    fn process(&mut self, sample: i16) -> i32 {
        self.count += 1;
        let index = (((self.increment * self.count) >> 8) & 255) as usize;
        let sine = i32::from(AcousticData::embedded().ring_sine[index]);
        i32::from(((i32::from(sample) * sine) >> 15) as i16)
    }
}

fn multiply_word_half(word: i32, half: i16) -> i32 {
    ((i64::from(word) * i64::from(half)) >> 16) as i32
}

struct Fluctuation {
    depth: i16,
    delay: i16,
    state: [i32; 4],
    history: [i32; 3001],
    count: usize,
}
impl Fluctuation {
    fn process(&mut self, sample: i16) -> i32 {
        if self.delay <= 0 {
            return i32::from(sample);
        }
        let input = i32::from(sample) << 13;
        let filtered = multiply_word_half(self.state[0], 27574)
            .wrapping_add(multiply_word_half(self.state[1], -11407))
            .wrapping_add(multiply_word_half(input, 2488))
            .wrapping_add(multiply_word_half(self.state[3], -2488));
        self.state = [
            filtered.wrapping_mul(4),
            self.state[0],
            input,
            self.state[2],
        ];
        let old = if self.count >= self.delay as usize {
            self.history[(self.count - self.delay as usize) % 3001]
        } else {
            0
        };
        self.history[self.count % 3001] =
            multiply_word_half(filtered.wrapping_mul(2).wrapping_add(old), self.depth)
                .wrapping_shl(1);
        let value = i32::from(sample).wrapping_add(old >> 12);
        self.count += 1;
        if value > 32764 {
            32764
        } else if value < -32768 {
            -32767
        } else {
            value
        }
    }
}
