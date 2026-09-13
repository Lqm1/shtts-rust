use super::lexicon;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Phrase {
    pub text: String,
    pub pauses: i32,
    pub endings: String,
    pub absorbed_pauses: i32,
    pub trailing_breaks: i32,
    pub initial_mark: Option<char>,
    pub word_breaks: Vec<usize>,
    pub accent_groups: Vec<usize>,
    pub accents: Vec<usize>,
}

fn flush(current: &mut String, phrases: &mut Vec<Phrase>, join: &mut bool) {
    if current.is_empty() {
        return;
    }
    if *join && !phrases.is_empty() {
        phrases.last_mut().unwrap().text.push_str(current);
        *join = false;
    } else {
        phrases.push(Phrase {
            text: current.clone(),
            ..Phrase::default()
        });
    }
    current.clear();
}

pub(super) fn kana_count(text: &str) -> usize {
    lexicon::split(text)
        .iter()
        .filter(|k| k.text != "ッ")
        .count()
}

pub(super) fn parse(text: &str) -> Vec<Phrase> {
    let mut phrases: Vec<Phrase> = Vec::new();
    let mut current = String::new();
    let mut join = false;
    let mut after_slash = false;
    for (index, ch) in text.char_indices() {
        let suffix = &text[index + ch.len_utf8()..];
        let only_markers = |allowed: &str| suffix.chars().all(|c| allowed.contains(c));
        match ch {
            '＿' => {
                if after_slash {
                    after_slash = false;
                    continue;
                }
                flush(&mut current, &mut phrases, &mut join);
                join = false;
                if let Some(last) = phrases.last_mut() {
                    last.pauses += 1;
                }
            }
            '／' | '｜' if only_markers("＿／｜．！？") => {
                flush(&mut current, &mut phrases, &mut join);
                if let Some(last) = phrases.last_mut() {
                    last.trailing_breaks += 1;
                }
                after_slash = true;
            }
            '／' | '｜' => {
                if phrases.is_empty() {
                    phrases.push(Phrase::default());
                }
                let last = phrases.last_mut().unwrap();
                last.text.push_str(&current);
                current.clear();
                let count = kana_count(&last.text);
                last.word_breaks.push(count);
                if ch == '｜' {
                    last.accent_groups.push(count);
                }
                join = true;
                after_slash = true;
            }
            '’' | '２' | '０' if only_markers("＿／｜．！？２０’") => {
                flush(&mut current, &mut phrases, &mut join);
                if let Some(last) = phrases.last_mut() {
                    last.initial_mark = Some(ch);
                }
            }
            '’' => {
                if !current.is_empty() {
                    if phrases.is_empty() {
                        phrases.push(Phrase::default());
                    }
                    phrases.last_mut().unwrap().text.push_str(&current);
                    current.clear();
                }
                // A leading accent marker has no preceding mora to mark.
                if let Some(last) = phrases.last_mut() {
                    last.accents.push(kana_count(&last.text));
                }
                join = true;
            }
            '．' | '！' | '？' => {
                flush(&mut current, &mut phrases, &mut join);
                if let Some(last) = phrases.last_mut() {
                    if last.endings.is_empty() {
                        last.absorbed_pauses = std::mem::take(&mut last.pauses);
                    }
                    last.endings.push(ch);
                }
            }
            _ => current.push(ch),
        }
    }
    flush(&mut current, &mut phrases, &mut join);
    let length = phrases.len();
    for phrase in phrases.iter_mut().take(length.saturating_sub(1)) {
        phrase.trailing_breaks = 0;
        phrase.initial_mark = None;
    }
    phrases
}

fn prefix(text: &str) -> Option<&'static lexicon::Kana> {
    let single = text.chars().next()?.len_utf8();
    let double = single + text[single..].chars().next().map_or(0, char::len_utf8);
    lexicon::lookup(&text[..double]).or_else(|| lexicon::lookup(&text[..single]))
}

pub(super) fn split_chunks<'a>(text: &'a str, bars: &[usize]) -> Vec<&'a str> {
    const LIMIT: usize = 32;
    let mut parts = Vec::new();
    let mut rest = text;
    let mut done = 0;
    while !rest.is_empty() {
        let mut bounds: Vec<(usize, usize, usize, Option<&lexicon::Kana>)> = Vec::new();
        let mut offset = 0;
        let mut moras = 0;
        let mut kana_index = 0;
        while offset < rest.len() {
            let Some(kana) = prefix(&rest[offset..]) else {
                offset += rest[offset..].chars().next().unwrap().len_utf8();
                continue;
            };
            let count = if kana.text == "ッ" {
                1
            } else {
                kana.mora_count()
            };
            if bars.iter().any(|&b| b > done && b - done == kana_index) {
                moras = 0;
            }
            offset += kana.text.len();
            moras += count;
            bounds.push((offset, moras, count, Some(kana)));
            kana_index += 1;
            if moras > LIMIT {
                break;
            }
        }
        let cut = bounds.iter().position(|b| b.1 > LIMIT);
        let end = if let Some(cut) = cut {
            if cut == 0 {
                break;
            }
            let mut cut = cut - 1;
            while cut > 0
                && bounds[cut].1 == LIMIT
                && bounds[cut].2 == 1
                && bounds[cut]
                    .3
                    .is_some_and(|k| k.moras.first().is_some_and(|m| m.phonemes.len() > 1))
                && (bounds[cut - 1].2 == 2
                    || bounds[cut - 1]
                        .3
                        .is_some_and(|k| matches!(k.text, "ッ" | "ン")))
            {
                cut -= 1;
            }
            bounds[cut].0
        } else {
            rest.len()
        };
        if end == 0 {
            break;
        }
        let (head, tail) = rest.split_at(end);
        parts.push(head);
        done += kana_count(head);
        rest = tail;
    }
    if parts.len() > 1 {
        return parts;
    }
    let mut parts = Vec::new();
    let mut start = 0;
    let mut offset = 0;
    let mut count = 0;
    while offset < text.len() {
        if let Some(kana) = prefix(&text[offset..]) {
            if count >= LIMIT {
                parts.push(&text[start..offset]);
                start = offset;
                count = 0;
            }
            offset += kana.text.len();
            if kana.text != "ッ" {
                count += kana.mora_count();
            }
        } else {
            offset += text[offset..].chars().next().unwrap().len_utf8();
        }
    }
    if start < text.len() {
        parts.push(&text[start..]);
    }
    if parts.is_empty() {
        parts.push(text);
    }
    parts
}
