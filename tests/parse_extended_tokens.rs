use bms_rs::{
    lex::parse,
    parse::{Bms, prompt::AlwaysHalt, rng::RngMock},
};

#[test]
fn test_atbga_parsing() {
    let source = r#"
#TITLE Test BMS
#@BGA01 02 10 20 100 200 30 40
"#;
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysHalt).expect("must be parsed");

    // Verify that #@BGA is parsed correctly
    assert!(
        bms.header
            .atbga_defs
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    let atbga_def =
        &bms.header.atbga_defs[&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap()];
    assert_eq!(
        atbga_def.source_bmp,
        bms_rs::lex::command::ObjId::from_chars(['0', '2']).unwrap()
    );
    assert_eq!(atbga_def.trim_top_left, (10, 20));
    assert_eq!(atbga_def.trim_size, (100, 200));
    assert_eq!(atbga_def.draw_point, (30, 40));
}

#[test]
fn test_bga_parsing() {
    let source = r#"
#TITLE Test BMS
#BGA01 02 10 20 110 220 30 40
"#;
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysHalt).expect("must be parsed");

    // Verify that #BGA is parsed correctly
    assert!(
        bms.header
            .bga_defs
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    let bga_def =
        &bms.header.bga_defs[&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap()];
    assert_eq!(
        bga_def.source_bmp,
        bms_rs::lex::command::ObjId::from_chars(['0', '2']).unwrap()
    );
    assert_eq!(bga_def.trim_top_left, (10, 20));
    assert_eq!(bga_def.trim_bottom_right, (110, 220));
    assert_eq!(bga_def.draw_point, (30, 40));
}

#[test]
fn test_exrank_parsing() {
    let source = r#"
#TITLE Test BMS
#EXRANK01 2
"#;
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysHalt).expect("must be parsed");

    // Verify that #EXRANK is parsed correctly
    assert!(
        bms.header
            .exrank_defs
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    let exrank_def =
        &bms.header.exrank_defs[&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap()];
    assert_eq!(
        exrank_def.judge_level,
        bms_rs::lex::command::JudgeLevel::Normal
    );
}

#[test]
fn test_exwav_parsing() {
    let source = r#"
#TITLE Test BMS
#EXWAV01 param1 param2 param3 test.wav
"#;
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysHalt).expect("must be parsed");

    // Verify that #EXWAV is parsed correctly
    assert!(
        bms.header
            .exwav_defs
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    let exwav_def =
        &bms.header.exwav_defs[&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap()];
    assert_eq!(exwav_def.params, ["param1", "param2", "param3", "test.wav"]);
    assert_eq!(exwav_def.path.to_string_lossy(), "test.wav");
}

#[test]
fn test_changeoption_parsing() {
    let source = r#"
#TITLE Test BMS
#CHANGEOPTION01 test_option
"#;
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysHalt).expect("must be parsed");

    // Verify that #CHANGEOPTION is parsed correctly
    assert!(
        bms.header
            .change_options
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    let option =
        &bms.header.change_options[&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap()];
    assert_eq!(option, "test_option");
}

#[test]
fn test_text_parsing() {
    let source = r#"
#TITLE Test BMS
#TEXT01 test_text
"#;
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysHalt).expect("must be parsed");

    // Verify that #TEXT is parsed correctly
    assert!(
        bms.header
            .texts
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    let text = &bms.header.texts[&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap()];
    assert_eq!(text, "test_text");
}

#[test]
fn test_notes_parse_extended_tokens() {
    let source = r#"
#TITLE Test BMS
#EXRANK01 2
#EXWAV01 param1 param2 param3 test.wav
#CHANGEOPTION01 test_option
#TEXT01 test_text
"#;
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysHalt).expect("must be parsed");

    // Verify that extended fields in Notes are parsed correctly
    assert!(
        bms.notes
            .exrank_defs
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    assert!(
        bms.notes
            .exwav_defs
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    assert!(
        bms.notes
            .change_options
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    assert!(
        bms.notes
            .texts
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
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
#EXWAV01 param1 param2 param3 test.wav
#CHANGEOPTION01 test_option
#TEXT01 test_text
"#;
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysHalt).expect("must be parsed");

    // Verify that all new tokens are parsed correctly
    assert_eq!(bms.header.artist, Some("Test Artist".to_string()));
    assert_eq!(bms.header.email, Some("test@example.com".to_string()));
    assert_eq!(bms.header.url, Some("http://example.com".to_string()));
    assert_eq!(bms.header.maker, Some("Test Maker".to_string()));
    assert_eq!(
        bms.header.midi_file,
        Some(std::path::PathBuf::from("test.mid"))
    );
    assert_eq!(
        bms.header.video_file,
        Some(std::path::PathBuf::from("test.mp4"))
    );
    assert_eq!(
        bms.header.poor_bga_mode,
        bms_rs::lex::command::PoorMode::Overlay
    );
    assert!(bms.header.is_octave);
    assert_eq!(
        bms.header.wav_path_root,
        Some(std::path::PathBuf::from("wav/"))
    );

    // Verify new definition structures
    assert!(
        bms.header
            .atbga_defs
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    assert!(
        bms.header
            .bga_defs
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '2']).unwrap())
    );
    assert!(
        bms.header
            .exrank_defs
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    assert!(
        bms.header
            .exwav_defs
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    assert!(
        bms.header
            .change_options
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
    assert!(
        bms.header
            .texts
            .contains_key(&bms_rs::lex::command::ObjId::from_chars(['0', '1']).unwrap())
    );
}
