use super::{
    AnalysisOptions, Entry, Node,
    accent::Accent,
    duration::{ConsonantContext, Position, VowelContext},
    lexicon::{self, DurationFeatures, Kana},
};
use crate::voice::VoiceSettings;
use std::collections::BTreeSet;

fn phonemes(kana: &[&Kana], index: usize, all: bool) -> Vec<usize> {
    let Some(k) = kana.get(index) else {
        return Vec::new();
    };
    if k.text == "ー" {
        let mut result = vec![27];
        if index > 0 {
            result.extend(phonemes(kana, index - 1, all));
        }
        return result;
    }
    k.moras
        .iter()
        .take(if all { usize::MAX } else { 1 })
        .flat_map(|m| m.phonemes.iter().copied())
        .collect()
}

fn next_vowel(kana: &[&Kana], index: usize, phonemes: &[usize], position: usize) -> i32 {
    let next = kana.get(index + 1);
    if next.is_some_and(|k| k.text == "ー")
        && let Some(&current) = phonemes.get(position)
        && (1..=6).contains(&current)
    {
        return current as i32 - 1;
    }
    let next_phoneme = phonemes.get(position + 1).copied().or_else(|| {
        next.and_then(|k| k.moras.first())
            .and_then(|m| m.phonemes.first())
            .copied()
    });
    match next_phoneme {
        Some(p @ 1..=6) => p as i32 - 1,
        Some(_) => next.and_then(|k| k.features).map_or(12, |f| f.vowel_left),
        None => 12,
    }
}

fn previous_vowel(kana: &[&Kana], index: usize) -> i32 {
    if index == 0 {
        return 26;
    }
    if let Some(&last) = phonemes(kana, index - 1, true).last()
        && (1..=6).contains(&last)
    {
        return last as i32 - 1;
    }
    kana[index - 1].features.map_or(26, |f| f.vowel_kind)
}

fn moraic_pair(kana: &[&Kana]) -> bool {
    if kana.len() < 2 || kana[0].moras.len() > 1 {
        return false;
    }
    if matches!(kana[1].text, "ン" | "ッ" | "ー") {
        return true;
    }
    let first = phonemes(kana, 0, false);
    let second = phonemes(kana, 1, false);
    second.len() == 1
        && first
            .last()
            .is_some_and(|last| (1..=5).contains(last) && Some(last) == second.last())
}

fn entry(phoneme: usize, value: i32, unscaled: i32, amplitude: i32, pitch: i32) -> Entry {
    Entry {
        phoneme,
        duration: value,
        unscaled_duration: unscaled,
        amplitude,
        pitch,
        merged: 1,
        ..Entry::default()
    }
}

struct PhraseContext<'a> {
    kana: Vec<&'static Kana>,
    geminates: BTreeSet<usize>,
    devoiced: BTreeSet<usize>,
    mora_before: Vec<usize>,
    word_lengths: Vec<usize>,
    total: usize,
    initial_geminate: bool,
    nasal: bool,
    settings: &'a VoiceSettings,
    options: &'a AnalysisOptions,
}

#[derive(Default)]
struct Analysis {
    output: Vec<Entry>,
    nodes: Vec<Node>,
    node_kana: Vec<usize>,
    multi_pitch: Vec<(usize, i32)>,
}

pub(super) fn analyze(
    text: &str,
    settings: &VoiceSettings,
    options: &AnalysisOptions,
) -> Vec<Entry> {
    let context = prepare(text, settings, options);
    let mut analysis = Analysis::default();
    for index in 0..context.kana.len() {
        append_kana(index, &context, &mut analysis);
    }
    finish_accent(&context, analysis)
}

