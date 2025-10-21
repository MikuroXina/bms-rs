use bms_rs::bms::prelude::*;

use std::num::NonZeroU64;
use std::path::Path;

// BMS source with various conflicts
const SOURCE_WITH_CONFLICTS: &str = r#"
    // BPM definition conflicts
    #BPM01 120
    #BPM01 140
    // Stop definition conflicts
    #STOP01 0.5
    #STOP01 1.0
    // Scroll definition conflicts
    #SCROLL01 1.0
    #SCROLL01 2.0
    // Speed definition conflicts
    #SPEED01 1.0
    #SPEED01 1.5
    // WAV definition conflicts
    #WAV01 old.wav
    #WAV01 new.wav
    // BMP definition conflicts
    #BMP01 old.bmp
    #BMP01 new.bmp
    // TEXT definition conflicts
    #TEXT01 old text
    #TEXT01 new text
    // Event conflicts
    #BPM 120
    #BPM01 120
    #BPM02 140
    #BPM03 160

    #00108:01
    #00208:02
    #00108:03
    // Same time as first
"#;

/// Test AlwaysUseOlder behavior with various conflict types
#[test]
fn test_always_use_older() {
    let LexOutput { tokens, .. } = TokenStream::parse_lex(SOURCE_WITH_CONFLICTS);

    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _, _>(
        &tokens,
        default_preset_with_prompter(&AlwaysUseOlder),
    )
    .unwrap();

    // Should have no warnings since AlwaysUseOlder handles conflicts silently
    assert_eq!(parse_warnings, vec![]);

    // Check that older values are used for all scope_defines conflicts
    assert_eq!(
        bms.scope_defines
            .bpm_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(120))
    );

    assert_eq!(
        bms.scope_defines
            .stop_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(0.5))
    );

    assert_eq!(
        bms.scope_defines
            .scroll_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(1.0))
    );

    assert_eq!(
        bms.scope_defines
            .speed_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(1.0))
    );

    // Check that older values are used for all other conflicts
    assert_eq!(
        bms.notes()
            .wav_files
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Path::new("old.wav").to_path_buf())
    );

    assert_eq!(
        bms.graphics
            .bmp_files
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Bmp {
            file: Path::new("old.bmp").to_path_buf(),
            transparent_color: Argb::default(),
        })
    );

    assert_eq!(
        bms.others.texts.get(&ObjId::try_from("01", false).unwrap()),
        Some(&"old text".to_string())
    );

    // Check that the older BPM change event is used (01, not 03)
    let bpm_changes: Vec<_> = bms.arrangers.bpm_changes.iter().collect();
    assert_eq!(bpm_changes.len(), 2); // Two different times
    assert_eq!(
        bpm_changes[0].0,
        &ObjTime::new(
            1,
            0,
            NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
        )
    );
    // The BPM change should be for the older event (01) - check the BPM value
    assert_eq!(bpm_changes[0].1.bpm, Decimal::from(120));
}

/// Test AlwaysUseNewer behavior with various conflict types
#[test]
fn test_always_use_newer() {
    let LexOutput { tokens, .. } = TokenStream::parse_lex(SOURCE_WITH_CONFLICTS);

    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _, _>(
        &tokens,
        default_preset_with_prompter(&AlwaysUseNewer),
    )
    .unwrap();

    // Should have no warnings since AlwaysUseNewer handles conflicts silently
    assert_eq!(parse_warnings, vec![]);

    // Check that newer values are used for all scope_defines conflicts
    assert_eq!(
        bms.scope_defines
            .bpm_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(120))
    );

    assert_eq!(
        bms.scope_defines
            .stop_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(1.0))
    );

    assert_eq!(
        bms.scope_defines
            .scroll_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(2.0))
    );

    assert_eq!(
        bms.scope_defines
            .speed_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(1.5))
    );

    // Check that newer values are used for all other conflicts
    assert_eq!(
        bms.notes()
            .wav_files
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Path::new("new.wav").to_path_buf())
    );

    assert_eq!(
        bms.graphics
            .bmp_files
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Bmp {
            file: Path::new("new.bmp").to_path_buf(),
            transparent_color: Argb::default(),
        })
    );

    assert_eq!(
        bms.others.texts.get(&ObjId::try_from("01", false).unwrap()),
        Some(&"new text".to_string())
    );

    // Check that the newer BPM change event is used (03, not 01)
    let bpm_changes: Vec<_> = bms.arrangers.bpm_changes.iter().collect();
    assert_eq!(bpm_changes.len(), 2); // Two different times
    assert_eq!(
        bpm_changes[0].0,
        &ObjTime::new(
            1,
            0,
            NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
        )
    );
    // The BPM change should be for the newer event (03)
    assert_eq!(bpm_changes[0].1.bpm, Decimal::from(160));
}

