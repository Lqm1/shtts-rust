//! Per-period pitch and per-frame amplitude envelopes.
use crate::{
    acoustic::AcousticData,
    dsp::{pitch_div, rounded_div},
    utterance::{Segment, UnitGroup, UnitPosition},
    voice::VoiceSettings,
};

fn ramp(previous: i32, target: i32, count: i32) -> Vec<i32> {
    let step = pitch_div((target - previous) << 6, count);
    (1..=count)
        .map(|index| previous + ((index * step) >> 6))
        .collect()
}

fn split_ramp(start: i32, middle: i32, target: i32, lead: i32, tail: i32) -> Vec<i32> {
    let mut result = ramp(start, middle, lead);
    result.extend(ramp(result.last().copied().unwrap_or(start), target, tail));
    result
}

pub(crate) fn pitches(
    segments: &[Segment],
    groups: &[UnitGroup],
    settings: &VoiceSettings,
) -> Vec<Vec<i32>> {
    let mut output = Vec::with_capacity(groups.len());
    let mut previous = 0;
    let mut pending = false;
    let modulated = (10 * settings.phrase_accent * 655 + 32768) >> 16 != 0;
    for group in groups {
        let range = group.segments.clone();
        let first = &segments[range.start];
        let last = &segments[range.end - 1];
        let count: i32 = segments[range.clone()].iter().map(|s| s.periods).sum();
        if group.position == UnitPosition::Silence || count <= 0 {
            if group.position == UnitPosition::Silence {
                pending = false;
            }
            output.push(Vec::new());
            continue;
        }
        let current = last.pitch();
        let carry = if range.start > 0 && segments[range.start - 1].phoneme == 0 {
            segments[range.start - 1].periods
        } else {
            0
        };
        let next = segments.get(range.end).filter(|s| s.periods > 0);
        let next_pitch = next.map_or(0, |s| if s.devoiced { 0 } else { s.pitch() });
        let next_count = next.map_or(0, |s| s.original_periods);
        let interpolate = next.is_some_and(|s| s.phoneme == last.phoneme && next_pitch != current);
        let half = last.periods / 2;
        let lead = count - half;
        let target = current
            + pitch_div(
                last.periods * (next_pitch - current),
                last.periods + next_count,
            );
        let result = if pending && group.position == UnitPosition::Medial {
            pending = false;
            let half = (first.periods + 1) / 2;
            split_ramp(previous, first.pitch(), current, half, count - half)
        } else if group.position == UnitPosition::Initial {
            let slope = if modulated { 15 } else { 0 };
            if carry != 0 {
                if interpolate {
                    pending = true;
                    let start = previous + pitch_div(carry * (current - previous), carry + lead);
                    split_ramp(start, current, target, lead, half)
                } else {
                    let start = previous + pitch_div(carry * (current - previous), carry + count);
                    ramp(start, current, count)
                }
            } else if interpolate {
                pending = true;
                let start = if first.flags() & 1 != 0 {
                    current
                } else {
                    current - lead * slope
                };
                split_ramp(start, current, target, lead, half)
            } else {
                let start = if first.flags() & 1 != 0 {
                    current
                } else {
                    current - count * slope
                };
                ramp(start, current, count)
            }
        } else if group.position == UnitPosition::Final {
            ramp(
                previous,
                previous - count * if modulated { 10 } else { 0 },
                count,
            )
        } else if interpolate {
            pending = true;
            split_ramp(previous, current, target, lead, half)
        } else {
            if previous <= 0 {
                previous = first.pitch();
            }
            ramp(previous, current, count)
        };
        previous = result.last().copied().unwrap_or(previous);
        output.push(result);
    }
    output
}

pub(crate) fn amplitudes(segments: &[Segment], group: &UnitGroup, data: &AcousticData) -> Vec<i32> {
    let first = &segments[group.segments.start];
    let last = &segments[group.segments.end - 1];
    let base = first.unit_start + group.frames.start;
    let length = group.frames.len().max(1).min(data.frames.len() - base);
    let mut amplitudes: Vec<_> = data.frames[base..base + length]
        .iter()
        .map(|f| f.amplitude)
        .collect();
    let mut voiced = vec![false; length];
    for segment in &segments[group.segments.clone()] {
        for offset in segment.frames.clone() {
            if let Some(value) = voiced.get_mut(offset - group.frames.start) {
                *value = !segment.devoiced
                    && segment.pitch >= 0
                    && data.frames[segment.unit_start + offset].voiced;
            }
        }
    }
    let head = amplitudes[0];
    let tail = amplitudes[length - 1];
    let (start_delta, end_delta) = if group.position == UnitPosition::Initial
        || group.position == UnitPosition::AlternatingSecond
    {
        (last.amplitude - tail, last.amplitude - tail)
    } else if group.position != UnitPosition::Final
        && group.segments.len() > 1
        && first.sonorant()
        && last.sonorant()
    {
        (first.amplitude - head, last.amplitude - tail)
    } else {
        (first.amplitude - head, first.amplitude - head)
    };
    amplitude_ramp(&mut amplitudes, &voiced, start_delta, end_delta);
    for segment in &segments[group.segments.clone()] {
        let flags = segment.flags();
        if segment.phoneme == 0
            || segment.periods <= segment.frames.len() as i32
            || flags & (128 | 4 | 8) != 0
            || flags & 16 == 0
            || flags & 2 == 0
        {
            continue;
        }
        let voicing: Vec<_> = segment
            .frames
            .clone()
            .map(|k| {
                !segment.devoiced
                    && segment.pitch >= 0
                    && data.frames[segment.unit_start + k].voiced
            })
            .collect();
        let Some(start) = voicing.iter().position(|&v| !v) else {
            continue;
        };
        let mut end = start;
        if start < voicing.len() - start {
            while !voicing[end] {
                end += 1;
                if end >= voicing.len() - start {
                    break;
                }
            }
        }
        if end - start < 6 {
            continue;
        }
        for offset in 0..6 {
            if let Some(value) =
                amplitudes.get_mut(segment.frames.start - group.frames.start + start + offset)
            {
                *value = (*value - (6 - offset) as i32 * 40).max(0);
            }
        }
    }
    amplitudes
}

fn amplitude_ramp(amplitudes: &mut [i32], voiced: &[bool], start: i32, end: i32) {
    let start = start.clamp(-400, 400);
    let end = end.clamp(-400, 400);
    let limit = (amplitudes[0] + start).max(amplitudes[amplitudes.len() - 1] + end);
    let increment = if amplitudes.len() > 1 {
        (rounded_div(i64::from(end - start) << 11, amplitudes.len() as i64 - 1) << 5)
            .clamp(-6553600, 6553600)
    } else {
        0
    };
    let mut delta = i64::from(end) << 16;
    let mut voiced_delta = delta;
    for (value, &is_voiced) in amplitudes.iter_mut().zip(voiced).rev() {
        *value = (*value + ((if is_voiced { delta } else { voiced_delta }) >> 16) as i32)
            .max(0)
            .min(limit);
        delta -= increment;
        if is_voiced {
            voiced_delta = delta;
        }
    }
}