fn prepare<'a>(
    text: &str,
    settings: &'a VoiceSettings,
    options: &'a AnalysisOptions,
) -> PhraseContext<'a> {
    let mut kana: Vec<&'static Kana> = Vec::new();
    let mut geminates = BTreeSet::new();
    let mut devoiced = BTreeSet::new();
    for (index, ch) in text.char_indices() {
        if ch == '％' {
            let count = lexicon::split(&text[..index]).len();
            if count > 0 {
                devoiced.insert(count - 1);
            }
        }
    }
    for token in lexicon::split(text) {
        if token.text == "ッ" {
            if !kana.is_empty() {
                geminates.insert(kana.len());
            }
            continue;
        }
        if token.text == "ー"
            && !kana
                .last()
                .and_then(|k| k.moras.first())
                .and_then(|m| m.phonemes.last())
                .is_some_and(|p| (1..=5).contains(p))
        {
            continue;
        }
        kana.push(token);
    }
    let mut mora_before = vec![0];
    for token in &kana {
        mora_before.push(mora_before.last().unwrap() + token.mora_count());
    }
    let total = *mora_before.last().unwrap();
    let mut word_lengths = vec![total; kana.len()];
    let mut previous = 0;
    if !options.word_breaks.is_empty() {
        for bound in options.word_breaks.iter().copied().chain([kana.len()]) {
            let bound = bound.min(kana.len());
            let count = (mora_before[bound] - mora_before[previous]).max(1);
            word_lengths[previous..bound].fill(count);
            previous = bound;
        }
    }
    let initial_geminate =
        geminates.contains(&1) && kana.first().is_none_or(|k| k.mora_count() == 1);
    let nasal = initial_geminate
        || moraic_pair(&kana)
        || kana
            .first()
            .is_some_and(|k| k.text != "ー" && k.phonetic.ends_with('-'));
    PhraseContext {
        kana,
        geminates,
        devoiced,
        mora_before,
        word_lengths,
        total,
        initial_geminate,
        nasal,
        settings,
        options,
    }
}

fn append_kana(index: usize, context: &PhraseContext<'_>, analysis: &mut Analysis) {
    let PhraseContext {
        kana,
        geminates,
        devoiced,
        mora_before,
        word_lengths,
        settings,
        ..
    } = context;
    let total = context.total;
    let token = kana[index];
    let Analysis {
        output,
        nodes,
        node_kana,
        multi_pitch,
    } = analysis;
    let start = output.len();
    let position = Position::new(mora_before[index], total, word_lengths[index]);
    if token.moras.is_empty() {
        if token.text == "ー" && output.last().is_some_and(|e| (1..=5).contains(&e.phoneme)) {
            let last = output.last().unwrap();
            let phoneme = last.phoneme;
            let context = VowelContext {
                left: previous_vowel(kana, index),
                vowel: phoneme as i32 - 1,
                right: next_vowel(kana, index, &[phoneme], 0),
                position,
            };
            let value = context.duration(settings.speed);
            let amplitude = VowelContext {
                left: phoneme as i32 - 1,
                ..context
            }
            .amplitude(total)
            .max(70)
                + settings.volume;
            output.push(entry(
                phoneme,
                value,
                context.duration(100),
                amplitude,
                last.pitch,
            ));
            nodes.push(Node {
                entries: start..output.len(),
                consonant: 0,
                vowel: value,
            });
        }
        node_kana.push(index);
        return;
    }
    let first = &token.moras[0];
    let layout = MoraLayout::new(token);
    append_phonemes(index, context, output, &layout);
    let mut long_start = None;
    if token.long_marker() && output.len() > start {
        let last = output.last().unwrap();
        let phoneme = last.phoneme;
        let context = VowelContext {
            left: phoneme as i32 - 1,
            vowel: phoneme as i32 - 1,
            right: next_vowel(kana, index, &[phoneme], 0),
            position: Position::new(mora_before[index] + 1, total, word_lengths[index]),
        };
        output.push(entry(
            phoneme,
            context.duration(settings.speed),
            context.duration(100),
            last.amplitude,
            last.pitch,
        ));
        long_start = Some(start + first.phonemes.len());
    }
    set_amplitudes(index, context, output, start, long_start, &layout);
    if token.moras.len() > 1 {
        multi_pitch.push((start + first.phonemes.len() - 1, first.pitch));
    }
    let mut devoice_node = None;
    if output.len() > start + 1 && devoiced.contains(&index) {
        let last = output.len() - 1;
        let unscaled = (output[last].unscaled_duration * 22938) >> 15;
        let value = (unscaled * settings.speed + 50).div_euclid(100);
        devoice_node = Some((output[start].duration, value));
        if value > 20 {
            output[start].duration += value - 20;
        }
        output[last].unscaled_duration = unscaled;
        output[last].duration = value.min(20);
        output[last].amplitude = (output[last].amplitude - 30).max(0);
        output[last].devoiced = true;
    }
    if output.len() > start {
        let last = (start + first.phonemes.len() - 1).min(output.len() - 1);
        let (consonant, vowel) = devoice_node.unwrap_or_else(|| {
            (
                if output.len() > start + 1 {
                    output[start].duration
                } else {
                    0
                },
                output[long_start.map_or(last, |s| s - 1)].duration,
            )
        });
        nodes.push(Node {
            entries: start..last + 1,
            consonant,
            vowel,
        });
        node_kana.push(index);
        let mut next = start + first.phonemes.len();
        for mora in token.moras.iter().skip(1) {
            let end = (next + mora.phonemes.len()).min(output.len());
            nodes.push(Node {
                entries: next..end,
                consonant: 0,
                vowel: output[next].duration,
            });
            node_kana.push(index);
            next = end;
        }
        if let Some(start) = long_start {
            nodes.push(Node {
                entries: start..output.len(),
                consonant: 0,
                vowel: output[start].duration,
            });
            node_kana.push(index);
        }
        if geminates.contains(&index) {
            for entry in &mut output[start..] {
                entry.follows_geminate = true;
            }
        }
    }
}

