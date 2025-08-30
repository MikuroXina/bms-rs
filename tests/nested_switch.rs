use bms_rs::bms::prelude::*;
use num::BigUint;

#[test]
fn switch() {
    const SRC: &str = r"
        #00111:11000000

        #SWITCH 2

        #CASE 1

        #SKIP

        #CASE 2
            #00113:00003300
        #SKIP

        #ENDSW

        #00114:00000044
    ";
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(warnings, vec![]);
    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput::<BeatKey> {
        bms: _,
        parse_warnings,
        ..
    } = Bms::<BeatKey>::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
}

#[test]
fn nested_switch_simpler() {
    const SRC: &str = r"
        #SWITCH 2

        #CASE 1

            #SWITCH 2

            #CASE 1

            #SKIP

            #ENDSW

        #SKIP

        #CASE 2

        #SKIP

        #ENDSW
    ";
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(warnings, vec![]);
    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput::<BeatKey> {
        bms: _,
        parse_warnings,
        ..
    } = Bms::<BeatKey>::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
}

#[test]
fn nested_switch() {
    const SRC: &str = r"
        #00111:11000000

        #SWITCH 2

        #CASE 1
            #00112:00220000

            #SWITCH 2

            #CASE 1
                #00115:00550000
            #SKIP

            #CASE 2
                #00116:00006600
            #SKIP

            #ENDSW

        #SKIP

        #CASE 2
            #00113:00003300
        #SKIP

        #ENDSW

        #00114:00000044
    ";

    let id11 = "11".try_into().unwrap();
    let id22 = "22".try_into().unwrap();
    let id33 = "33".try_into().unwrap();
    let id44 = "44".try_into().unwrap();
    let id55 = "55".try_into().unwrap();
    let id66 = "66".try_into().unwrap();

    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(warnings, vec![]);
    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj::new_beat(ObjTime::new(1, 0, 4), PlayerSide::Player1, Key::Key1, id11),
            Obj::new_beat(ObjTime::new(1, 1, 4), PlayerSide::Player1, Key::Key2, id22),
            Obj::new_beat(ObjTime::new(1, 1, 4), PlayerSide::Player1, Key::Key5, id55),
            Obj::new_beat(ObjTime::new(1, 3, 4), PlayerSide::Player1, Key::Key4, id44),
        ]
    );

    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64), BigUint::from(2u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj::new_beat(ObjTime::new(1, 0, 4), PlayerSide::Player1, Key::Key1, id11),
            Obj::new_beat(ObjTime::new(1, 1, 4), PlayerSide::Player1, Key::Key2, id22),
            Obj::new_beat(
                ObjTime::new(1, 2, 4),
                PlayerSide::Player1,
                Key::Scratch,
                id66
            ),
            Obj::new_beat(ObjTime::new(1, 3, 4), PlayerSide::Player1, Key::Key4, id44),
        ]
    );

    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(2u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj::new_beat(ObjTime::new(1, 0, 4), PlayerSide::Player1, Key::Key1, id11),
            Obj::new_beat(ObjTime::new(1, 2, 4), PlayerSide::Player1, Key::Key3, id33),
            Obj::new_beat(ObjTime::new(1, 3, 4), PlayerSide::Player1, Key::Key4, id44),
        ]
    );
}

#[test]
fn nested_random_in_switch() {
    const SRC: &str = r"
        #00111:11000000

        #SWITCH 2

        #CASE 1
            #00112:00220000

            #RANDOM 2

            #IF 1
                #00115:00550000
            #ELSEIF 2
                #00116:00006600
            #ENDIF

            #ENDRANDOM

        #SKIP

        #CASE 2
            #00113:00003300
        #SKIP

        #ENDSW

        #00114:00000044
    ";

    let id11 = "11".try_into().unwrap();
    let id22 = "22".try_into().unwrap();
    let id33 = "33".try_into().unwrap();
    let id44 = "44".try_into().unwrap();
    let id55 = "55".try_into().unwrap();
    let id66 = "66".try_into().unwrap();

    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(warnings, vec![]);
    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj::new_beat(ObjTime::new(1, 0, 4), PlayerSide::Player1, Key::Key1, id11),
            Obj::new_beat(ObjTime::new(1, 1, 4), PlayerSide::Player1, Key::Key2, id22),
            Obj::new_beat(ObjTime::new(1, 1, 4), PlayerSide::Player1, Key::Key5, id55),
            Obj::new_beat(ObjTime::new(1, 3, 4), PlayerSide::Player1, Key::Key4, id44),
        ]
    );

    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64), BigUint::from(2u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj::new_beat(ObjTime::new(1, 0, 4), PlayerSide::Player1, Key::Key1, id11),
            Obj::new_beat(ObjTime::new(1, 1, 4), PlayerSide::Player1, Key::Key2, id22),
            Obj::new_beat(
                ObjTime::new(1, 2, 4),
                PlayerSide::Player1,
                Key::Scratch,
                id66
            ),
            Obj::new_beat(ObjTime::new(1, 3, 4), PlayerSide::Player1, Key::Key4, id44),
        ]
    );

    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(2u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj::new_beat(ObjTime::new(1, 0, 4), PlayerSide::Player1, Key::Key1, id11),
            Obj::new_beat(ObjTime::new(1, 2, 4), PlayerSide::Player1, Key::Key3, id33),
            Obj::new_beat(ObjTime::new(1, 3, 4), PlayerSide::Player1, Key::Key4, id44),
        ]
    );
}

