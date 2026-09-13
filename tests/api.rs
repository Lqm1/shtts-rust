use shtts::{Emotion, Voice, VoiceSettings, synthesize};

#[test]
fn emotion_keeps_speaker_speed_unless_overridden() {
    let shout = VoiceSettings::from(Voice::Male).with_emotion(Emotion::Shout);
    assert_eq!(shout.pitch, 5800);
    assert_eq!(shout.speed, 90);
    assert_eq!(shout.spectral, 90);
    let angry = VoiceSettings::from(Voice::Female).with_emotion(Emotion::Angry);
    assert_eq!(angry.speed, 80);
    assert_eq!(angry.pitch, 5450);
}

#[test]
fn output_state_is_independent_between_utterances_and_threads() {
    let settings = VoiceSettings::from(Voice::Sisters);
    let expected = synthesize("コンニチハ", &settings).unwrap();
    let other = VoiceSettings::from(Voice::Space);
    assert!(!synthesize("サクラ", &other).unwrap().is_empty());
    std::thread::scope(|scope| {
        let outputs: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| synthesize("コンニチハ", &settings).unwrap()))
            .collect();
        for output in outputs {
            assert_eq!(output.join().unwrap(), expected);
        }
    });
}

#[test]
fn invalid_properties_return_named_errors() {
    for (speed, spectral, name) in [
        (0, 100, "speed"),
        (100, 0, "spectral"),
        (40000, 100, "speed"),
    ] {
        let settings = VoiceSettings {
            speed,
            spectral,
            ..VoiceSettings::default()
        };
        assert_eq!(synthesize("ア", &settings).unwrap_err().name, name);
    }
}
