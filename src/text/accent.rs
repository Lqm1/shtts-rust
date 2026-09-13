use super::{Entry, Node};
use crate::{dsp::rounded_div, utterance::PHONEME_FLAGS, voice::VoiceSettings};
use std::ops::Range;

pub(super) struct Accent<'a> {
    pub settings: &'a VoiceSettings,
    pub nasal: bool,
    pub initial_geminate: bool,
    pub marker: i32,
    pub initial_scale: (i32, i32),
    pub words: Vec<usize>,
    pub accents: Vec<usize>,
    pub groups: Vec<Range<usize>>,
    pub nasal_words: Vec<bool>,
    pub duration_override: Option<i32>,
    pub half_amplitude: bool,
    pub zero_amplitude: bool,
    pub sentence_ending: bool,
}

fn curve(position: i32) -> i32 {
    const X: [i32; 5] = [0, 2000, 4000, 16000, 24000];
    const Y: [i32; 5] = [0, 5003, 5886, 1177, 235];
    if position <= 0 {
        return 0;
    }
    for index in 1..5 {
        if position < X[index] {
            let numerator = ((position - X[index - 1]) * (Y[index] - Y[index - 1])) >> 4;
            return (Y[index - 1] >> 4)
                + rounded_div(i64::from(numerator), i64::from(X[index] - X[index - 1])) as i32;
        }
    }
    0
}

fn rate(duration: i32) -> i32 {
    if duration <= 400 {
        6301
    } else {
        4096000 / (duration + 250)
    }
}
fn offset(rate: i32, node: &Node) -> i32 {
    rounded_div(1024000, i64::from(rate)) as i32 - 83 - node.consonant - ((node.vowel + 1) >> 1)
}
fn modulation(amplitude: i32, curve: i32) -> i32 {
    (amplitude * 3 * curve + 1024) >> 11
}

