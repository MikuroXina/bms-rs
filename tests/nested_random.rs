use bms_rs::bms::prelude::*;
use num::BigUint;

#[test]
fn nested_random() {
    const SRC: &str = r"
        #00111:11000000

        #RANDOM 2

        #IF 1
            #00112:00220000

            #RANDOM 2

            #IF 1
                #00115:00550000
            #ENDIF

            #IF 2
                #00116:00006600
            #ENDIF

            #ENDRANDOM

        #ENDIF

        #IF 2
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
        lex_warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(lex_warnings, vec![]);

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
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(1, 0, 4),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(1, 1, 4),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                    .to_channel_id(),
                wav_id: id22,
            },
            WavObj {
                offset: ObjTime::new(1, 1, 4),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(5))
                    .to_channel_id(),
                wav_id: id55,
            },
            WavObj {
                offset: ObjTime::new(1, 3, 4),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                    .to_channel_id(),
                wav_id: id44,
            }
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
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(1, 0, 4),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(1, 1, 4),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                    .to_channel_id(),
                wav_id: id22,
            },
            WavObj {
                offset: ObjTime::new(1, 2, 4),
                channel_id: KeyLayoutBeat::new(
                    PlayerSide::Player1,
                    NoteKind::Visible,
                    Key::Scratch(1)
                )
                .to_channel_id(),
                wav_id: id66,
            },
            WavObj {
                offset: ObjTime::new(1, 3, 4),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                    .to_channel_id(),
                wav_id: id44,
            }
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
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(token_refs, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(1, 0, 4),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(1, 2, 4),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3))
                    .to_channel_id(),
                wav_id: id33,
            },
            WavObj {
                offset: ObjTime::new(1, 3, 4),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                    .to_channel_id(),
                wav_id: id44,
            }
        ]
    );
}
