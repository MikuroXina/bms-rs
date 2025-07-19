use bms_rs::lex::BmsLexOutput;
use bms_rs::parse::BmsParseOutput;
use bms_rs::{
    lex::{
        command::{Key, NoteKind},
        parse,
    },
    parse::{Bms, obj::Obj, prompt::AlwaysWarn, rng::RngMock},
    time::ObjTime,
};

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

    let BmsLexOutput { tokens, warnings } = parse(SRC);
    assert_eq!(warnings, vec![]);
    let rng = RngMock([1]);
    let BmsParseOutput { bms, warnings } = Bms::from_token_stream(&tokens, rng, AlwaysWarn);
    assert_eq!(warnings, vec![]);
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
    let BmsParseOutput { bms, warnings } = Bms::from_token_stream(&tokens, rng, AlwaysWarn);
    assert_eq!(warnings, vec![]);
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
    let BmsParseOutput { bms, warnings } = Bms::from_token_stream(&tokens, rng, AlwaysWarn);
    assert_eq!(warnings, vec![]);
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
