use super::{
    rounded_div,
    tables::{AMPLITUDE, COSINE},
};
use crate::acoustic::ORDER;

// The original multiply discards the low-by-low product. A normal Q16 multiply
// changes the predictor, even when it is rounded in the same direction.
fn multiply_high(left: i32, right: i32) -> i32 {
    let sign = if (left >= 0) == (right >= 0) { 1 } else { -1 };
    let left = i64::from(left).abs();
    let right = i64::from(right).abs();
    let left_high = i64::from((left >> 16) as i16);
    let right_high = i64::from((right >> 16) as i16);
    let cross = (left & 65535) * right_high + left_high * (right & 65535);
    ((left_high * right_high + ((cross + 32768) >> 16)) as i32).wrapping_mul(sign)
}

#[cfg(test)]
fn coefficients(spectrum: &[i32; ORDER]) -> [i16; ORDER] {
    coefficients_with_order(spectrum, ORDER)
}

pub(crate) fn coefficients_with_order(spectrum: &[i32; ORDER], order: usize) -> [i16; ORDER] {
    let cosines: [i32; ORDER] = std::array::from_fn(|k| {
        let phase = (spectrum[k] & 65535) << 7;
        let index = (phase >> 16) as usize;
        let fraction = (phase >> 1) & 32767;
        let start = COSINE[index];
        let end = COSINE[index + 1];
        -i32::from((start + (((end - start) * fraction) >> 15)) as i16)
    });
    let mut even_delay = [0i32; 5];
    let mut odd_delay = [0i32; 5];
    let mut even_previous = [0i32; 5];
    let mut odd_previous = [0i32; 5];
    let mut even = [0i32; 6];
    let mut odd = [0i32; 6];
    let mut previous = 0i32;
    let mut impulse = 0x2000000i32;
    let mut output = [0; ORDER];
    let half = order / 2;
    for step in 0..=order {
        even[0] = impulse.wrapping_add(previous);
        odd[0] = impulse.wrapping_sub(previous);
        previous = impulse;
        for k in 0..half {
            let product = multiply_high(cosines[2 * k].wrapping_shl(16), even_delay[k]);
            even[k + 1] = even_previous[k]
                .wrapping_add(even[k])
                .wrapping_add(product.wrapping_shl(2));
            even_previous[k] = even_delay[k];
            let product = multiply_high(cosines[2 * k + 1].wrapping_shl(16), odd_delay[k]);
            odd[k + 1] = odd_previous[k]
                .wrapping_add(odd[k])
                .wrapping_add(product.wrapping_shl(2));
            odd_previous[k] = odd_delay[k];
            even_delay[k] = even[k];
            odd_delay[k] = odd[k];
        }
        if step > 0 {
            output[step - 1] = ((i64::from(even[half]) + i64::from(odd[half])) >> 15) as i16;
        }
        impulse = 0;
    }
    output
}

pub(crate) fn shift_spectrum(spectrum: &mut [i32; ORDER], shift: i32, source: i32) {
    if shift == 1024 {
        return;
    }
    let pivot = i64::from((spectrum[0] & 65535) >> 2);
    let shift = i64::from(shift);
    for value in spectrum {
        let unsigned = i64::from(*value & 65535);
        let shifted = if source > 100 {
            ((unsigned >> 1) + 32768).min((shift * unsigned) >> 10)
        } else if unsigned < 0x7000 {
            ((((unsigned - pivot) * shift) >> 10) + pivot).max((unsigned << 1) - 65536)
        } else {
            i64::from(*value)
        };
        *value = i32::from(shifted as i16);
    }
}

fn approximate_sqrt(mut value: i64) -> i64 {
    let mut shift = 15i32;
    let mut mask = 0x40000000;
    while value & mask == 0 {
        shift -= 1;
        mask >>= 1;
        if mask <= 1 {
            break;
        }
    }
    if shift & 1 != 0 {
        shift += 1;
    }
    value = if shift > 0 {
        value >> shift
    } else {
        value << -shift
    };
    let correction = 0x28a - ((value * 0x4767) >> 16);
    value += (correction * value) >> 16;
    let result = value + 0x4e0 + 0x4000;
    if shift > 0 {
        result << (shift / 2)
    } else {
        result >> (-shift / 2)
    }
}

fn filter(source: impl Iterator<Item = i16>, coefficients: &[i16; ORDER]) -> Vec<i32> {
    let mut history = [0i32; ORDER];
    source
        .map(|sample| {
            let prediction = history
                .iter()
                .zip(coefficients)
                .fold(0i32, |sum, (&h, &c)| {
                    sum.wrapping_add(((i64::from(h) * i64::from(c)) >> 16) as i32)
                });
            let residual = (i32::from(sample) << 7).wrapping_sub(prediction);
            history.rotate_right(1);
            history[0] = residual.wrapping_shl(5);
            residual >> 7
        })
        .collect()
}

