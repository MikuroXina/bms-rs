use bms_rs::bms::prelude::*;

use std::borrow::Cow;
use std::path::Path;

/// Test AlwaysUseOlder behavior with various conflict types
#[test]
fn test_always_use_older() {
    // Create tokens with various conflicts
    let tokens: Vec<TokenWithPos> = vec![
        // BPM definition conflicts
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(140)),
        // Stop definition conflicts
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(0.5)),
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        // Scroll definition conflicts
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(2.0)),
        // Speed definition conflicts
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.5)),
        // WAV definition conflicts
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("old.wav")),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("new.wav")),
        // BMP definition conflicts
        Token::Bmp(Some(ObjId::try_from("01").unwrap()), Path::new("old.bmp")),
        Token::Bmp(Some(ObjId::try_from("01").unwrap()), Path::new("new.bmp")),
        // TEXT definition conflicts
        Token::Text(ObjId::try_from("01").unwrap(), "old text"),
        Token::Text(ObjId::try_from("01").unwrap(), "new text"),
        // Event conflicts
        Token::Bpm(Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("02").unwrap(), Decimal::from(140)),
        Token::BpmChange(ObjId::try_from("03").unwrap(), Decimal::from(160)),
        Token::Message {
            track: Track(1),
            channel: Channel::BpmChange,
            message: Cow::Borrowed("01"),
        },
        Token::Message {
            track: Track(2),
            channel: Channel::BpmChange,
            message: Cow::Borrowed("02"),
        },
        Token::Message {
            track: Track(1),
            channel: Channel::BpmChange,
            message: Cow::Borrowed("03"),
        }, // Same time as first
    ]
    .into_iter()
    .enumerate()
    .map(|(i, content)| content.into_wrapper_manual(i, i))
    .collect();

    let ParseOutput::<BeatKey> {
        bms,
        parse_warnings,
        ..
    } = Bms::<BeatKey>::from_token_stream(&tokens, AlwaysUseOlder);

    // Should have no warnings since AlwaysUseOlder handles conflicts silently
    assert_eq!(parse_warnings, vec![]);

    // Check that older values are used for all scope_defines conflicts
    assert_eq!(
        bms.scope_defines
            .bpm_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(120))
    );

    assert_eq!(
        bms.scope_defines
            .stop_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(0.5))
    );

    assert_eq!(
        bms.scope_defines
            .scroll_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(1.0))
    );

    assert_eq!(
        bms.scope_defines
            .speed_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(1.0))
    );

    // Check that older values are used for all other conflicts
    assert_eq!(
        bms.notes.wav_files.get(&ObjId::try_from("01").unwrap()),
        Some(&Path::new("old.wav").to_path_buf())
    );

    assert_eq!(
        bms.graphics.bmp_files.get(&ObjId::try_from("01").unwrap()),
        Some(&Bmp {
            file: Path::new("old.bmp").to_path_buf(),
            transparent_color: Argb::default(),
        })
    );

    assert_eq!(
        bms.others.texts.get(&ObjId::try_from("01").unwrap()),
        Some(&"old text".to_string())
    );

    // Check that the older BPM change event is used (01, not 03)
    let bpm_changes: Vec<_> = bms.arrangers.bpm_changes.iter().collect();
    assert_eq!(bpm_changes.len(), 2); // Two different times
    assert_eq!(bpm_changes[0].0, &ObjTime::new(1, 0, 1));
    // The BPM change should be for the older event (01) - check the BPM value
    assert_eq!(bpm_changes[0].1.bpm, Decimal::from(120));
}

