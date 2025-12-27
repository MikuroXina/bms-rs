use bms_rs::bms::prelude::*;

#[test]
fn test_atbga_parsing() {
    let source = r#"
#TITLE Test BMS
#@BGA01 02 10 20 100 200 30 40
"#;
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();
    // Verify that #@BGA is parsed correctly
    let id_01 = ObjId::try_from("01", false).unwrap();
    let id_02 = ObjId::try_from("02", false).unwrap();
    let Some(atbga_def) = bms.bmp.atbga_defs.get(&id_01) else {
        panic!("Expected #@BGA{}/ definition to exist", id_01);
    };
    assert_eq!(atbga_def.source_bmp, id_02);
    assert_eq!(atbga_def.trim_top_left, PixelPoint::new(10, 20));
    assert_eq!(atbga_def.trim_size, PixelSize::new(100, 200));
    assert_eq!(atbga_def.draw_point, PixelPoint::new(30, 40));
}

#[test]
fn test_bga_parsing() {
    let source = r#"
#TITLE Test BMS
#BGA01 02 10 20 110 220 30 40
"#;
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    // Verify that #BGA is parsed correctly
    let id_01 = ObjId::try_from("01", false).unwrap();
    let id_02 = ObjId::try_from("02", false).unwrap();
    let Some(bga_def) = bms.bmp.bga_defs.get(&id_01) else {
        panic!("Expected #BGA{}/ definition to exist", id_01);
    };
    assert_eq!(bga_def.source_bmp, id_02);
    assert_eq!(bga_def.trim_top_left, PixelPoint::new(10, 20));
    assert_eq!(bga_def.trim_bottom_right, PixelPoint::new(110, 220));
    assert_eq!(bga_def.draw_point, PixelPoint::new(30, 40));
}

#[test]
fn test_exrank_parsing() {
    let source = r#"
#TITLE Test BMS
#EXRANK01 2
"#;
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    // Verify that #EXRANK is parsed correctly
    let id_01 = ObjId::try_from("01", false).unwrap();
    let Some(exrank_def) = bms.judge.exrank_defs.get(&id_01) else {
        panic!("Expected #EXRANK{}/ definition to exist", id_01);
    };
    assert_eq!(exrank_def.judge_level, JudgeLevel::Normal);
}

#[test]
fn test_exwav_parsing() {
    let source = r#"
#TITLE Test BMS
#EXWAV01 pvf 10000 0 48000 test.wav
"#;
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    // Verify that #EXWAV is parsed correctly
    let id_01 = ObjId::try_from("01", false).unwrap();
    let Some(exwav_def) = bms.wav.exwav_defs.get(&id_01) else {
        panic!("Expected #EXWAV{}/ definition to exist", id_01);
    };
    assert_eq!(*exwav_def.pan.as_ref(), 10000);
    assert_eq!(*exwav_def.volume.as_ref(), 0);
    assert_eq!(exwav_def.frequency.map(u64::from), Some(48000));
    assert_eq!(exwav_def.path.to_string_lossy(), "test.wav");
}

#[test]
fn test_changeoption_parsing() {
    let source = r#"
#TITLE Test BMS
#CHANGEOPTION01 test_option
"#;
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    // Verify that #CHANGEOPTION is parsed correctly
    let id_01 = ObjId::try_from("01", false).unwrap();
    let Some(option) = bms.option.change_options.get(&id_01) else {
        panic!("Expected #CHANGEOPTION{}/ definition to exist", id_01);
    };
    assert_eq!(option, "test_option");
}

#[test]
fn test_text_parsing() {
    let source = r#"
#TITLE Test BMS
#TEXT01 test_text
"#;
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    // Verify that #TEXT is parsed correctly
    let id_01 = ObjId::try_from("01", false).unwrap();
    let Some(text) = bms.text.texts.get(&id_01) else {
        panic!("Expected #TEXT{}/ definition to exist", id_01);
    };
    assert_eq!(text, "test_text");
}

#[test]
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
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    // Verify that extended fields in Notes are parsed correctly
    assert!(
        bms.judge
            .exrank_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.wav
            .exwav_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.wav
            .exwav_defs
            .contains_key(&ObjId::try_from("02", false).unwrap())
    );
    assert!(
        bms.option
            .change_options
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.text
            .texts
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
}

#[test]
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
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    // Verify that all new tokens are parsed correctly
    assert_eq!(bms.music_info.artist, Some("Test Artist".to_string()));
    assert_eq!(bms.metadata.email, Some("test@example.com".to_string()));
    assert_eq!(bms.metadata.url, Some("http://example.com".to_string()));
    assert_eq!(bms.music_info.maker, Some("Test Maker".to_string()));
    assert_eq!(
        bms.resources.midi_file,
        Some(std::path::PathBuf::from("test.mid"))
    );
    assert_eq!(
        bms.video.video_file,
        Some(std::path::PathBuf::from("test.mp4"))
    );
    assert_eq!(bms.bmp.poor_bga_mode, PoorMode::Overlay);
    assert!(bms.metadata.is_octave);
    assert_eq!(
        bms.metadata.wav_path_root,
        Some(std::path::PathBuf::from("wav/"))
    );

    // Verify new definition structures
    assert!(
        bms.bmp
            .atbga_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.bmp
            .bga_defs
            .contains_key(&ObjId::try_from("02", false).unwrap())
    );
    assert!(
        bms.judge
            .exrank_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.wav
            .exwav_defs
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.option
            .change_options
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
    assert!(
        bms.text
            .texts
            .contains_key(&ObjId::try_from("01", false).unwrap())
    );
}

#[test]
fn test_exwav_out_of_range_values() {
    {
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
        } = Bms::from_token_stream(&tokens, default_config().prompter(AlwaysUseNewer));
        let [warn]: &[_] = &parse_warnings[..] else {
            panic!("expected 1 warning, got: {0:?}", parse_warnings);
        };
        assert_eq!(
            warn.content(),
            &ParseWarning::SyntaxError(
                "expected pan value but out of range [-10000, 10000]".into()
            )
        );
    }

    {
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
        } = Bms::from_token_stream(&tokens, default_config().prompter(AlwaysUseNewer));
        let [warn]: &[_] = &parse_warnings[..] else {
            panic!("expected 1 warning, got: {0:?}", parse_warnings);
        };
        assert_eq!(
            warn.content(),
            &ParseWarning::SyntaxError("expected volume value but out of range [-10000, 0]".into())
        );
    }

    {
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
        } = Bms::from_token_stream(&tokens, default_config().prompter(AlwaysUseNewer));
        let [warn]: &[_] = &parse_warnings[..] else {
            panic!("expected 1 warning, got: {0:?}", parse_warnings);
        };
        assert_eq!(
            warn.content(),
            &ParseWarning::SyntaxError(
                "expected frequency value but out of range [100, 100000]".into()
            )
        );
    }
}
