#[cfg(feature = "minor-command")]
use bms_rs::bms::prelude::*;

#[test]
#[cfg(feature = "minor-command")]
fn test_atbga_parsing() {
    let source = r#"
#TITLE Test BMS
#@BGA01 02 10 20 100 200 30 40
"#;
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    // Verify that #@BGA is parsed correctly
    assert!(
        bms.scope_defines
            .atbga_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    let atbga_def = &bms.scope_defines.atbga_defs[&ObjId::try_from("01", false).unwrap()];
    assert_eq!(atbga_def.source_bmp, ObjId::try_from("02", false).unwrap());
    assert_eq!(atbga_def.trim_top_left, PixelPoint::new(10, 20));
    assert_eq!(atbga_def.trim_size, PixelSize::new(100, 200));
    assert_eq!(atbga_def.draw_point, PixelPoint::new(30, 40));
}

#[test]
#[cfg(feature = "minor-command")]
fn test_bga_parsing() {
    let source = r#"
#TITLE Test BMS
#BGA01 02 10 20 110 220 30 40
"#;
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    // Verify that #BGA is parsed correctly
    assert!(
        bms.scope_defines
            .bga_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    let bga_def = &bms.scope_defines.bga_defs[&ObjId::try_from("01", false).unwrap()];
    assert_eq!(bga_def.source_bmp, ObjId::try_from("02", false).unwrap());
    assert_eq!(bga_def.trim_top_left, PixelPoint::new(10, 20));
    assert_eq!(bga_def.trim_bottom_right, PixelPoint::new(110, 220));
    assert_eq!(bga_def.draw_point, PixelPoint::new(30, 40));
}

#[test]
#[cfg(feature = "minor-command")]
fn test_exrank_parsing() {
    let source = r#"
#TITLE Test BMS
#EXRANK01 2
"#;
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    // Verify that #EXRANK is parsed correctly
    assert!(
        bms.scope_defines
            .exrank_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    let exrank_def = &bms.scope_defines.exrank_defs[&ObjId::try_from("01", false).unwrap()];
    assert_eq!(exrank_def.judge_level, JudgeLevel::Normal);
}

#[test]
#[cfg(feature = "minor-command")]
fn test_exwav_parsing() {
    let source = r#"
#TITLE Test BMS
#EXWAV01 pvf 10000 0 48000 test.wav
"#;
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    // Verify that #EXWAV is parsed correctly
    assert!(
        bms.scope_defines
            .exwav_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    let exwav_def = &bms.scope_defines.exwav_defs[&ObjId::try_from("01", false).unwrap()];
    assert_eq!(exwav_def.pan.value(), 10000);
    assert_eq!(exwav_def.volume.value(), 0);
    assert_eq!(exwav_def.frequency.map(|f| f.value()), Some(48000));
    assert_eq!(exwav_def.path.to_string_lossy(), "test.wav");
}

#[test]
#[cfg(feature = "minor-command")]
fn test_changeoption_parsing() {
    let source = r#"
#TITLE Test BMS
#CHANGEOPTION01 test_option
"#;
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    // Verify that #CHANGEOPTION is parsed correctly
    assert!(
        bms.others
            .change_options
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    let option = &bms.others.change_options[&ObjId::try_from("01", false).unwrap()];
    assert_eq!(option, "test_option");
}

#[test]
#[cfg(feature = "minor-command")]
fn test_text_parsing() {
    let source = r#"
#TITLE Test BMS
#TEXT01 test_text
"#;
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    // Verify that #TEXT is parsed correctly
    assert!(
        bms.others
            .texts
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    let text = &bms.others.texts[&ObjId::try_from("01", false).unwrap()];
    assert_eq!(text, "test_text");
}

#[test]
#[cfg(feature = "minor-command")]
fn test_notes_parse_extended_tokens() {
    let source = r#"
#TITLE Test BMS
#EXRANK01 2
#EXWAV01 pvf 10000 0 48000 test.wav
#EXWAV02 vpf 0 10000 48000 test2.wav
#CHANGEOPTION01 test_option
#TEXT01 test_text
"#;
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    // Verify that extended fields in Notes are parsed correctly
    assert!(
        bms.scope_defines
            .exrank_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.scope_defines
            .exwav_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.scope_defines
            .exwav_defs
            .contains_key(&ObjId::try_from("02", false).unwrap())
    );
    assert!(
        bms.others
            .change_options
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.others
            .texts
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
}

#[test]
#[cfg(feature = "minor-command")]
fn test_token_parsing_comprehensive() {
    let source = r#"
#TITLE Test BMS
#ARTIST Test Artist
#EMAIL test@example.com
#URL http://example.com
#MAKER Test Maker
#MIDIFILE test.mid
#VIDEOFILE test.mp4
#POORBGA 1
#OCT/FP
#PATH_WAV wav/
#@BGA01 02 10 20 100 200 30 40
#BGA02 03 15 25 150 250 35 45
#EXRANK01 2
#EXWAV01 pvf 10000 0 48000 test.wav
#CHANGEOPTION01 test_option
#TEXT01 test_text
"#;
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    // Verify that all new tokens are parsed correctly
    assert_eq!(bms.header.artist, Some("Test Artist".to_string()));
    assert_eq!(bms.header.email, Some("test@example.com".to_string()));
    assert_eq!(bms.header.url, Some("http://example.com".to_string()));
    assert_eq!(bms.header.maker, Some("Test Maker".to_string()));
    assert_eq!(
        bms.notes().midi_file,
        Some(std::path::PathBuf::from("test.mid"))
    );
    assert_eq!(
        bms.graphics.video_file,
        Some(std::path::PathBuf::from("test.mp4"))
    );
    assert_eq!(bms.graphics.poor_bga_mode, PoorMode::Overlay);
    assert!(bms.others.is_octave);
    assert_eq!(
        bms.notes().wav_path_root,
        Some(std::path::PathBuf::from("wav/"))
    );

    // Verify new definition structures
    assert!(
        bms.scope_defines
            .atbga_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.scope_defines
            .bga_defs
            .contains_key(&ObjId::try_from("02", false).unwrap())
    );
    assert!(
        bms.scope_defines
            .exrank_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.scope_defines
            .exwav_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.others
            .change_options
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.others
            .texts
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
}

#[test]
#[cfg(feature = "minor-command")]
fn test_exwav_out_of_range_values() {
    // Test pan value out of range
    let source = r#"
#TITLE Test BMS
#EXWAV01 p 10001 test.wav
"#;
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms: _,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysWarnAndUseOlder);
    let [warn] = &parse_warnings[..] else {
        panic!("expected 1 warning, got: {parse_warnings:?}");
    };
    assert_eq!(
        warn.content(),
        &ParseWarning::SyntaxError("expected pan value but out of range [-10000, 10000]".into())
    );

    // Test volume value out of range
    let source = r#"
#TITLE Test BMS
#EXWAV01 v 1 test.wav
"#;
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms: _,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysWarnAndUseOlder);
    let [warn] = &parse_warnings[..] else {
        panic!("expected 1 warning, got: {parse_warnings:?}");
    };
    assert_eq!(
        warn.content(),
        &ParseWarning::SyntaxError("expected volume value but out of range [-10000, 0]".into())
    );

    // Test frequency value out of range
    let source = r#"
#TITLE Test BMS
#EXWAV01 f 99 test.wav
"#;
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms: _,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysWarnAndUseOlder);
    let [warn] = &parse_warnings[..] else {
        panic!("expected 1 warning, got: {parse_warnings:?}");
    };
    assert_eq!(
        warn.content(),
        &ParseWarning::SyntaxError(
            "expected frequency value but out of range [100, 100000]".into()
        )
    );
}
