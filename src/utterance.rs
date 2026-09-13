//! Typed intermediate representation between linguistic analysis and the vocoder.
use std::ops::Range;

pub(crate) const PHONEME_FLAGS: [u8; 38] = [
    0x00, 0x80, 0x80, 0x80, 0x80, 0x80, 0x84, 0x00, 0x53, 0x59, 0x01, 0x53, 0x53, 0x53, 0x44, 0x58,
    0x58, 0x44, 0x40, 0x40, 0xc0, 0x42, 0x00, 0x48, 0x40, 0x42, 0x42, 0x52, 0x32, 0x24, 0x38, 0x24,
    0x20, 0x22, 0x22, 0x62, 0x32, 0x72,
];

#[derive(Clone, Debug)]
pub(crate) struct Segment {
    pub phoneme: usize,
    pub periods: i32,
    pub original_periods: i32,
    pub unit_start: usize,
    pub frames: Range<usize>,
    pub pitch: i32,
    pub amplitude: i32,
    pub devoiced: bool,
    pub tail_periods: i32,
}

impl Segment {
    pub fn flags(&self) -> u8 {
        PHONEME_FLAGS[self.phoneme]
    }
    pub fn sonorant(&self) -> bool {
        self.phoneme != 20 && self.flags() & 128 != 0
    }
    pub fn pitch(&self) -> i32 {
        if self.devoiced {
            self.pitch.abs()
        } else {
            self.pitch
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnitPosition {
    Initial,
    Final,
    Medial,
    AlternatingFirst,
    AlternatingSecond,
    Silence,
}

#[derive(Debug)]
pub(crate) struct UnitGroup {
    pub segments: Range<usize>,
    pub frames: Range<usize>,
    pub position: UnitPosition,
}

pub(crate) fn group_segments(segments: &[Segment]) -> Vec<UnitGroup> {
    let mut groups: Vec<UnitGroup> = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        if let Some(last) = groups.last_mut()
            && segments[last.segments.start].unit_start == segment.unit_start
            && segment.frames.start == last.frames.end
        {
            last.frames.end = last.frames.end.max(segment.frames.end);
            last.segments.end = index + 1;
            continue;
        }
        groups.push(UnitGroup {
            segments: index..index + 1,
            frames: segment.frames.clone(),
            position: UnitPosition::Silence,
        });
    }
    let mut phrase_start = 0;
    for index in 0..=groups.len() {
        let silence = index == groups.len() || {
            let range = &groups[index].segments;
            segments[range.start].phoneme == 0 && segments[range.end - 1].phoneme == 0
        };
        if !silence {
            continue;
        }
        let phrase_len = index - phrase_start;
        let mut previous = UnitPosition::Medial;
        for (offset, group) in groups[phrase_start..index].iter_mut().enumerate() {
            group.position = if offset == 0 {
                UnitPosition::Initial
            } else if offset == phrase_len - 1 {
                UnitPosition::Final
            } else if group.segments.len() > 1
                && segments[group.segments.start].sonorant()
                && segments[group.segments.end - 1].sonorant()
            {
                UnitPosition::Medial
            } else if previous == UnitPosition::AlternatingFirst {
                UnitPosition::AlternatingSecond
            } else {
                UnitPosition::AlternatingFirst
            };
            previous = group.position;
        }
        phrase_start = index + 1;
    }
    groups
}
