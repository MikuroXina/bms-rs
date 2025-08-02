use bms_rs::bms::{
    Decimal,
    command::{Argb, Channel, ObjId, ObjTime, Track},
    lex::token::Token,
    parse::{
        BmsParseOutput, ParseWarning,
        model::{Bms, def::Bmp},
        prompt::{AlwaysUseNewer, AlwaysUseOlder, AlwaysWarn},
        random::rng::RngMock,
    },
};
use num::BigUint;
use std::borrow::Cow;
use std::path::Path;

/// Test that AlwaysUseOlder correctly uses the older value when there's a conflict
#[test]
fn test_always_use_older() {
    // Create tokens directly for testing
    let tokens = vec![
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(140)),
    ];

    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysUseOlder);

    // Should have no warnings since AlwaysUseOlder handles conflicts silently
    assert_eq!(parse_warnings, vec![]);

    // Check that the older BPM definition is used (120, not 140)
    assert_eq!(
        bms.scope_defines
            .bpm_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(120))
    );
}

/// Test that AlwaysUseNewer correctly uses the newer value when there's a conflict
#[test]
fn test_always_use_newer() {
    // Create tokens directly for testing
    let tokens = vec![
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(140)),
    ];

    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysUseNewer);

    // Should have no warnings since AlwaysUseNewer handles conflicts silently
    assert_eq!(parse_warnings, vec![]);

    // Check that the newer BPM definition is used (140, not 120)
    assert_eq!(
        bms.scope_defines
            .bpm_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(140))
    );
}

/// Test that AlwaysWarn correctly warns and uses the preferred value (older in this case)
#[test]
fn test_always_warn() {
    // Create tokens directly for testing
    let tokens = vec![
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(140)),
    ];

    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysWarn);

    // Should have warnings for each conflict
    assert_eq!(parse_warnings.len(), 1);
    assert!(
        parse_warnings
            .iter()
            .all(|w| matches!(w, ParseWarning::PromptHandlerWarning))
    );

    // Check that the older BPM definition is used (120, not 140) - AlwaysWarn uses older as preferred
    assert_eq!(
        bms.scope_defines
            .bpm_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(120))
    );
}

/// Test that AlwaysWarn works with different types of conflicts
#[test]
fn test_always_warn_various_conflicts() {
    // Create tokens directly for testing
    let tokens = vec![
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("old.wav")),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("new.wav")),
        Token::Bmp(Some(ObjId::try_from("01").unwrap()), Path::new("old.bmp")),
        Token::Bmp(Some(ObjId::try_from("01").unwrap()), Path::new("new.bmp")),
        Token::Text(ObjId::try_from("01").unwrap(), "old text"),
        Token::Text(ObjId::try_from("01").unwrap(), "new text"),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(140)),
    ];

    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysWarn);

    // Should have warnings for each conflict (4 conflicts)
    assert_eq!(parse_warnings.len(), 4);
    assert!(
        parse_warnings
            .iter()
            .all(|w| matches!(w, ParseWarning::PromptHandlerWarning))
    );

    // Check that older values are used for all conflicts
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

    assert_eq!(
        bms.scope_defines
            .bpm_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(120))
    );
}

/// Test scope_defines conflicts for various definition types
#[test]
fn test_scope_defines_conflicts() {
    // Create tokens with various scope_defines conflicts
    let tokens = vec![
        // BPM definitions
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(140)),
        // Stop definitions
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(0.5)),
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        // Scroll definitions
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(2.0)),
        // Speed definitions
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.5)),
    ];

    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysUseOlder);

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
}

/// Test scope_defines conflicts with AlwaysUseNewer
#[test]
fn test_scope_defines_conflicts_newer() {
    // Create tokens with various scope_defines conflicts
    let tokens = vec![
        // BPM definitions
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(140)),
        // Stop definitions
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(0.5)),
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        // Scroll definitions
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(2.0)),
        // Speed definitions
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.5)),
    ];

    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysUseNewer);

    // Should have no warnings since AlwaysUseNewer handles conflicts silently
    assert_eq!(parse_warnings, vec![]);

    // Check that newer values are used for all scope_defines conflicts
    assert_eq!(
        bms.scope_defines
            .bpm_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(140))
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
}

/// Test scope_defines conflicts with AlwaysWarn
#[test]
fn test_scope_defines_conflicts_warn() {
    // Create tokens with various scope_defines conflicts
    let tokens = vec![
        // BPM definitions
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(120)),
        Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(140)),
        // Stop definitions
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(0.5)),
        Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        // Scroll definitions
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(2.0)),
        // Speed definitions
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.0)),
        Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.5)),
    ];

    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysWarn);

    // Should have warnings for each conflict (4 conflicts)
    assert_eq!(parse_warnings.len(), 4);
    assert!(
        parse_warnings
            .iter()
            .all(|w| matches!(w, ParseWarning::PromptHandlerWarning))
    );

    // Check that older values are used for all scope_defines conflicts (AlwaysWarn uses older as preferred)
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
}

/// Test that AlwaysUseOlder works with event conflicts (BPM changes, etc.)
#[test]
fn test_always_use_older_events() {
    // Create tokens directly for testing
    let tokens = vec![
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
    ];

    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysUseOlder);

    // Should have no warnings
    assert_eq!(parse_warnings, vec![]);

    // Check that the older BPM change event is used (01, not 03)
    let bpm_changes: Vec<_> = bms.arrangers.bpm_changes.iter().collect();
    assert_eq!(bpm_changes.len(), 2); // Two different times
    assert_eq!(bpm_changes[0].0, &ObjTime::new(1, 0, 1));
    // The BPM change should be for the older event (01) - check the BPM value
    assert_eq!(bpm_changes[0].1.bpm, Decimal::from(120));
}

/// Test that AlwaysUseNewer works with event conflicts
#[test]
fn test_always_use_newer_events() {
    // Create tokens directly for testing
    let tokens = vec![
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
    ];

    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysUseNewer);

    // Should have no warnings
    assert_eq!(parse_warnings, vec![]);

    // Check that the newer BPM change event is used (03, not 01)
    let bpm_changes: Vec<_> = bms.arrangers.bpm_changes.iter().collect();
    assert_eq!(bpm_changes.len(), 2); // Two different times
    assert_eq!(bpm_changes[0].0, &ObjTime::new(1, 0, 1));
    // The BPM change should be for the newer event (03)
    assert_eq!(bpm_changes[0].1.bpm, Decimal::from(160));
}

/// Test that AlwaysWarn works with event conflicts and uses older as preferred
#[test]
fn test_always_warn_events() {
    // Create tokens directly for testing
    let tokens = vec![
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
    ];

    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysWarn);

    // Should have warnings for the conflict
    assert_eq!(parse_warnings.len(), 1);
    assert!(
        parse_warnings
            .iter()
            .all(|w| matches!(w, ParseWarning::PromptHandlerWarning))
    );

    // Check that the older BPM change event is used (01, not 03) - AlwaysWarn uses older as preferred
    let bpm_changes: Vec<_> = bms.arrangers.bpm_changes.iter().collect();
    assert_eq!(bpm_changes.len(), 2); // Two different times
    assert_eq!(bpm_changes[0].0, &ObjTime::new(1, 0, 1));
    // The BPM change should be for the older event (01)
    assert_eq!(bpm_changes[0].1.bpm, Decimal::from(120));
}
