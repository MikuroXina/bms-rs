use bms_rs::bms::prelude::*;
use num::BigUint;
use pretty_assertions::assert_eq;

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

    let id11 = ObjId::try_from("11", false).unwrap();
    let id22 = ObjId::try_from("22", false).unwrap();
    let id33 = ObjId::try_from("33", false).unwrap();
    let id44 = ObjId::try_from("44", false).unwrap();
    let id55 = ObjId::try_from("55", false).unwrap();
    let id66 = ObjId::try_from("66", false).unwrap();

    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms,
        parse_warnings: warnings,
        control_flow_errors,
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _, _, _>(
        &tokens,
        default_config_with_rng(RngMock([BigUint::from(1u64)])),
    );
    assert_eq!(warnings, vec![]);
    assert_eq!(control_flow_errors, vec![]);
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

    let ParseOutput {
        bms,
        parse_warnings: warnings,
        control_flow_errors,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(RngMock([BigUint::from(1u64), BigUint::from(2u64)])),
    );
    assert_eq!(warnings, vec![]);
    assert_eq!(control_flow_errors, vec![]);
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

    let ParseOutput {
        bms,
        parse_warnings: warnings,
        control_flow_errors,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(RngMock([BigUint::from(2u64)])),
    );
    assert_eq!(warnings, vec![]);
    assert_eq!(control_flow_errors, vec![]);
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