impl Accent<'_> {
    pub fn apply(&self, entries: &mut [Entry], nodes: &[Node]) {
        if entries.is_empty() || nodes.is_empty() {
            return;
        }
        for (index, entry) in entries.iter_mut().enumerate() {
            if PHONEME_FLAGS[entry.phoneme] & 128 == 0 || entry.phoneme == 20 {
                entry.amplitude = 0;
                if index == 0 {
                    entry.pitch = 0;
                }
            }
        }
        let duration = self
            .duration_override
            .unwrap_or_else(|| entries.iter().map(|e| e.duration).sum());
        let phrase_rate = rate(duration);
        let first_mod = (10 * self.settings.accent * 655 + 32768) >> 16;
        let next_mod = (10 * self.settings.phrase_accent * 655 + 32768) >> 16;
        let initial = first_mod * (if self.nasal { 33974 } else { 37749 }) * self.initial_scale.0
            / self.initial_scale.1;
        let effective_mod = if self.zero_amplitude {
            0
        } else if self.half_amplitude {
            next_mod >> 1
        } else {
            next_mod
        };
        let amplitude = 105 * effective_mod;
        let initial_pitch = self.initial_pitches(nodes, entries, first_mod, initial);
        let mut second_pass = vec![0; nodes.len()];
        if !self.groups.is_empty() {
            let total: i32 = entries.iter().map(|e| e.duration).sum();
            let second_rate = if total <= 1080 {
                2662
            } else {
                3532800 / (total + 250)
            };
            let second_offset = offset(second_rate, &nodes[0]);
            let mut time = 0;
            for (index, node) in nodes.iter().enumerate() {
                time += node.consonant + ((node.vowel + 1) >> 1);
                second_pass[index] = modulation(
                    amplitude,
                    curve((second_rate * (time + second_offset)) >> 8),
                );
                time += node.vowel >> 1;
            }
        }
        let mut rates = vec![phrase_rate; nodes.len()];
        let mut offsets = vec![offset(phrase_rate, &nodes[0]); nodes.len()];
        for group in &self.groups {
            let start = nodes[group.start].entries.start;
            let end = nodes[group.end - 1].entries.end;
            let duration = entries[start..end].iter().map(|e| e.duration).sum();
            let group_rate = rate(duration);
            for index in group.clone() {
                rates[index] = group_rate;
                offsets[index] = offset(group_rate, &nodes[group.start]);
            }
        }
        let mut time = 0;
        let mut pitches = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            if !self.groups.is_empty()
                && rates[index] != phrase_rate
                && (index == 0 || rates[index - 1] != rates[index])
            {
                time = 0;
            }
            time += node.consonant + ((node.vowel + 1) >> 1);
            let value = curve((rates[index] * (time + offsets[index])) >> 8);
            let mut variation = modulation(amplitude, value);
            if !self.groups.is_empty() {
                variation = second_pass[index];
                if let Some((group_index, group)) = self
                    .groups
                    .iter()
                    .enumerate()
                    .find(|(_, g)| g.contains(&index))
                    && group_index >= 1
                {
                    let first = initial_pitch[group.start] + second_pass[group.start];
                    let amplitude = (105 * (next_mod >> 1) - first).max(0);
                    variation += modulation(amplitude, value);
                }
            }
            pitches.push(self.settings.pitch + initial_pitch[index] + variation);
            time += node.vowel >> 1;
        }
        let lowering = effective_mod / 10 * 170;
        if !self.sentence_ending
            && self.groups.is_empty()
            && (self.settings.speed - 1) * 10 < self.marker
            && self.marker < (self.settings.speed + 1) * 10
            && self.settings.phrase_accent >= 100
            && lowering != 0
        {
            *pitches.last_mut().unwrap() -= lowering;
        }
        for (node, pitch) in nodes.iter().zip(pitches) {
            let last = &mut entries[node.entries.end - 1];
            last.pitch = pitch;
            last.accent_pitch = true;
            if node.entries.len() > 1 {
                entries[node.entries.start].pitch = 0;
            }
        }
        if PHONEME_FLAGS[entries[0].phoneme] & 128 == 0 {
            entries[0].pitch = 0;
        }
    }

    fn initial_pitches(
        &self,
        nodes: &[Node],
        entries: &[Entry],
        first_mod: i32,
        initial: i32,
    ) -> Vec<i32> {
        let mut words = Vec::new();
        let mut total = 0;
        for &word in &self.words {
            let word = word.min(nodes.len() - total);
            if word > 0 {
                words.push(word);
                total += word;
            }
        }
        if total < nodes.len() {
            words.push(nodes.len() - total);
        }
        let mut output = Vec::new();
        let mut previous_marker = None;
        let mut previous_initial = 0;
        let mut previous_length = 0;
        let mut previous_accent = false;
        for (word_index, &length) in words.iter().enumerate() {
            let start = output.len();
            let marker = if word_index + 1 == words.len() {
                self.marker
            } else {
                0
            };
            let base = if word_index == 0 {
                initial
            } else if let Some(&nasal) = self.nasal_words.get(word_index) {
                first_mod * (if nasal { 33974 } else { 37749 }) * self.initial_scale.0
                    / self.initial_scale.1
            } else if self.nasal {
                first_mod * 37749
            } else {
                initial
            };
            let word_initial = if marker > 3 * self.settings.speed {
                ((i64::from(base) * 1638) >> 12) as i32
            } else {
                base
            };
            let accent = (0..length).find(|i| self.accents.contains(&(start + i)));
            let carried = if previous_marker == Some(0) && previous_length > 1 && !previous_accent {
                (previous_initial + 2048) >> 12
            } else {
                0
            };
            if let Some(accent) = accent {
                let next_nasal = self.nasal
                    && nodes
                        .get(accent + 1)
                        .is_some_and(|n| entries[n.entries.clone()].iter().any(|e| e.phoneme == 6));
                let accent_base = if next_nasal {
                    first_mod * 37749
                } else {
                    initial
                };
                let distance = length - accent;
                if accent == 0 {
                    let mut base = if marker <= 3 * self.settings.speed || length >= 3 {
                        accent_base
                    } else {
                        accent_base / 4
                    };
                    base -= base >> 2;
                    for index in 0..length {
                        output.push(if index == 0 {
                            (base + 512) >> 10
                        } else if length >= 3 && index == 1 {
                            (base + 1024) >> 11
                        } else {
                            0
                        });
                    }
                } else {
                    let base = if marker <= 3 * self.settings.speed || distance >= 3 {
                        accent_base
                    } else {
                        ((i64::from(accent_base) * 5 / 4 * 1638) >> 12) as i32
                    };
                    for index in 0..length {
                        output.push(if index == 0 {
                            carried
                        } else if index <= accent && index == 1 && !self.initial_geminate {
                            (base * 3 + 2048) >> 12
                        } else if index <= accent {
                            (base + 512) >> 10
                        } else if distance >= 3 && index == accent + 1 {
                            (base + 1024) >> 11
                        } else {
                            0
                        });
                    }
                }
            } else {
                for index in 0..length {
                    output.push(if index == 0 {
                        carried
                    } else if index == 1 && !self.initial_geminate {
                        (word_initial * 3 + 2048) >> 12
                    } else {
                        (word_initial + 512) >> 10
                    });
                }
            }
            previous_marker = Some(marker);
            previous_initial = word_initial;
            previous_length = length;
            previous_accent = accent.is_some_and(|a| a != length - 1);
        }
        output
    }
}
