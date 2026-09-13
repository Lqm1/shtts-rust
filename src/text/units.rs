//! Context-dependent unit lookup and duration allocation.
use super::Entry;
use crate::{
    acoustic::{AcousticData, Unit},
    utterance::Segment,
};
use std::ops::Range;

#[derive(Clone, Copy, Eq, PartialEq)]
enum Boundary {
    Initial,
    Final,
    Medial,
    LeftOnly,
}

fn key(entries: &[Entry], boundary: Boundary, span: Range<usize>) -> [usize; 3] {
    let mut result = [7; 3];
    let mut offset = if boundary == Boundary::Initial {
        result[0] = 0;
        1
    } else {
        0
    };
    for entry in entries.get(span).unwrap_or(&[]) {
        if offset >= 3 {
            break;
        }
        result[offset] = entry.phoneme;
        offset += 1;
    }
    if boundary == Boundary::Final && offset < 3 {
        result[offset] = 0;
    }
    result
}

fn lookup(entries: &[Entry], boundary: Boundary, span: Range<usize>) -> Option<Unit> {
    let [a, b, c] = key(entries, boundary, span);
    AcousticData::embedded().unit(a, b, c)
}

fn emit(
    segments: &mut Vec<Segment>,
    groups: &mut Vec<Range<usize>>,
    entries: &[Entry],
    unit: Unit,
    boundary: Boundary,
    mut span: Range<usize>,
) {
    if boundary == Boundary::LeftOnly {
        span.end = span.end.saturating_sub(1);
    }
    let covered = &entries[span];
    let mut previous = 0;
    let ranges: Vec<_> = unit
        .boundaries
        .iter()
        .filter_map(|&end| {
            let end = usize::from(end);
            if end <= previous {
                return None;
            }
            let range = previous..end;
            previous = end;
            Some(range)
        })
        .collect();
    let group_count = if !ranges.is_empty() && ranges.len() < covered.len() {
        ranges.len()
    } else {
        covered.len()
    };
    if group_count == 0 {
        return;
    }
    let base = covered.len() / group_count;
    let extra = covered.len() % group_count;
    let start = segments.len();
    let mut offset = 0;
    for (index, frames) in ranges.into_iter().take(group_count).enumerate() {
        let size = base + usize::from(index < extra);
        let group = &covered[offset..offset + size];
        offset += size;
        let entry = group.last().unwrap();
        let value: i32 = group.iter().map(|e| e.duration).sum();
        let periods = if index == 0 && boundary != Boundary::Initial {
            ((value + 7) * 3277) >> 15
        } else if index == group_count - 1
            && !matches!(boundary, Boundary::Final | Boundary::LeftOnly)
        {
            ((value + 3) * 3277) >> 15
        } else {
            ((value + 2) * 6554) >> 15
        };
        let periods = periods.max(1);
        let mut pitch = if index == 0 && boundary != Boundary::Initial && entry.secondary_pitch != 0
        {
            entry.secondary_pitch
        } else {
            entry.pitch
        };
        let devoiced = entry.devoiced && (1..=6).contains(&entry.phoneme);
        if devoiced {
            pitch = -pitch;
        }
        segments.push(Segment {
            phoneme: entry.phoneme,
            periods,
            original_periods: periods,
            unit_start: unit.start,
            frames,
            pitch,
            amplitude: entry.amplitude,
            devoiced,
            tail_periods: 200,
        });
    }
    if segments.len() > start {
        groups.push(start..segments.len());
    }
}

