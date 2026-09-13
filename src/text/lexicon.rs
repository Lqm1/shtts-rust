use super::lexicon_data::KANA;

#[derive(Debug)]
pub(super) struct Mora {
    pub phonemes: &'static [usize],
    pub consonant_duration: i32,
    pub vowel_duration: i32,
    pub amplitude: i32,
    pub pitch: i32,
}

#[derive(Debug)]
pub(super) struct Kana {
    pub text: &'static str,
    pub phonetic: &'static str,
    pub features: Option<DurationFeatures>,
    pub amplitude_class: Option<i32>,
    pub moras: &'static [Mora],
}

#[derive(Clone, Copy, Debug)]
pub(super) struct VowelFeature {
    pub left: i32,
    pub vowel: i32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DurationFeatures {
    pub consonant_group: i32,
    pub consonant_category: i32,
    pub vowel_left: i32,
    pub vowel_kind: i32,
    pub extra_vowels: &'static [VowelFeature],
}

impl DurationFeatures {
    pub const VOWEL: Self = Self {
        consonant_group: 26,
        consonant_category: 26,
        vowel_left: 26,
        vowel_kind: 0,
        extra_vowels: &[],
    };
}

impl Kana {
    pub fn mora_count(&self) -> usize {
        self.moras.len().max(1) + usize::from(self.long_marker())
    }
    pub fn long_marker(&self) -> bool {
        self.text.chars().count() > 1
            && self.phonetic.ends_with('-')
            && self
                .moras
                .last()
                .is_some_and(|m| m.phonemes.last().is_some_and(|p| (1..=5).contains(p)))
    }
    pub fn consonant(&self) -> &str {
        self.phonetic
            .trim_end_matches('-')
            .trim_end_matches(['a', 'i', 'u', 'e', 'o'])
    }
}

pub(super) fn lookup(text: &str) -> Option<&'static Kana> {
    KANA.binary_search_by_key(&text, |k| k.text)
        .ok()
        .map(|index| &KANA[index])
}

pub(super) fn split(mut text: &str) -> Vec<&'static Kana> {
    let mut result = Vec::new();
    while !text.is_empty() {
        let boundaries: Vec<_> = text.char_indices().map(|(i, _)| i).take(3).collect();
        let single = boundaries.get(1).copied().unwrap_or(text.len());
        let double = boundaries.get(2).copied().unwrap_or(text.len());
        if let Some(kana) = lookup(&text[..double]).or_else(|| lookup(&text[..single])) {
            result.push(kana);
            text = &text[kana.text.len()..];
        } else {
            text = &text[single..];
        }
    }
    result
}
