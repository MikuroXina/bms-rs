use bms_rs::bms::{
    command::{
        channel::{Key, NoteKind, PlayerSide},
        time::ObjTime,
    },
    lex::{BmsLexOutput, parse_lex_tokens},
    parse::{
        BmsParseOutput,
        model::{Bms, obj::Obj},
        prompt::AlwaysWarn,
        random::rng::RngMock,
    },
};
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

    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse_lex_tokens(SRC);
    assert_eq!(warnings, vec![]);
    let rng = RngMock([BigUint::from(1u64)]);
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, rng, AlwaysWarn);
    assert_eq!(parse_warnings, vec![]);
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
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, rng, AlwaysWarn);
    assert_eq!(parse_warnings, vec![]);
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
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, rng, AlwaysWarn);
    assert_eq!(parse_warnings, vec![]);
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
