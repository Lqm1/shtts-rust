use shtts::{VoiceSettings, synthesize};
use std::{fs, path::Path};

#[test]
fn every_reference_pcm_matches_from_text() {
    compare("ref", include_str!("data/cases.tsv"), 1248);
}

#[test]
fn additional_inputs_match_python() {
    compare("python", include_str!("data/python/cases.tsv"), 130);
}

fn compare(folder: &str, cases: &str, expected_count: usize) {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(folder);
    let files = fs::read_dir(&directory)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|e| e == "pcm"))
        .count();
    let mut failures = Vec::new();
    let mut count = 0;
    for line in cases.lines() {
        let mut fields = line.split('\t');
        let name = fields.next().unwrap();
        let text = fields.next().unwrap();
        let mut integer = || fields.next().unwrap().parse::<i32>().unwrap();
        let settings = VoiceSettings {
            pitch: integer(),
            accent: integer(),
            phrase_accent: integer(),
            volume: integer(),
            speed: integer(),
            fluctuation_depth: integer(),
            fluctuation_delay: integer(),
            echo_delay: integer(),
            spectral: integer(),
            echo_depth: integer(),
            ring_rate: integer(),
        };
        let expected = fs::read(directory.join(format!("{name}.pcm"))).unwrap();
        assert_eq!(expected.len() % 2, 0, "truncated PCM: {name}");
        let expected: Vec<_> = expected
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let actual = synthesize(text, &settings).unwrap();
        let differences = actual.iter().zip(&expected).filter(|(a, b)| a != b).count()
            + actual.len().abs_diff(expected.len());
        if differences > 0 {
            failures.push(format!(
                "{name} {text}: {differences} differences; {} / {} samples",
                actual.len(),
                expected.len()
            ));
        }
        count += 1;
    }
    assert_eq!(count, expected_count);
    assert_eq!(files, count, "every PCM must have an input case");
    assert!(
        failures.is_empty(),
        "{} / {count} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
