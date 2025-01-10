use bms_rs::{
    lex::{
        command::{Key, NoteKind},
        parse,
    },
    parse::{obj::Obj, rng::RngMock, Bms},
    time::ObjTime,
};

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

        #ENDSWITCH

        #00114:00000044
    ";
    let tokens = parse(SRC).expect("must be parsed");
    dbg!(tokens.iter());
    let rng = RngMock([1]);
    let _bms = Bms::from_token_stream(&tokens, rng).expect("must be parsed");
}

#[test]
fn nested_switch_simpler() {
    const SRC: &str = r"
        #SWITCH 2

        #CASE 1

            #SWITCH 2

            #CASE 1

            #SKIP

            #ENDSWITCH

        #SKIP

        #CASE 2

        #SKIP

        #ENDSWITCH
    ";
    let tokens = parse(SRC).expect("must be parsed");
    dbg!(tokens.iter());
    let rng = RngMock([1]);
    let _bms = Bms::from_token_stream(&tokens, rng).expect("must be parsed");
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

            #ENDSWITCH

        #SKIP

        #CASE 2
            #00113:00003300
        #SKIP

        #ENDSWITCH

        #00114:00000044
    ";

    let id11 = "11".try_into().unwrap();
    let id22 = "22".try_into().unwrap();
    let id33 = "33".try_into().unwrap();
    let id44 = "44".try_into().unwrap();
    let id55 = "55".try_into().unwrap();
    let id66 = "66".try_into().unwrap();

    let tokens = parse(SRC).expect("must be parsed");
    dbg!(tokens.iter());
    let rng = RngMock([1]);
    let bms = Bms::from_token_stream(&tokens, rng).expect("must be parsed");
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key2,
                obj: id22,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key5,
                obj: id55,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );

    let rng = RngMock([1, 2]);
    let bms = Bms::from_token_stream(&tokens, rng).expect("must be parsed");
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key2,
                obj: id22,
            },
            Obj {
                offset: ObjTime::new(1, 2, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Scratch,
                obj: id66,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );

    let rng = RngMock([2]);
    let bms = Bms::from_token_stream(&tokens, rng).expect("must be parsed");
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 2, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key3,
                obj: id33,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                is_player1: true,
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
            #SKIP

            #ELSEIF 2
                #00116:00006600
            #ENDIF

            #ENDRANDOM

        #SKIP

        #CASE 2
            #00113:00003300
        #SKIP

        #ENDSWITCH

        #00114:00000044
    ";

    let id11 = "11".try_into().unwrap();
    let id22 = "22".try_into().unwrap();
    let id33 = "33".try_into().unwrap();
    let id44 = "44".try_into().unwrap();
    let id55 = "55".try_into().unwrap();
    let id66 = "66".try_into().unwrap();

    let tokens = parse(SRC).expect("must be parsed");
    dbg!(tokens.iter());
    let rng = RngMock([1]);
    let bms = Bms::from_token_stream(&tokens, rng).expect("must be parsed");
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key2,
                obj: id22,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key5,
                obj: id55,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );

    let rng = RngMock([1, 2]);
    let bms = Bms::from_token_stream(&tokens, rng).expect("must be parsed");
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key2,
                obj: id22,
            },
            Obj {
                offset: ObjTime::new(1, 2, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Scratch,
                obj: id66,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );

    let rng = RngMock([2]);
    let bms = Bms::from_token_stream(&tokens, rng).expect("must be parsed");
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 2, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key3,
                obj: id33,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                is_player1: true,
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

            #ENDSWITCH

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

    let tokens = parse(SRC).expect("must be parsed");
    dbg!(tokens.iter());
    let rng = RngMock([1]);
    let bms = Bms::from_token_stream(&tokens, rng).expect("must be parsed");
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key2,
                obj: id22,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key5,
                obj: id55,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );

    let rng = RngMock([1, 2]);
    let bms = Bms::from_token_stream(&tokens, rng).expect("must be parsed");
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 1, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key2,
                obj: id22,
            },
            Obj {
                offset: ObjTime::new(1, 2, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Scratch,
                obj: id66,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );

    let rng = RngMock([2]);
    let bms = Bms::from_token_stream(&tokens, rng).expect("must be parsed");
    assert_eq!(
        bms.notes.into_all_notes(),
        vec![
            Obj {
                offset: ObjTime::new(1, 0, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key1,
                obj: id11,
            },
            Obj {
                offset: ObjTime::new(1, 2, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key3,
                obj: id33,
            },
            Obj {
                offset: ObjTime::new(1, 3, 4),
                kind: NoteKind::Visible,
                is_player1: true,
                key: Key::Key4,
                obj: id44,
            }
        ]
    );
}

/// https://hitkey.bms.ms/cmds.htm#TEST-CASES
///
/// TODO: This example cannot be resolved for now. It cannot be parsed by just scanning by line order.
#[test]
fn test_switch_unimpl() {
    const _SRC: &str = r"
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
}
