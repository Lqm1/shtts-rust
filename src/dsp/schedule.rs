//! Select acoustic frames when a segment is shortened or lengthened.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Hold {
    Head,
    Tail,
    None,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FramePolicy {
    pub phoneme_flags: u8,
    pub hold: Hold,
    pub phrase_initial: bool,
}

/// `None` is a silent frame inserted before or within an unvoiced onset.
pub(crate) fn schedule(
    voicing: &[bool],
    periods: usize,
    policy: FramePolicy,
) -> Vec<Option<usize>> {
    let length = voicing.len();
    if length == 0 {
        return vec![None; periods];
    }
    if periods == 0 {
        return Vec::new();
    }
    let flags = policy.phoneme_flags;
    if flags & (128 | 4 | 8) == 0 {
        return unvoiced(voicing, periods, flags);
    }
    if periods == length {
        return (0..length).map(Some).collect();
    }
    if periods > length && flags & 128 != 0 {
        match policy.hold {
            Hold::Head => {
                return std::iter::repeat_n(Some(0), periods - length)
                    .chain((0..length).map(Some))
                    .collect();
            }
            Hold::Tail => {
                return (0..length)
                    .map(Some)
                    .chain(std::iter::repeat_n(Some(length - 1), periods - length))
                    .collect();
            }
            Hold::None => {}
        }
    }
    if periods < length
        && flags & 128 != 0
        && policy.phrase_initial
        && length - periods <= periods
        && policy.hold != Hold::None
    {
        let mut result = vec![None; periods];
        let mut remaining = length - periods;
        let mut pointer = if policy.hold == Hold::Head {
            0isize
        } else {
            length as isize - 1
        };
        let direction = if policy.hold == Hold::Head { 1 } else { -1 };
        for iteration in 0..periods {
            let index = if policy.hold == Hold::Head {
                iteration
            } else {
                periods - 1 - iteration
            };
            result[index] = Some(pointer as usize);
            let skip = remaining.min(2);
            remaining -= skip;
            pointer += direction * (1 + skip as isize);
        }
        return result;
    }
    (0..periods)
        .map(|index| Some(index * length / periods))
        .collect()
}

fn unvoiced(voicing: &[bool], periods: usize, flags: u8) -> Vec<Option<usize>> {
    let length = voicing.len();
    let onset = voicing.iter().position(|&v| !v);
    // This bound is deliberately measured from the onset twice. Changing it to
    // scan the entire suffix changes the original engine's onset treatment.
    let unvoiced_count = onset.map_or(0, |start| {
        let end = length - start;
        let mut index = start;
        while index < end && !voicing[index] {
            index += 1;
        }
        index - start
    });
    if periods >= length {
        if periods == length {
            return (0..length).map(Some).collect();
        }
        if flags & 16 == 0 || flags & 2 == 0 {
            return pairs(length, periods, unvoiced_count);
        }
        let prefix = if unvoiced_count > 0 {
            onset.unwrap_or(0)
        } else {
            0
        };
        return (0..prefix)
            .map(Some)
            .chain(std::iter::repeat_n(None, periods - length))
            .chain((prefix..length).map(Some))
            .take(periods)
            .collect();
    }
    let prefix = if unvoiced_count > 0 {
        onset.unwrap_or(0)
    } else {
        0
    };
    (0..prefix)
        .map(Some)
        .chain((prefix + length - periods..).map(|index| Some(index.min(length - 1))))
        .take(periods)
        .collect()
}

fn pairs(length: usize, periods: usize, prefix: usize) -> Vec<Option<usize>> {
    let remaining_frames = length - prefix;
    let extra = periods - length;
    let mut result: Vec<_> = (0..prefix).map(Some).collect();
    let repeats = if remaining_frames >= extra {
        extra
    } else {
        result.extend(std::iter::repeat_n(Some(prefix), extra - remaining_frames));
        remaining_frames
    };
    for index in prefix..prefix + repeats {
        result.extend([Some(index); 2]);
    }
    result.extend((prefix + repeats..length).map(Some));
    result.truncate(periods);
    result
}
