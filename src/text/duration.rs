//! Context-dependent duration and amplitude estimates.
use crate::dsp::rounded_div;

mod tables;
use tables::{AMPLITUDES, CONSONANTS, NASALS, VOWELS};

fn correction(table: &[i16], index: i32) -> i32 {
    usize::try_from(index)
        .ok()
        .and_then(|i| table.get(i))
        .map_or(0, |&value| i32::from(value))
}

pub(super) fn scale(value: i32, speed: i32) -> i32 {
    rounded_div(i64::from(value) * i64::from(speed), 100) as i32
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Position {
    pub phrase: i32,
    pub word: i32,
}
impl Position {
    pub fn new(index: usize, total: usize, word_length: usize) -> Self {
        Self {
            phrase: if index == total - 1 {
                2
            } else if index == 0 {
                0
            } else {
                1
            },
            word: if index == 0 && total > 1 {
                0
            } else if index == total - 1 && word_length == 1 && total > 1 {
                2
            } else {
                1
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct VowelContext {
    pub left: i32,
    pub vowel: i32,
    pub right: i32,
    pub position: Position,
}
impl VowelContext {
    pub fn duration(self, speed: i32) -> i32 {
        let sum = correction(&VOWELS.vowel, self.vowel)
            + 4860
            + correction(&VOWELS.left, self.left)
            + correction(&VOWELS.right, self.right)
            + correction(&VOWELS.word, self.position.word)
            + correction(&VOWELS.phrase, self.position.phrase);
        scale((sum >> 6).max(70), speed)
    }
    pub fn nasal_duration(self, speed: i32) -> i32 {
        let sum = correction(&NASALS.left, self.left)
            + 7680
            + correction(&NASALS.right, self.right)
            + correction(&NASALS.word, self.position.word)
            + correction(&NASALS.phrase, self.position.phrase);
        scale(sum >> 6, speed)
    }
    pub fn amplitude(self, mora_count: usize) -> i32 {
        let count_index = match mora_count {
            0..=3 => 0,
            4..=8 => mora_count - 3,
            9..=10 => 6,
            11..=13 => 7,
            14..=16 => 8,
            17..=25 => 9,
            _ => 10,
        };
        let mut sum = correction(&AMPLITUDES.left, self.left)
            + correction(&AMPLITUDES.right, self.right)
            + i32::from(AMPLITUDES.mora_count[count_index])
            + correction(&AMPLITUDES.word, self.position.word)
            + correction(&AMPLITUDES.phrase, self.position.phrase);
        if self.vowel != 5 {
            sum += correction(&AMPLITUDES.vowel, self.vowel);
        }
        600 + ((sum * 1024) >> 16)
    }
}

pub(super) struct ConsonantContext {
    pub group: i32,
    pub category: i32,
    pub previous_phoneme: i32,
    pub vowel: i32,
    pub position: Position,
    pub palatalized: bool,
    pub geminate: bool,
    pub phoneme: usize,
}
impl ConsonantContext {
    pub fn duration(&self, speed: i32) -> i32 {
        let coefficients = &CONSONANTS[self.group as usize];
        let mut sum = i32::from(coefficients.length[0])
            + correction(&coefficients.vowel, self.vowel)
            + i32::from(coefficients.base)
            + correction(&coefficients.previous, self.previous_phoneme)
            + correction(&coefficients.phrase, self.position.phrase)
            + correction(&coefficients.category, self.category)
            + correction(&coefficients.word, self.position.word);
        if self.palatalized {
            sum += match self.category {
                6 => 1651,
                8 => 2573,
                9 => 3648,
                10 => 2560,
                11 => 2246,
                12 => 2176,
                13 => 2995,
                15 => 1760,
                19 => 2624,
                _ => 0,
            };
        }
        if self.geminate {
            sum += i32::from(coefficients.length[1]) - i32::from(coefficients.length[0]);
        }
        let result = (sum >> 6).max(25);
        if (14..27).contains(&self.phoneme) || self.geminate {
            scale(result, speed)
        } else {
            result
        }
    }
}