struct MoraLayout {
    ids: Vec<usize>,
    owners: Vec<usize>,
    ends: Vec<usize>,
    features: Option<DurationFeatures>,
}
impl MoraLayout {
    fn new(token: &'static Kana) -> Self {
        let ids: Vec<_> = token
            .moras
            .iter()
            .flat_map(|m| m.phonemes.iter().copied())
            .collect();
        let owners: Vec<_> = token
            .moras
            .iter()
            .enumerate()
            .flat_map(|(i, m)| std::iter::repeat_n(i, m.phonemes.len()))
            .collect();
        let mut ends = Vec::new();
        let mut count = 0;
        for mora in token.moras {
            count += mora.phonemes.len();
            ends.push(count - 1);
        }
        let features = if token.features.is_none() && ids.len() == 1 {
            Some(DurationFeatures::VOWEL)
        } else {
            token.features
        };
        Self {
            ids,
            owners,
            ends,
            features,
        }
    }
}

fn append_phonemes(
    index: usize,
    context: &PhraseContext<'_>,
    output: &mut Vec<Entry>,
    layout: &MoraLayout,
) {
    let PhraseContext {
        kana,
        geminates,
        mora_before,
        word_lengths,
        settings,
        ..
    } = context;
    let total = context.total;
    let token = kana[index];
    let first = &token.moras[0];
    let position = Position::new(mora_before[index], total, word_lengths[index]);
    let MoraLayout {
        ids,
        owners,
        ends,
        features,
    } = layout;
    for (phoneme_index, &phoneme) in ids.iter().enumerate() {
        let mut value = if phoneme_index == 0 && ids.len() > 1 {
            first.consonant_duration
        } else {
            first.vowel_duration
        };
        let mut unscaled = value;
        let mut completed = false;
        let owner_index = owners[phoneme_index];
        let owner = &token.moras[owner_index];
        if phoneme_index >= first.phonemes.len() {
            if let Some(extra) = features.and_then(|f| f.extra_vowels.get(owner_index - 1))
                && phoneme_index == ends[owner_index]
            {
                let context = VowelContext {
                    left: extra.left,
                    vowel: extra.vowel,
                    right: next_vowel(kana, index, ids, phoneme_index),
                    position: Position::new(
                        mora_before[index] + owner_index,
                        total,
                        word_lengths[index],
                    ),
                };
                value = context.duration(settings.speed);
                unscaled = context.duration(100);
                completed = true;
            } else {
                unscaled = owner.vowel_duration;
                value = (unscaled * settings.speed + 50).div_euclid(100);
            }
        }
        if let Some(features) = features
            && !completed
        {
            if phoneme == 6 {
                let context = VowelContext {
                    left: output.last().map_or(26, |e| e.phoneme as i32 - 1),
                    vowel: 5,
                    right: next_vowel(kana, index, ids, phoneme_index),
                    position,
                };
                value = context.nasal_duration(settings.speed);
                unscaled = context.nasal_duration(100);
            } else if phoneme_index == 0 && ids.len() == 1 {
                let context = VowelContext {
                    left: previous_vowel(kana, index),
                    vowel: if (1..=5).contains(&phoneme) {
                        phoneme as i32 - 1
                    } else {
                        features.vowel_kind
                    },
                    right: next_vowel(kana, index, ids, phoneme_index),
                    position,
                };
                value = context.duration(settings.speed);
            } else if phoneme_index == 0 {
                let context = ConsonantContext {
                    group: features.consonant_group,
                    category: features.consonant_category,
                    previous_phoneme: output.last().map_or(26, |e| e.phoneme as i32 - 1),
                    vowel: ids[1] as i32 - 1,
                    position,
                    palatalized: token.consonant().ends_with('y') || token.consonant() == "kw",
                    geminate: geminates.contains(&index),
                    phoneme,
                };
                value = context.duration(settings.speed);
                unscaled = ConsonantContext {
                    geminate: false,
                    ..context
                }
                .duration(100);
                // The unscaled estimate still includes the geminate offset.
                if geminates.contains(&index) {
                    unscaled = context.duration(100);
                }
            } else {
                let vowel = if (1..=5).contains(&phoneme) {
                    phoneme as i32 - 1
                } else {
                    features.vowel_kind
                };
                let next = ids
                    .get(phoneme_index + 1)
                    .copied()
                    .or_else(|| phonemes(kana, index + 1, false).first().copied());
                let right = if token.long_marker()
                    && phoneme_index == ids.len() - 1
                    && (1..=5).contains(&phoneme)
                {
                    phoneme as i32 - 1
                } else if let Some(p @ 1..=6) = next {
                    p as i32 - 1
                } else {
                    next_vowel(kana, index, ids, phoneme_index)
                };
                let context = VowelContext {
                    left: features.vowel_left,
                    vowel,
                    right,
                    position,
                };
                value = context.duration(settings.speed);
                unscaled = context.duration(100);
            }
        }
        output.push(entry(
            phoneme,
            value,
            unscaled,
            owner.amplitude,
            owner.pitch,
        ));
    }
}