/// Test AlwaysUseNewer behavior with various conflict types
#[test]
fn test_always_use_newer() {
    // Create tokens with various conflicts
    let tokens: Vec<TokenWithPos> = vec![
        // BPM definition conflicts
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(140)),
        // Stop definition conflicts
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(0.5)),
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        // Scroll definition conflicts
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(2.0)),
        // Speed definition conflicts
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.5)),
        // WAV definition conflicts
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("old.wav")),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("new.wav")),
        // BMP definition conflicts
        Token::Bmp(Some(ObjId::try_from("01").unwrap()), Path::new("old.bmp")),
        Token::Bmp(Some(ObjId::try_from("01").unwrap()), Path::new("new.bmp")),
        // TEXT definition conflicts
        Token::Text(ObjId::try_from("01").unwrap(), "old text"),
        Token::Text(ObjId::try_from("01").unwrap(), "new text"),
        // Event conflicts
        Token::Bpm(Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("02").unwrap(), Decimal::from(140)),
        Token::BpmChange(ObjId::try_from("03").unwrap(), Decimal::from(160)),
        Token::Message {
            track: Track(1),
            channel: Channel::BpmChange,
            message: Cow::Borrowed("01"),
        },
        Token::Message {
            track: Track(2),
            channel: Channel::BpmChange,
            message: Cow::Borrowed("02"),
        },
        Token::Message {
            track: Track(1),
            channel: Channel::BpmChange,
            message: Cow::Borrowed("03"),
        }, // Same time as first
    ]
    .into_iter()
    .enumerate()
    .map(|(i, content)| content.into_wrapper_manual(i, i))
    .collect();

    let ParseOutput::<BeatKey> {
        bms,
        parse_warnings,
        ..
    } = Bms::<BeatKey>::from_token_stream(&tokens, AlwaysUseNewer);

    // Should have no warnings since AlwaysUseNewer handles conflicts silently
    assert_eq!(parse_warnings, vec![]);

    // Check that newer values are used for all scope_defines conflicts
    assert_eq!(
        bms.scope_defines
            .bpm_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(120))
    );

    assert_eq!(
        bms.scope_defines
            .stop_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(1.0))
    );

    assert_eq!(
        bms.scope_defines
            .scroll_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(2.0))
    );

    assert_eq!(
        bms.scope_defines
            .speed_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(1.5))
    );

    // Check that newer values are used for all other conflicts
    assert_eq!(
        bms.notes.wav_files.get(&ObjId::try_from("01").unwrap()),
        Some(&Path::new("new.wav").to_path_buf())
    );

    assert_eq!(
        bms.graphics.bmp_files.get(&ObjId::try_from("01").unwrap()),
        Some(&Bmp {
            file: Path::new("new.bmp").to_path_buf(),
            transparent_color: Argb::default(),
        })
    );

    assert_eq!(
        bms.others.texts.get(&ObjId::try_from("01").unwrap()),
        Some(&"new text".to_string())
    );

    // Check that the newer BPM change event is used (03, not 01)
    let bpm_changes: Vec<_> = bms.arrangers.bpm_changes.iter().collect();
    assert_eq!(bpm_changes.len(), 2); // Two different times
    assert_eq!(bpm_changes[0].0, &ObjTime::new(1, 0, 1));
    // The BPM change should be for the newer event (03)
    assert_eq!(bpm_changes[0].1.bpm, Decimal::from(160));
}

/// Test AlwaysWarnAndUseOlder behavior with various conflict types
#[test]
fn test_always_warn_and_use_older() {
    // Create tokens with various conflicts
    let tokens: Vec<TokenWithPos> = vec![
        // BPM definition conflicts
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(140)),
        // Stop definition conflicts
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(0.5)),
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        // Scroll definition conflicts
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(2.0)),
        // Speed definition conflicts
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.5)),
        // WAV definition conflicts
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("old.wav")),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("new.wav")),
        // BMP definition conflicts
        Token::Bmp(Some(ObjId::try_from("01").unwrap()), Path::new("old.bmp")),
        Token::Bmp(Some(ObjId::try_from("01").unwrap()), Path::new("new.bmp")),
        // TEXT definition conflicts
        Token::Text(ObjId::try_from("01").unwrap(), "old text"),
        Token::Text(ObjId::try_from("01").unwrap(), "new text"),
        // Event conflicts
        Token::Bpm(Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("02").unwrap(), Decimal::from(140)),
        Token::BpmChange(ObjId::try_from("03").unwrap(), Decimal::from(160)),
        Token::Message {
            track: Track(1),
            channel: Channel::BpmChange,
            message: Cow::Borrowed("01"),
        },
        Token::Message {
            track: Track(2),
            channel: Channel::BpmChange,
            message: Cow::Borrowed("02"),
        },
        Token::Message {
            track: Track(1),
            channel: Channel::BpmChange,
            message: Cow::Borrowed("03"),
        }, // Same time as first
    ]
    .into_iter()
    .enumerate()
    .map(|(i, content)| content.into_wrapper_manual(i, i))
    .collect();

    let ParseOutput::<BeatKey> {
        bms,
        parse_warnings,
        ..
    } = Bms::<BeatKey>::from_token_stream(&tokens, AlwaysWarnAndUseOlder);

    // Should have warnings for each conflict (9 conflicts: 4 scope_defines + 3 others + 2 events)
    assert_eq!(parse_warnings.len(), 9);
    assert!(parse_warnings.iter().all(|w| matches!(
        w.content(),
        ParseWarning::DuplicatingChannelObj(_, _) | ParseWarning::DuplicatingDef(_)
    )));

    // Check that older values are used for all scope_defines conflicts
    assert_eq!(
        bms.scope_defines
            .bpm_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(120))
    );

    assert_eq!(
        bms.scope_defines
            .stop_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(0.5))
    );

    assert_eq!(
        bms.scope_defines
            .scroll_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(1.0))
    );

    assert_eq!(
        bms.scope_defines
            .speed_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(1.0))
    );

    // Check that older values are used for all other conflicts
    assert_eq!(
        bms.notes.wav_files.get(&ObjId::try_from("01").unwrap()),
        Some(&Path::new("old.wav").to_path_buf())
    );

    assert_eq!(
        bms.graphics.bmp_files.get(&ObjId::try_from("01").unwrap()),
        Some(&Bmp {
            file: Path::new("old.bmp").to_path_buf(),
            transparent_color: Argb::default(),
        })
    );

    assert_eq!(
        bms.others.texts.get(&ObjId::try_from("01").unwrap()),
        Some(&"old text".to_string())
    );

    // Check that the older BPM change event is used (01, not 03)
    let bpm_changes: Vec<_> = bms.arrangers.bpm_changes.iter().collect();
    assert_eq!(bpm_changes.len(), 2); // Two different times
    assert_eq!(bpm_changes[0].0, &ObjTime::new(1, 0, 1));
    // The BPM change should be for the older event (01)
    assert_eq!(bpm_changes[0].1.bpm, Decimal::from(120));
}