/// Test AlwaysWarnAndUseOlder behavior with various conflict types
#[test]
fn test_always_warn_and_use_older() {
    let LexOutput { tokens, .. } = TokenStream::parse_lex(SOURCE_WITH_CONFLICTS);

    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _, _>(
        &tokens,
        default_preset_with_prompter(&AlwaysWarnAndUseOlder),
    )
    .unwrap();

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
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(120))
    );

    assert_eq!(
        bms.scope_defines
            .stop_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(0.5))
    );

    assert_eq!(
        bms.scope_defines
            .scroll_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(1.0))
    );

    assert_eq!(
        bms.scope_defines
            .speed_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(1.0))
    );

    // Check that older values are used for all other conflicts
    assert_eq!(
        bms.notes()
            .wav_files
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Path::new("old.wav").to_path_buf())
    );

    assert_eq!(
        bms.graphics
            .bmp_files
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Bmp {
            file: Path::new("old.bmp").to_path_buf(),
            transparent_color: Argb::default(),
        })
    );

    assert_eq!(
        bms.others.texts.get(&ObjId::try_from("01", false).unwrap()),
        Some(&"old text".to_string())
    );

    // Check that the older BPM change event is used (01, not 03)
    let bpm_changes: Vec<_> = bms.arrangers.bpm_changes.iter().collect();
    assert_eq!(bpm_changes.len(), 2); // Two different times
    assert_eq!(
        bpm_changes[0].0,
        &ObjTime::new(
            1,
            0,
            NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
        )
    );
    // The BPM change should be for the older event (01)
    assert_eq!(bpm_changes[0].1.bpm, Decimal::from(120));
}

/// Test AlwaysWarnAndUseNewer behavior with various conflict types
#[test]
fn test_always_warn_and_use_newer() {
    let LexOutput { tokens, .. } = TokenStream::parse_lex(SOURCE_WITH_CONFLICTS);

    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _, _>(
        &tokens,
        default_preset_with_prompter(&AlwaysWarnAndUseNewer),
    )
    .unwrap();

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
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(120))
    );

    assert_eq!(
        bms.scope_defines
            .stop_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(1.0))
    );

    assert_eq!(
        bms.scope_defines
            .scroll_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(2.0))
    );

    assert_eq!(
        bms.scope_defines
            .speed_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(1.5))
    );

    // Check that newer values are used for all other conflicts
    assert_eq!(
        bms.notes()
            .wav_files
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Path::new("new.wav").to_path_buf())
    );

    assert_eq!(
        bms.graphics
            .bmp_files
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Bmp {
            file: Path::new("new.bmp").to_path_buf(),
            transparent_color: Argb::default(),
        })
    );

    assert_eq!(
        bms.others.texts.get(&ObjId::try_from("01", false).unwrap()),
        Some(&"new text".to_string())
    );

    // Check that the newer BPM change event is used (03, not 01)
    let bpm_changes: Vec<_> = bms.arrangers.bpm_changes.iter().collect();
    assert_eq!(bpm_changes.len(), 2); // Two different times
    assert_eq!(
        bpm_changes[0].0,
        &ObjTime::new(
            1,
            0,
            NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
        )
    );
    // The BPM change should be for the newer event (03)
    assert_eq!(bpm_changes[0].1.bpm, Decimal::from(160));
}