fn set_amplitudes(
    index: usize,
    context: &PhraseContext<'_>,
    output: &mut [Entry],
    start: usize,
    long_start: Option<usize>,
    layout: &MoraLayout,
) {
    let PhraseContext {
        kana,
        mora_before,
        word_lengths,
        settings,
        ..
    } = context;
    let total = context.total;
    let token = kana[index];
    let first = &token.moras[0];
    let position = Position::new(mora_before[index], total, word_lengths[index]);
    let MoraLayout {
        ids,
        ends,
        features,
        ..
    } = layout;
    if output.len() > start && (token.moras.len() == 1 || token.long_marker()) {
        let target = if token.long_marker() && long_start.is_some() {
            start + first.phonemes.len() - 1
        } else {
            output.len() - 1
        };
        let phoneme = output[target].phoneme;
        let vowel = if (1..=6).contains(&phoneme) {
            phoneme as i32 - 1
        } else {
            0
        };
        let has_consonant = ids.len() > 1;
        let left = if index == 0 {
            if has_consonant {
                token.amplitude_class.unwrap_or(26)
            } else {
                26
            }
        } else {
            token
                .amplitude_class
                .filter(|_| has_consonant)
                .unwrap_or_else(|| {
                    if start > 0 {
                        output[start - 1].phoneme as i32 - 1
                    } else {
                        26
                    }
                })
        };
        let right = if token.long_marker() && (1..=5).contains(&phoneme) {
            phoneme as i32 - 1
        } else {
            next_vowel(kana, index, ids, ids.len() - 1)
        };
        output[target].amplitude = VowelContext {
            left,
            vowel,
            right,
            position,
        }
        .amplitude(total)
        .max(70)
            + settings.volume;
    } else if output.len() > start
        && token.moras.len() > 1
        && let Some(features) = features
        && !features.extra_vowels.is_empty()
    {
        let mut target = start;
        for (mora_index, mora) in token.moras.iter().enumerate() {
            let extra = features.extra_vowels.get(mora_index.saturating_sub(1));
            if mora_index > 0 && extra.is_none() {
                target += mora.phonemes.len();
                continue;
            }
            let (left, vowel, right) = if mora_index == 0 {
                (
                    features.vowel_left,
                    features.vowel_kind,
                    features.extra_vowels[0].vowel,
                )
            } else {
                (
                    extra.unwrap().left,
                    extra.unwrap().vowel,
                    next_vowel(kana, index, ids, ends[mora_index]),
                )
            };
            let context = VowelContext {
                left,
                vowel,
                right,
                position: Position::new(
                    mora_before[index] + mora_index,
                    total,
                    word_lengths[index],
                ),
            };
            output[target + mora.phonemes.len() - 1].amplitude =
                context.amplitude(total).max(70) + settings.volume;
            target += mora.phonemes.len();
        }
    }
}

