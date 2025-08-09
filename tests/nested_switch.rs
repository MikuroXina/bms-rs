use bms_rs::bms::{parse_bms_with_tokens, prelude::*};
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
    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse_lex_tokens(SRC);
    assert_eq!(warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let BmsOutput {
        bms: _, warnings, ..
    } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );
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
    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse_lex_tokens(SRC);
    assert_eq!(warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let BmsOutput {
        bms: _, warnings, ..
    } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );
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

    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse_lex_tokens(SRC);
    assert_eq!(warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key2,
                obj: id22,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key5,
                obj: id55,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );

    let rng = RngMock([BigUint::from(1u64), BigUint::from(2u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key2,
                obj: id22,
            },
            Obj {
                offset: ObjTime::new(1, 2, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Scratch,
                obj: id66,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );

    let rng = RngMock([BigUint::from(2u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 2, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key3,
                obj: id33,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key4,
                obj: id44,
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

    let id11 = "11".try_into().unwrap();
    let id22 = "22".try_into().unwrap();
    let id33 = "33".try_into().unwrap();
    let id44 = "44".try_into().unwrap();
    let id55 = "55".try_into().unwrap();
    let id66 = "66".try_into().unwrap();

    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse_lex_tokens(SRC);
    assert_eq!(warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key2,
                obj: id22,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key5,
                obj: id55,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );

    let rng = RngMock([BigUint::from(1u64), BigUint::from(2u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key2,
                obj: id22,
            },
            Obj {
                offset: ObjTime::new(1, 2, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Scratch,
                obj: id66,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );

    let rng = RngMock([BigUint::from(2u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 2, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key3,
                obj: id33,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key4,
                obj: id44,
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

    let id11 = "11".try_into().unwrap();
    let id22 = "22".try_into().unwrap();
    let id33 = "33".try_into().unwrap();
    let id44 = "44".try_into().unwrap();
    let id55 = "55".try_into().unwrap();
    let id66 = "66".try_into().unwrap();

    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse_lex_tokens(SRC);
    assert_eq!(warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key2,
                obj: id22,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key5,
                obj: id55,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );

    let rng = RngMock([BigUint::from(1u64), BigUint::from(2u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key2,
                obj: id22,
            },
            Obj {
                offset: ObjTime::new(1, 2, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Scratch,
                obj: id66,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );

    let rng = RngMock([BigUint::from(2u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 2, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key3,
                obj: id33,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key4,
                obj: id44,
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

    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = bms_rs::lex::parse_lex_tokens(SRC);
    assert_eq!(warnings, vec![]);

    // CASE 1, RANDOM 1
    let rng = RngMock([BigUint::from(1u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            // #CASE 1, #RANDOM 1, #IF 1
            Obj {
                offset: ObjTime::new(0, 0, 8),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key3,
                obj: "01".try_into().unwrap(),
            },
            Obj {
                offset: ObjTime::new(0, 0, 1),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key4,
                obj: "04".try_into().unwrap(),
            },
        ]
    );

    // CASE 1, RANDOM 2
    let rng = RngMock([BigUint::from(1u64), BigUint::from(2u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            // #CASE 1, #RANDOM 2, #ELSE
            Obj {
                offset: ObjTime::new(0, 0, 8),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key3,
                obj: "01".try_into().unwrap(),
            },
            Obj {
                offset: ObjTime::new(0, 0, 1),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key4,
                obj: "05".try_into().unwrap(),
            },
        ]
    );

    // CASE 2
    let rng = RngMock([BigUint::from(2u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            // #CASE 2
            Obj {
                offset: ObjTime::new(0, 0, 8),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key3,
                obj: "02".try_into().unwrap(),
            },
        ]
    );

    // CASE 3, SWITCH 1
    let rng = RngMock([BigUint::from(3u64), BigUint::from(1u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            // #CASE 3, #SWITCH 1
            Obj {
                offset: ObjTime::new(0, 0, 8),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key3,
                obj: "03".try_into().unwrap(),
            },
            Obj {
                offset: ObjTime::new(0, 0, 2),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Scratch,
                obj: "11".try_into().unwrap(),
            },
            Obj {
                offset: ObjTime::new(0, 1, 2),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Scratch,
                obj: "11".try_into().unwrap(),
            },
        ]
    );

    // CASE 3, SWITCH 2
    let rng = RngMock([BigUint::from(3u64), BigUint::from(2u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            // #CASE 3, #SWITCH 2
            Obj {
                offset: ObjTime::new(0, 0, 8),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Key3,
                obj: "03".try_into().unwrap(),
            },
            Obj {
                offset: ObjTime::new(0, 0, 2),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Scratch,
                obj: "22".try_into().unwrap(),
            },
            Obj {
                offset: ObjTime::new(0, 1, 2),
                kind: NoteKind::Visible,
                side: PlayerSide::Player1,
                key: Key::Scratch,
                obj: "22".try_into().unwrap(),
            },
        ]
    );

    // CASE 4 (DEFAULT)
    let rng = RngMock([BigUint::from(4u64)]);
    let BmsOutput { bms, warnings, .. } = parse_bms_with_tokens(&tokens, rng);
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingWarning(_) | BmsWarning::PlayingError(_)
            ))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(
        bms.notes.into_all_notes(),
        vec![Obj {
            offset: ObjTime::new(0, 1, 2),
            kind: NoteKind::Visible,
            side: PlayerSide::Player1,
            key: Key::Key3,
            obj: "55".try_into().unwrap(),
        },]
    );
}