/// Test AlwaysWarnAndUseNewer behavior with various conflict types
#[test]
fn test_always_warn_and_use_newer() {
    // Create tokens with various conflicts
    let tokens: Vec<TokenWithPos> = vec![
        // BPM definition conflicts
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(140)),
        // Stop definition conflicts
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(0.5)),
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        // Scroll definition conflicts
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(2.0)),
        // Speed definition conflicts
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.5)),
        // WAV definition conflicts
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("old.wav")),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("new.wav")),
        // BMP definition conflicts
        Token::Bmp(Some(ObjId::try_from("01").unwrap()), Path::new("old.bmp")),
        Token::Bmp(Some(ObjId::try_from("01").unwrap()), Path::new("new.bmp")),
        // TEXT definition conflicts
        Token::Text(ObjId::try_from("01").unwrap(), "old text"),
        Token::Text(ObjId::try_from("01").unwrap(), "new text"),
        // Event conflicts
        Token::Bpm(Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("02").unwrap(), Decimal::from(140)),
        Token::BpmChange(ObjId::try_from("03").unwrap(), Decimal::from(160)),
        Token::Message {
            track: Track(1),
            channel: Channel::BpmChange,
            message: Cow::Borrowed("01"),
        },
        Token::Message {
            track: Track(2),
            channel: Channel::BpmChange,
            message: Cow::Borrowed("02"),
        },
        Token::Message {
            track: Track(1),
            channel: Channel::BpmChange,
            message: Cow::Borrowed("03"),
        }, // Same time as first
    ]
    .into_iter()
    .enumerate()
    .map(|(i, content)| content.into_wrapper_manual(i, i))
    .collect();

    let ParseOutput::<BeatKey> {
        bms,
        parse_warnings,
        ..
    } = Bms::<BeatKey>::from_token_stream(&tokens, AlwaysWarnAndUseNewer);

    // Should have no warnings since AlwaysWarnAndUseNewer handles conflicts silently
    assert!(
        parse_warnings
            .iter()
            .any(|w| matches!(w.content(), ParseWarning::DuplicatingDef(_)))
    );

    // Check that newer values are used for all scope_defines conflicts
    assert_eq!(
        bms.scope_defines
            .bpm_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(120))
    );

    assert_eq!(
        bms.scope_defines
            .stop_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(1.0))
    );

    assert_eq!(
        bms.scope_defines
            .scroll_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(2.0))
    );

    assert_eq!(
        bms.scope_defines
            .speed_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(1.5))
    );

    // Check that newer values are used for all other conflicts
    assert_eq!(
        bms.notes.wav_files.get(&ObjId::try_from("01").unwrap()),
        Some(&Path::new("new.wav").to_path_buf())
    );

    assert_eq!(
        bms.graphics.bmp_files.get(&ObjId::try_from("01").unwrap()),
        Some(&Bmp {
            file: Path::new("new.bmp").to_path_buf(),
            transparent_color: Argb::default(),
        })
    );

    assert_eq!(
        bms.others.texts.get(&ObjId::try_from("01").unwrap()),
        Some(&"new text".to_string())
    );

    // Check that the newer BPM change event is used (03, not 01)
    let bpm_changes: Vec<_> = bms.arrangers.bpm_changes.iter().collect();
    assert_eq!(bpm_changes.len(), 2); // Two different times
    assert_eq!(bpm_changes[0].0, &ObjTime::new(1, 0, 1));
    // The BPM change should be for the newer event (03)
    assert_eq!(bpm_changes[0].1.bpm, Decimal::from(160));
}
