//! Kana tokenization, linguistic analysis and acoustic unit selection.
mod accent;
mod analyze;
mod duration;
mod lexicon;
mod lexicon_data;
mod parse;
mod units;

#[derive(Clone, Debug)]
struct AnalysisOptions {
    marker: Option<i32>,
    ending: String,
    initial_scale: (i32, i32),
    word_breaks: Vec<usize>,
    accents: Vec<usize>,
    accent_groups: Vec<usize>,
    duration_override: Option<i32>,
    half_amplitude: bool,
    zero_amplitude: bool,
}
impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            marker: None,
            ending: String::new(),
            initial_scale: (1, 1),
            word_breaks: Vec::new(),
            accents: Vec::new(),
            accent_groups: Vec::new(),
            duration_override: None,
            half_amplitude: false,
            zero_amplitude: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Entry {
    phoneme: usize,
    duration: i32,
    unscaled_duration: i32,
    amplitude: i32,
    pitch: i32,
    secondary_pitch: i32,
    devoiced: bool,
    follows_geminate: bool,
    accent_pitch: bool,
    no_merge: bool,
    merged: usize,
}

#[derive(Clone, Debug)]
struct Node {
    entries: std::ops::Range<usize>,
    consonant: i32,
    vowel: i32,
}

use crate::{utterance::Segment, voice::VoiceSettings};

fn silence(periods: i32) -> Segment {
    Segment {
        phoneme: 0,
        periods,
        original_periods: periods,
        unit_start: 0,
        frames: 0..1,
        pitch: 0,
        amplitude: 0,
        devoiced: false,
        tail_periods: 200,
    }
}

pub(crate) fn analyze(text: &str, settings: &VoiceSettings) -> Vec<Segment> {
    let mut phrases = parse::parse(text);
    if phrases.is_empty() {
        phrases.push(parse::Phrase::default());
    }
    if phrases.len() == 1 {
        let p = &phrases[0];
        if p.pauses == 0
            && p.endings.is_empty()
            && p.trailing_breaks == 0
            && p.initial_mark.is_none()
            && p.word_breaks.is_empty()
            && p.accents.is_empty()
        {
            return phrase(text, settings, &AnalysisOptions::default());
        }
    }
    let mut output = Vec::new();
    for (index, p) in phrases.iter().enumerate() {
        let last = index + 1 == phrases.len();
        let marker = if !p.endings.is_empty() {
            10 * settings.speed + 2 * settings.speed * p.absorbed_pauses
        } else if p.trailing_breaks > 0 {
            if p.pauses > 0 {
                2 * settings.speed * p.pauses
            } else {
                settings.speed
            }
        } else if p.pauses > 0 {
            2 * settings.speed * p.pauses
        } else {
            10 * settings.speed
        };
        let initial_scale = if last {
            match p.initial_mark {
                Some('２') => (3, 2),
                Some('０') => (1, 2),
                Some('’') if p.pauses == 0 => (5, 4),
                _ => (1, 1),
            }
        } else {
            (1, 1)
        };
        let options = AnalysisOptions {
            marker: Some(marker),
            ending: p.endings.clone(),
            initial_scale,
            word_breaks: p.word_breaks.clone(),
            accents: p.accents.clone(),
            accent_groups: p.accent_groups.clone(),
            ..AnalysisOptions::default()
        };
        output.extend(phrase(&p.text, settings, &options));
        if !last {
            let periods = if !p.endings.is_empty() {
                200 + 40 * p.absorbed_pauses
            } else if p.trailing_breaks > 0 {
                if p.pauses > 0 { 40 * p.pauses } else { 20 }
            } else {
                40 * p.pauses
            };
            output.push(silence((periods * settings.speed + 50).div_euclid(100)));
        }
    }
    if let Some(last) = output.last_mut() {
        let p = phrases.last().unwrap();
        last.tail_periods = if !p.endings.is_empty() {
            200 + 40 * p.absorbed_pauses
        } else if p.pauses > 0 {
            40 * p.pauses
        } else if p.trailing_breaks > 0 {
            20
        } else {
            200
        };
    }
    output
}

fn phrase(text: &str, settings: &VoiceSettings, options: &AnalysisOptions) -> Vec<Segment> {
    let bars: Vec<_> = options
        .accent_groups
        .iter()
        .chain(&options.word_breaks)
        .copied()
        .collect();
    let parts = parse::split_chunks(text, &bars);
    if parts.len() <= 1 {
        return units::select(analyze::analyze(text, settings, options));
    }
    let sums: Vec<i32> = parts
        .iter()
        .enumerate()
        .map(|(index, part)| {
            let mut opts = AnalysisOptions {
                initial_scale: options.initial_scale,
                ..AnalysisOptions::default()
            };
            if index + 1 == parts.len() {
                opts.marker = options.marker;
                opts.ending = options.ending.clone();
            } else {
                opts.marker = Some(settings.speed);
            }
            analyze::analyze(part, settings, &opts)
                .iter()
                .filter(|e| !matches!(e.phoneme, 0 | 7))
                .map(|e| e.duration)
                .sum()
        })
        .collect();
    let mut output = Vec::new();
    let mut offset = 0;
    let mut later_group = false;
    let mut later_word = false;
    for (index, part) in parts.iter().enumerate() {
        let count = parse::kana_count(part);
        let relative = |values: &[usize]| {
            values
                .iter()
                .filter_map(|&v| v.checked_sub(offset).filter(|&v| v > 0 && v <= count))
                .collect::<Vec<_>>()
        };
        let mut opts = options.clone();
        opts.word_breaks = relative(&options.word_breaks);
        opts.accents = relative(&options.accents);
        opts.accent_groups = relative(&options.accent_groups);
        opts.half_amplitude = later_group && !later_word;
        opts.zero_amplitude = later_word;
        opts.duration_override = Some(sums[index]);
        if index + 1 < parts.len() {
            opts.marker = Some(settings.speed);
            opts.ending.clear();
        }
        let sub = units::select(analyze::analyze(part, settings, &opts));
        if !opts.accent_groups.is_empty() {
            later_group = true;
        }
        let slash: Vec<_> = options
            .word_breaks
            .iter()
            .filter(|b| !options.accent_groups.contains(b))
            .copied()
            .collect();
        if !relative(&slash).is_empty() {
            later_word = true;
        }
        if index > 0 && !output.is_empty() {
            output.push(silence((20 * settings.speed + 50).div_euclid(100)));
        }
        output.extend(sub);
        offset += count;
    }
    if let Some(last) = output.last_mut() {
        last.tail_periods = 200;
    }
    output
}