pub(super) fn select(mut entries: Vec<Entry>) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut groups = Vec::new();
    let mut previous = 0;
    let mut boundary = Boundary::Initial;
    for current in 0..entries.len() {
        let phoneme = entries[current].phoneme;
        if phoneme == 7 {
            break;
        }
        if phoneme == 0 {
            if current > 0 {
                let last = entries[current - 1].phoneme;
                let left = if (1..=6).contains(&last) { last } else { 0 };
                if let Some(unit) = AcousticData::embedded().unit(left, 0, 7) {
                    emit(
                        &mut segments,
                        &mut groups,
                        &entries,
                        unit,
                        Boundary::Final,
                        previous..current,
                    );
                }
            }
            break;
        }
        if phoneme > 6 {
            continue;
        }
        let abort = boundary != Boundary::Initial
            && previous != current
            && entries[current].follows_geminate
            && [26, 25, 21, 19, 18, 15, 20, 23, 24].contains(&entries[current - 1].phoneme);
        let unit = if abort {
            None
        } else {
            lookup(&entries, boundary, previous..current + 1)
        };
        if let Some(unit) = unit {
            emit(
                &mut segments,
                &mut groups,
                &entries,
                unit,
                boundary,
                previous..current + 1,
            );
        } else if boundary != Boundary::Initial {
            if entries[previous].phoneme == phoneme && previous + 1 == current {
                entries[current].duration =
                    (entries[previous].duration + entries[current].duration).min(32767);
                if let Some(unit) = lookup(&entries, boundary, current..current + 1) {
                    emit(
                        &mut segments,
                        &mut groups,
                        &entries,
                        unit,
                        boundary,
                        current..current + 1,
                    );
                }
            } else {
                let mut left = lookup(&entries, Boundary::LeftOnly, previous..current);
                if left.is_none() {
                    let old = entries[current - 1].phoneme;
                    entries[current - 1].phoneme = match old {
                        18 => 2,
                        20 => 3,
                        16 => 15,
                        _ => old,
                    };
                    left = lookup(&entries, Boundary::LeftOnly, previous..current);
                    entries[current - 1].phoneme = old;
                }
                if let Some(unit) = left {
                    emit(
                        &mut segments,
                        &mut groups,
                        &entries,
                        unit,
                        Boundary::LeftOnly,
                        previous..current,
                    );
                } else if let Some(unit) = lookup(&entries, Boundary::Final, previous..previous + 1)
                {
                    emit(
                        &mut segments,
                        &mut groups,
                        &entries,
                        unit,
                        Boundary::Final,
                        previous..previous + 1,
                    );
                }
                if let Some(unit) = lookup(&entries, Boundary::Initial, current - 1..current + 1) {
                    emit(
                        &mut segments,
                        &mut groups,
                        &entries,
                        unit,
                        Boundary::Initial,
                        current - 1..current + 1,
                    );
                }
            }
        }
        previous = current;
        boundary = Boundary::Medial;
    }
    for range in groups {
        if range.start != 0
            && range.end != segments.len()
            && range.len() > 1
            && segments[range.start].sonorant()
            && segments[range.end - 1].sonorant()
        {
            redistribute(&mut segments[range], true);
        } else {
            redistribute(&mut segments[range], false);
        }
    }
    segments
}

fn short(value: i32) -> i32 {
    i32::from(value as i16)
}

fn redistribute(segments: &mut [Segment], medial: bool) {
    let total = segments.iter().fold(0, |total, s| short(total + s.periods));
    let span = segments.last().unwrap().frames.end as i32 - segments[0].frames.start as i32;
    let special = medial && segments.get(1).is_some_and(|s| s.flags() & 86 == 64);
    let mut carry = 0;
    let mut head = 0;
    let mut allocated = 0;
    let mut index = 0;
    while index < segments.len() {
        let mut periods = segments[index].periods;
        if periods >= 0 {
            while index + 1 < segments.len()
                && segments[index + 1].frames.len() == 1
                && segments[index].flags() & 128 != 0
            {
                index += 1;
                periods = short(periods + segments[index].periods);
            }
            if special {
                let look = (1..=3)
                    .filter_map(|k| segments.get(index + k))
                    .find(|s| s.frames.len() != 1)
                    .map_or(0, |s| s.frames.len() as i32);
                if span > total {
                    if index == 0 && segments.len() > 1 {
                        let length = segments[index].frames.len() as i32;
                        periods = (((periods + segments[index + 1].periods) * length)
                            .div_euclid(length + look)
                            + periods
                            + 1)
                            >> 1;
                        head = periods;
                    } else if index > 0 {
                        if index <= 1 || carry == 0 {
                            carry = periods + segments[index - 1].periods - head;
                            periods = carry;
                        } else {
                            periods = total - head - carry;
                        }
                    }
                } else if index == 0 && segments.len() > 1 {
                    if segments[1].flags() & 64 != 0 {
                        periods = segments[0].periods + ((segments[1].periods - look) >> 1);
                    }
                    head = periods;
                } else if index > 0 {
                    if index <= 1 || carry == 0 {
                        if segments[index].flags() & 64 != 0 {
                            carry = segments[index].frames.len() as i32;
                            periods = carry;
                        } else {
                            periods = total - head;
                        }
                    } else {
                        periods = total - head - carry;
                    }
                }
            }
            if segments[index].frames.is_empty() {
                periods = 0;
            }
            if short(allocated + periods) >= 2559 {
                periods = short(2559 - allocated);
            }
            periods = periods.max(0);
            allocated = short(allocated + periods);
        }
        segments[index].periods = periods;
        index += 1;
    }
}
