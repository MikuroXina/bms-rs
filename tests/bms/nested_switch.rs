use bms_rs::bms::prelude::*;
use num::BigUint;
use pretty_assertions::assert_eq;

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
    let rng = RngMock([BigUint::from(1u64)]);
    let ParseOutput {
        bms: _,
        parse_warnings: warnings,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(rng).prompter(AlwaysUseNewer),
    );
    assert_eq!(warnings, vec![]);
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
    let rng = RngMock([BigUint::from(1u64)]);
    let ParseOutput {
        bms: _,
        parse_warnings: warnings,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(rng).prompter(AlwaysUseNewer),
    );
    assert_eq!(warnings, vec![]);
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

    let id11 = ObjId::try_from("11", false).unwrap();
    let id22 = ObjId::try_from("22", false).unwrap();
    let id33 = ObjId::try_from("33", false).unwrap();
    let id44 = ObjId::try_from("44", false).unwrap();
    let id55 = ObjId::try_from("55", false).unwrap();
    let id66 = ObjId::try_from("66", false).unwrap();

    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(rng).prompter(AlwaysUseNewer),
    );
    let bms = bms.unwrap();
    assert_eq!(warnings, vec![]);
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::start_of(1.into()),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(1, 1, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                    .to_channel_id(),
                wav_id: id22,
            },
            WavObj {
                offset: ObjTime::new(1, 1, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(5))
                    .to_channel_id(),
                wav_id: id55,
            },
            WavObj {
                offset: ObjTime::new(1, 3, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                    .to_channel_id(),
                wav_id: id44,
            }
        ]
    );

    let rng = RngMock([BigUint::from(1u64), BigUint::from(2u64)]);
    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(rng).prompter(AlwaysUseNewer),
    );
    let bms = bms.unwrap();
    assert_eq!(warnings, vec![]);
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::start_of(1.into()),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(1, 1, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                    .to_channel_id(),
                wav_id: id22,
            },
            WavObj {
                offset: ObjTime::new(1, 2, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(
                    PlayerSide::Player1,
                    NoteKind::Visible,
                    Key::Scratch(1)
                )
                .to_channel_id(),
                wav_id: id66,
            },
            WavObj {
                offset: ObjTime::new(1, 3, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                    .to_channel_id(),
                wav_id: id44,
            }
        ]
    );

    let rng = RngMock([BigUint::from(2u64)]);
    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(rng).prompter(AlwaysUseNewer),
    );
    let bms = bms.unwrap();
    assert_eq!(warnings, vec![]);
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::start_of(1.into()),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(1, 2, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3))
                    .to_channel_id(),
                wav_id: id33,
            },
            WavObj {
                offset: ObjTime::new(1, 3, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                    .to_channel_id(),
                wav_id: id44,
            }
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

    let id11 = ObjId::try_from("11", false).unwrap();
    let id22 = ObjId::try_from("22", false).unwrap();
    let id33 = ObjId::try_from("33", false).unwrap();
    let id44 = ObjId::try_from("44", false).unwrap();
    let id55 = ObjId::try_from("55", false).unwrap();
    let id66 = ObjId::try_from("66", false).unwrap();

    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let parse_output = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(rng).prompter(AlwaysUseNewer),
    );
    let bms = parse_output.bms.unwrap();
    assert_eq!(parse_output.parse_warnings, vec![]);
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::start_of(1.into()),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(1, 1, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                    .to_channel_id(),
                wav_id: id22,
            },
            WavObj {
                offset: ObjTime::new(1, 1, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(5))
                    .to_channel_id(),
                wav_id: id55,
            },
            WavObj {
                offset: ObjTime::new(1, 3, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                    .to_channel_id(),
                wav_id: id44,
            }
        ]
    );

    let rng = RngMock([BigUint::from(1u64), BigUint::from(2u64)]);
    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(rng).prompter(AlwaysUseNewer),
    );
    assert_eq!(warnings, vec![]);
    let bms = bms.unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::start_of(1.into()),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(1, 1, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                    .to_channel_id(),
                wav_id: id22,
            },
            WavObj {
                offset: ObjTime::new(1, 2, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(
                    PlayerSide::Player1,
                    NoteKind::Visible,
                    Key::Scratch(1)
                )
                .to_channel_id(),
                wav_id: id66,
            },
            WavObj {
                offset: ObjTime::new(1, 3, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                    .to_channel_id(),
                wav_id: id44,
            }
        ]
    );

    let rng = RngMock([BigUint::from(2u64)]);
    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(rng).prompter(AlwaysUseNewer),
    );
    assert_eq!(warnings, vec![]);
    let bms = bms.unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::start_of(1.into()),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(1, 2, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3))
                    .to_channel_id(),
                wav_id: id33,
            },
            WavObj {
                offset: ObjTime::new(1, 3, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                    .to_channel_id(),
                wav_id: id44,
            }
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

    let id22 = ObjId::try_from("22", false).unwrap();
    let id11 = ObjId::try_from("11", false).unwrap();
    let id33 = ObjId::try_from("33", false).unwrap();
    let id44 = ObjId::try_from("44", false).unwrap();
    let id55 = ObjId::try_from("55", false).unwrap();
    let id66 = ObjId::try_from("66", false).unwrap();

    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(rng).prompter(AlwaysUseNewer),
    );
    assert_eq!(warnings, vec![]);
    let bms = bms.unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::start_of(1.into()),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(1, 1, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                    .to_channel_id(),
                wav_id: id22,
            },
            WavObj {
                offset: ObjTime::new(1, 1, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(5))
                    .to_channel_id(),
                wav_id: id55,
            },
            WavObj {
                offset: ObjTime::new(1, 3, 4).expect("2 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                    .to_channel_id(),
                wav_id: id44,
            }
        ]
    );

    let rng = RngMock([BigUint::from(1u64), BigUint::from(2u64)]);
    let parse_output = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(rng).prompter(AlwaysUseNewer),
    );
    assert_eq!(parse_output.parse_warnings, vec![]);
    let bms = parse_output.bms.unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::start_of(1.into()),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(1, 1, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                    .to_channel_id(),
                wav_id: id22,
            },
            WavObj {
                offset: ObjTime::new(1, 2, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(
                    PlayerSide::Player1,
                    NoteKind::Visible,
                    Key::Scratch(1)
                )
                .to_channel_id(),
                wav_id: id66,
            },
            WavObj {
                offset: ObjTime::new(1, 3, 4).expect("2 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                    .to_channel_id(),
                wav_id: id44,
            }
        ]
    );

    let rng = RngMock([BigUint::from(2u64)]);
    let parse_output = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(rng).prompter(AlwaysUseNewer),
    );
    assert_eq!(parse_output.parse_warnings, vec![]);
    let bms = parse_output.bms.unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::start_of(1.into()),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                    .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(1, 2, 4).expect("4 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3))
                    .to_channel_id(),
                wav_id: id33,
            },
            WavObj {
                offset: ObjTime::new(1, 3, 4).expect("2 should be a valid denominator"),
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                    .to_channel_id(),
                wav_id: id44,
            }
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
    let expected = vec![WavObj {
        offset: ObjTime::new(0, 1, 2).expect("2 should be a valid denominator"),
        channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3))
            .to_channel_id(),
        wav_id: ObjId::try_from("55", false).unwrap(),
    }];

    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(lex_warnings, vec![]);

    for rng in [
        Box::new(RngMock([BigUint::from(1u64)])) as Box<dyn Rng>,
        Box::new(RngMock([BigUint::from(1u64), BigUint::from(2u64)])),
        Box::new(RngMock([BigUint::from(2u64)])),
        Box::new(RngMock([BigUint::from(3u64), BigUint::from(1u64)])),
        Box::new(RngMock([BigUint::from(3u64), BigUint::from(2u64)])),
        Box::new(RngMock([BigUint::from(4u64)])),
    ] {
        let ParseOutput {
            bms,
            parse_warnings: warnings,
        } = Bms::from_token_stream(
            &tokens,
            default_config_with_rng(rng).prompter(AlwaysUseNewer),
        );
        let bms = bms.unwrap();
        assert_eq!(warnings, vec![]);
        assert_eq!(
            bms.notes().all_notes().cloned().collect::<Vec<_>>(),
            expected
        );
    }
}