fn finish_accent(context: &PhraseContext<'_>, analysis: Analysis) -> Vec<Entry> {
    let PhraseContext {
        kana,
        geminates,
        settings,
        options,
        ..
    } = context;
    let nasal = context.nasal;
    let initial_geminate = context.initial_geminate;
    let Analysis {
        mut output,
        nodes,
        node_kana,
        multi_pitch,
    } = analysis;
    let to_node = |bound: usize| {
        node_kana
            .iter()
            .position(|&k| k >= bound)
            .unwrap_or(node_kana.len())
    };
    let mut nasal_words = Vec::new();
    let mut previous = 0;
    let breaks = if options.accent_groups.is_empty() {
        &options.word_breaks
    } else {
        &options.accent_groups
    };
    for bound in breaks.iter().copied().chain([kana.len()]) {
        let bound = bound.min(kana.len());
        nasal_words.push(
            moraic_pair(&kana[previous..bound])
                || geminates.contains(&(previous + 1))
                || kana
                    .get(previous)
                    .filter(|_| previous < bound)
                    .is_some_and(|k| k.text != "ー" && k.phonetic.ends_with('-')),
        );
        previous = bound;
    }
    let mut words = Vec::new();
    let mut previous = 0;
    if !options.word_breaks.is_empty() {
        for bound in options.word_breaks.iter().copied().chain([nodes.len()]) {
            let bound = to_node(bound);
            if bound > previous {
                words.push(bound - previous);
                previous = bound;
            }
        }
    }
    let mut groups = Vec::new();
    let mut previous = 0;
    if !options.accent_groups.is_empty() {
        for bound in options.accent_groups.iter().copied().chain([nodes.len()]) {
            let bound = to_node(bound);
            if bound > previous {
                groups.push(previous..bound);
                previous = bound;
            }
        }
    }
    let mut accents: Vec<_> = options
        .accents
        .iter()
        .filter(|&&b| b >= 1)
        .map(|&b| to_node(b).saturating_sub(1))
        .collect();
    let mut initial_scale = options.initial_scale;
    if initial_scale == (5, 4) {
        accents.push(to_node(kana.len()).saturating_sub(1));
        initial_scale = (1, 1);
    }
    Accent {
        settings,
        nasal,
        initial_geminate,
        marker: options.marker.unwrap_or(10 * settings.speed),
        initial_scale,
        words,
        accents,
        groups,
        nasal_words,
        duration_override: options.duration_override,
        half_amplitude: options.half_amplitude,
        zero_amplitude: options.zero_amplitude,
        sentence_ending: options.ending.contains(['！', '？']),
    }
    .apply(&mut output, &nodes);
    sentence_ending(&mut output, &nodes, &options.ending);
    for (index, pitch) in multi_pitch {
        if !output[index].accent_pitch {
            output[index].pitch = pitch;
            output[index].secondary_pitch = 0;
        }
    }
    merge_vowels_and_interpolate(output)
}

fn merge_vowels_and_interpolate(output: Vec<Entry>) -> Vec<Entry> {
    let mut merged: Vec<Entry> = Vec::new();
    for current in output {
        if let Some(last) = merged.last_mut()
            && last.phoneme == current.phoneme
            && (1..=5).contains(&current.phoneme)
            && !current.no_merge
            && last.merged.max(1) < 2
        {
            last.duration = (last.duration + current.duration).min(32767);
            last.merged = last.merged.max(1) + 1;
            if current.pitch != 0 {
                last.secondary_pitch = current.pitch;
            }
            continue;
        }
        merged.push(current);
    }
    for index in 1..merged.len().saturating_sub(1) {
        if merged[index].pitch != 0 || merged[index - 1].phoneme == 0 {
            continue;
        }
        let previous = &merged[index - 1];
        let current = &merged[index];
        let next = &merged[index + 1];
        let denominator = next.duration + previous.duration + 2 * current.duration;
        if denominator != 0 {
            let numerator = i64::from(previous.duration + current.duration)
                * i64::from(previous.pitch - next.pitch);
            merged[index].pitch = next.pitch + (numerator / i64::from(denominator)) as i32;
        }
    }
    merged.push(entry(0, 1000, 1000, 0, 0));
    merged.push(entry(7, 0, 0, 0, 0));
    merged
}

fn sentence_ending(entries: &mut Vec<Entry>, nodes: &[Node], ending: &str) {
    let Some(node) = nodes.last() else {
        return;
    };
    if ending.is_empty() {
        return;
    }
    let peak = nodes
        .iter()
        .map(|n| entries[n.entries.end - 1].pitch)
        .max()
        .unwrap_or(0);
    let last = &mut entries[node.entries.end - 1];
    if ending.contains('！') {
        last.duration = last.duration * 4 / 5;
        last.amplitude += 30;
        last.pitch += 100;
    }
    let questions = ending.chars().filter(|&c| c == '？').count().min(3) as i32;
    if questions > 0 {
        let duration = last.duration;
        let amplitude = last.amplitude;
        for _ in 0..questions {
            last.duration = last.duration * 4 / 5;
        }
        let candidate = last.pitch + (peak - last.pitch) / 3;
        let mut duplicate = last.clone();
        duplicate.duration = duration / 2;
        duplicate.amplitude = (amplitude - 90).max(0);
        duplicate.pitch = candidate + 110 * questions;
        duplicate.secondary_pitch = 0;
        duplicate.no_merge = true;
        last.pitch = candidate - 110 * questions;
        entries.push(duplicate);
    }
}