fn normalize(block: &mut [i32], target: i32) {
    let length_factor = match block.len() {
        55 => 59,
        60 => 62,
        120 => 88,
        240 => 124,
        length => (approximate_sqrt(length as i64) << 11) >> 16,
    };
    let mut energy = block.iter().fold(0i32, |sum, &value| {
        sum.wrapping_add(((i64::from(value) * i64::from(value)) >> 6) as i32)
    });
    if energy == 0 {
        energy = 1;
    }
    energy = energy.min(0x3fff0001);
    let gain = rounded_div(
        i64::from(target) * length_factor,
        approximate_sqrt(i64::from(energy)) >> 8,
    ) as i32;
    for value in block {
        *value = (value.wrapping_mul(gain) >> 6).clamp(-32768, 32767);
    }
}

fn scale(block: &mut [i32], amplitude: usize) {
    let factor = i64::from(AMPLITUDE[amplitude]) * 2;
    for value in block {
        *value = ((i64::from(*value) * factor) >> 12).clamp(-32767, 32767) as i32;
    }
}

fn window(block: &mut [i32], start: i64, step: i64) {
    let mut gain = start;
    for value in block {
        *value = ((i64::from(*value) * gain) >> 16) as i32;
        gain += step;
    }
}

pub(crate) fn excitation(
    table: &[i16],
    coefficients: &[i16; ORDER],
    amplitude: usize,
    length: usize,
    target: i32,
) -> Vec<i32> {
    let mut block = filter(
        table
            .iter()
            .copied()
            .chain(std::iter::repeat(0))
            .take(length),
        coefficients,
    );
    normalize(&mut block, target);
    scale(&mut block, amplitude);
    if length > 0 {
        let half = length / 2;
        let step = rounded_div(-65536, half as i64);
        window(&mut block[half..], -(step * (half as i64 - 1)), step);
    }
    block
}

pub(crate) fn noise(
    table: &[i16],
    index: usize,
    coefficients: &[i16; ORDER],
    amplitude: usize,
    length: usize,
    target: i32,
) -> Vec<i32> {
    let source = (0..length).map(|i| table[(length * index + i) % table.len()]);
    let mut block = filter(source, coefficients);
    normalize(&mut block, target);
    scale(&mut block, amplitude);
    if length > 55 {
        let overlap = length - 55;
        let step = rounded_div(65536, overlap as i64);
        window(&mut block[..overlap], 0, step);
        window(&mut block[55..], step * (overlap as i64 - 1), -step);
    }
    block
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acoustic::AcousticData;

    #[test]
    fn every_frame_matches_python_predictor() {
        let data = AcousticData::embedded();
        let expected = include_bytes!("../../tests/data/kernel/coefficients.bin");
        assert_eq!(expected.len(), data.frames.len() * ORDER * 2);
        for (index, (&frame, raw)) in data
            .frames
            .iter()
            .zip(expected.chunks_exact(20))
            .enumerate()
        {
            let expected: [i16; ORDER] =
                std::array::from_fn(|k| i16::from_le_bytes([raw[k * 2], raw[k * 2 + 1]]));
            assert_eq!(
                coefficients(&data.spectrum(frame)),
                expected,
                "frame {index}"
            );
        }
    }

    #[test]
    fn excitation_blocks_match_python_samples() {
        let data = AcousticData::embedded();
        let mut bytes = include_bytes!("../../tests/data/kernel/blocks.bin").as_slice();
        let mut count = 0;
        while !bytes.is_empty() {
            let index = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
            let kind = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
            let length = u16::from_le_bytes(bytes[6..8].try_into().unwrap()) as usize;
            let amplitude = u16::from_le_bytes(bytes[8..10].try_into().unwrap()) as usize;
            bytes = &bytes[10..];
            let expected: Vec<i32> = bytes[..length * 2]
                .chunks_exact(2)
                .map(|b| i32::from(i16::from_le_bytes(b.try_into().unwrap())))
                .collect();
            bytes = &bytes[length * 2..];
            let coef = coefficients(&data.spectrum(data.frames[index]));
            let actual = match kind {
                0 => excitation(&data.pulse, &coef, amplitude, length, 1564),
                1 => excitation(&data.impulse, &coef, amplitude, length, 1564),
                2 => noise(&data.noise, index % 6, &coef, amplitude, length, 2048),
                _ => panic!("invalid fixture kind"),
            };
            assert_eq!(
                actual, expected,
                "frame {index}, kind {kind}, length {length}"
            );
            count += 1;
        }
        assert_eq!(count, 1252);
    }
}