#[test]
fn nested_switch_in_random() {
    const SRC: &str = r"
        #00111:11000000

        #RANDOM 2

        #IF 1
            #00112:00220000

            #SWITCH 2

            #CASE 1
                #00115:00550000
            #SKIP

            #CASE 2
                #00116:00006600
            #SKIP

            #ENDSW

        #ELSE
            #00113:00003300
        #ENDIF

        #ENDRANDOM

        #00114:00000044
    ";

    let id11 = "11".try_into().unwrap();
    let id22 = "22".try_into().unwrap();
    let id33 = "33".try_into().unwrap();
    let id44 = "44".try_into().unwrap();
    let id55 = "55".try_into().unwrap();
    let id66 = "66".try_into().unwrap();

    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(warnings, vec![]);
    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj::new_beat(ObjTime::new(1, 0, 4), PlayerSide::Player1, Key::Key1, id11),
            Obj::new_beat(ObjTime::new(1, 1, 4), PlayerSide::Player1, Key::Key2, id22),
            Obj::new_beat(ObjTime::new(1, 1, 4), PlayerSide::Player1, Key::Key5, id55),
            Obj::new_beat(ObjTime::new(1, 3, 4), PlayerSide::Player1, Key::Key4, id44),
        ]
    );

    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64), BigUint::from(2u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj::new_beat(ObjTime::new(1, 0, 4), PlayerSide::Player1, Key::Key1, id11),
            Obj::new_beat(ObjTime::new(1, 1, 4), PlayerSide::Player1, Key::Key2, id22),
            Obj::new_beat(
                ObjTime::new(1, 2, 4),
                PlayerSide::Player1,
                Key::Scratch,
                id66
            ),
            Obj::new_beat(ObjTime::new(1, 3, 4), PlayerSide::Player1, Key::Key4, id44),
        ]
    );

    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(2u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj::new_beat(ObjTime::new(1, 0, 4), PlayerSide::Player1, Key::Key1, id11),
            Obj::new_beat(ObjTime::new(1, 2, 4), PlayerSide::Player1, Key::Key3, id33),
            Obj::new_beat(ObjTime::new(1, 3, 4), PlayerSide::Player1, Key::Key4, id44),
        ]
    );
}

/// https://hitkey.bms.ms/cmds.htm#TEST-CASES
#[test]
fn test_switch_insane() {
    const SRC: &str = r"
    #SWITCH 5
        #DEF
            #00013:0055
            #SKIP
        #CASE 1
            #00013:0100000000000000
            #RANDOM 2
                #IF 1
                    #00014:04
                #ELSE
                    #00014:05
                #ENDIF
        #CASE 2
            #00013:0200000000000000
            #SKIP
        #CASE 3
            #00013:0300000000000000
            #SWITCH 2
                #CASE 1
                    #00016:1111
                    #SKIP
                #CASE 2
                    #00016:2222
                    #SKIP
            #ENDSW
            #SKIP
    #ENDSW
    ";

    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(lex_warnings, vec![]);

    // CASE 1, RANDOM 1
    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            // #CASE 1, #RANDOM 1, #IF 1
            Obj::new_beat(
                ObjTime::new(0, 0, 8),
                PlayerSide::Player1,
                Key::Key3,
                "01".try_into().unwrap()
            ),
            Obj::new_beat(
                ObjTime::new(0, 0, 1),
                PlayerSide::Player1,
                Key::Key4,
                "04".try_into().unwrap()
            ),
        ]
    );

    // CASE 1, RANDOM 2
    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64), BigUint::from(2u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            // #CASE 1, #RANDOM 2, #ELSE
            Obj::new_beat(
                ObjTime::new(0, 0, 8),
                PlayerSide::Player1,
                Key::Key3,
                "01".try_into().unwrap()
            ),
            Obj::new_beat(
                ObjTime::new(0, 0, 1),
                PlayerSide::Player1,
                Key::Key4,
                "05".try_into().unwrap()
            ),
        ]
    );

    // CASE 2
    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(2u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            // #CASE 2
            Obj::new_beat(
                ObjTime::new(0, 0, 8),
                PlayerSide::Player1,
                Key::Key3,
                "02".try_into().unwrap()
            ),
        ]
    );

    // CASE 3, SWITCH 1
    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(3u64), BigUint::from(1u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            // #CASE 3, #SWITCH 1
            Obj::new_beat(
                ObjTime::new(0, 0, 8),
                PlayerSide::Player1,
                Key::Key3,
                "03".try_into().unwrap()
            ),
            Obj::new_beat(
                ObjTime::new(0, 0, 2),
                PlayerSide::Player1,
                Key::Scratch,
                "11".try_into().unwrap()
            ),
            Obj::new_beat(
                ObjTime::new(0, 1, 2),
                PlayerSide::Player1,
                Key::Scratch,
                "11".try_into().unwrap()
            ),
        ]
    );

    // CASE 3, SWITCH 2
    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(3u64), BigUint::from(2u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            // #CASE 3, #SWITCH 2
            Obj::new_beat(
                ObjTime::new(0, 0, 8),
                PlayerSide::Player1,
                Key::Key3,
                "03".try_into().unwrap()
            ),
            Obj::new_beat(
                ObjTime::new(0, 0, 2),
                PlayerSide::Player1,
                Key::Scratch,
                "22".try_into().unwrap()
            ),
            Obj::new_beat(
                ObjTime::new(0, 1, 2),
                PlayerSide::Player1,
                Key::Scratch,
                "22".try_into().unwrap()
            ),
        ]
    );

    // CASE 4 (DEFAULT)
    let AstBuildOutput {
        root,
        ast_build_warnings,
    } = AstRoot::from_token_stream(&tokens);
    assert_eq!(ast_build_warnings, vec![]);
    let rng = RngMock([BigUint::from(4u64)]);
    let AstParseOutput { token_refs } = root.parse(rng);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![Obj::new_beat(
            ObjTime::new(0, 1, 2),
            PlayerSide::Player1,
            Key::Key3,
            "55".try_into().unwrap()
        ),]
    );
}
