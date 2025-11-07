use bms_rs::bms::prelude::*;
use num::BigUint;
use pretty_assertions::assert_eq;
use std::num::NonZeroU64;

fn msg(track: u64, channel: Channel, message: &'static str) -> Token<'static> {
    Token::Message {
        track: Track(track),
        channel,
        message: message.into(),
    }
}

#[test]
fn into_tokens_basic_random() {
    // Outside of random
    let mut tokens: Vec<Token<'static>> = vec![msg(
        1,
        Channel::Note {
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
        },
        "11000000",
    )];

    // Build random block: max=2, cond 1 -> Key2:22, cond 2 -> Key3:33
    let random = Random::new(ControlFlowValue::GenMax(BigUint::from(2u64)))
        .if_block(If::new(
            BigUint::from(1u64),
            vec![TokenUnit::from_tokens(vec![msg(
                1,
                Channel::Note {
                    channel_id: KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        NoteKind::Visible,
                        Key::Key(2),
                    )
                    .to_channel_id(),
                },
                "00220000",
            )])],
        ))
        .if_block(If::new(
            BigUint::from(2u64),
            vec![TokenUnit::from_tokens(vec![msg(
                1,
                Channel::Note {
                    channel_id: KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        NoteKind::Visible,
                        Key::Key(3),
                    )
                    .to_channel_id(),
                },
                "00003300",
            )])],
        ));

    tokens.extend(random.into_tokens());

    // After random
    tokens.push(msg(
        1,
        Channel::Note {
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                .to_channel_id(),
        },
        "00000044",
    ));

    let id11 = ObjId::try_from("11", false).unwrap();
    let id22 = ObjId::try_from("22", false).unwrap();
    let id33 = ObjId::try_from("33", false).unwrap();
    let id44 = ObjId::try_from("44", false).unwrap();

    let tokens_wrapped: Vec<TokenWithRange<'static>> = tokens
        .into_iter()
        .map(|t| SourceRangeMixin::new(t, 0..0))
        .collect();

    // RNG = 1 -> branch cond 1
    let bms = Bms::from_token_stream(
        &tokens_wrapped,
        default_config_with_rng(RngMock([BigUint::from(1u64)])),
    )
    .unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(
                    1,
                    0,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1),)
                        .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    1,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2),)
                        .to_channel_id(),
                wav_id: id22,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    3,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4),)
                        .to_channel_id(),
                wav_id: id44,
            },
        ]
    );

    // RNG = 2 -> branch cond 2
    let tokens_wrapped2: Vec<TokenWithRange<'static>> = tokens_wrapped.to_vec();
    let bms = Bms::from_token_stream(
        &tokens_wrapped2,
        default_config_with_rng(RngMock([BigUint::from(2u64)])),
    )
    .unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(
                    1,
                    0,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1),)
                        .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    2,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3),)
                        .to_channel_id(),
                wav_id: id33,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    3,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4),)
                        .to_channel_id(),
                wav_id: id44,
            },
        ]
    );
}

#[test]
fn into_tokens_basic_setrandom() {
    // Outside of setrandom
    let mut tokens: Vec<Token<'static>> = vec![msg(
        1,
        Channel::Note {
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
        },
        "11000000",
    )];

    // Build setrandom block: value=2, branches same as above
    let setrandom = Random::new(ControlFlowValue::Set(BigUint::from(2u64)))
        .if_block(If::new(
            BigUint::from(1u64),
            vec![TokenUnit::from_tokens(vec![msg(
                1,
                Channel::Note {
                    channel_id: KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        NoteKind::Visible,
                        Key::Key(2),
                    )
                    .to_channel_id(),
                },
                "00220000",
            )])],
        ))
        .if_block(If::new(
            BigUint::from(2u64),
            vec![TokenUnit::from_tokens(vec![msg(
                1,
                Channel::Note {
                    channel_id: KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        NoteKind::Visible,
                        Key::Key(3),
                    )
                    .to_channel_id(),
                },
                "00003300",
            )])],
        ));

    tokens.extend(setrandom.into_tokens());
    tokens.push(msg(
        1,
        Channel::Note {
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                .to_channel_id(),
        },
        "00000044",
    ));

    let id11 = ObjId::try_from("11", false).unwrap();
    let id33 = ObjId::try_from("33", false).unwrap();
    let id44 = ObjId::try_from("44", false).unwrap();

    let tokens_wrapped: Vec<TokenWithRange<'static>> = tokens
        .into_iter()
        .map(|t| SourceRangeMixin::new(t, 0..0))
        .collect();

    // Because SETRANDOM=2, cond 2 branch is selected regardless of RNG
    let bms = Bms::from_token_stream(
        &tokens_wrapped,
        default_config_with_rng(RngMock([BigUint::from(1u64)])),
    )
    .unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(
                    1,
                    0,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1),)
                        .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    2,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3),)
                        .to_channel_id(),
                wav_id: id33,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    3,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4),)
                        .to_channel_id(),
                wav_id: id44,
            },
        ]
    );
}

#[test]
fn builder_and_mutation() {
    // Random with 6; entries: 4 -> Key5:55, 5 -> Scratch1:66, else -> Key2:22
    let mut random = Random::new(ControlFlowValue::GenMax(BigUint::from(6u64))).if_block(
        If::new(
            BigUint::from(4u64),
            vec![TokenUnit::from_tokens(vec![msg(
                1,
                Channel::Note {
                    channel_id: KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        NoteKind::Visible,
                        Key::Key(5),
                    )
                    .to_channel_id(),
                },
                "00005500",
            )])],
        )
        .or_else_if(
            BigUint::from(5u64),
            vec![TokenUnit::from_tokens(vec![msg(
                1,
                Channel::Note {
                    channel_id: KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        NoteKind::Visible,
                        Key::Scratch(1),
                    )
                    .to_channel_id(),
                },
                "00006600",
            )])],
        )
        .or_else(vec![TokenUnit::from_tokens(vec![msg(
            1,
            Channel::Note {
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                    .to_channel_id(),
            },
            "00220000",
        )])]),
    );

    // Mutate: change first cond 4 -> 3, change else-if tokens to Key3:33 (instead of scratch)
    {
        let first = random.at_mut(0).unwrap();
        let prev = first.set_condition(BigUint::from(3u64));
        assert_eq!(prev, BigUint::from(4u64));

        let _prev_units = first.set_units_at(
            1,
            vec![TokenUnit::from_tokens(vec![msg(
                1,
                Channel::Note {
                    channel_id: KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        NoteKind::Visible,
                        Key::Key(3),
                    )
                    .to_channel_id(),
                },
                "00003300",
            )])],
        );
    }

    // Compose full tokens (with a header before and after)
    let mut tokens: Vec<Token<'static>> = vec![msg(
        1,
        Channel::Note {
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
        },
        "11000000",
    )];
    tokens.extend(random.clone().into_tokens());
    tokens.push(msg(
        1,
        Channel::Note {
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                .to_channel_id(),
        },
        "00000044",
    ));

    let tokens_wrapped: Vec<TokenWithRange<'static>> = tokens
        .into_iter()
        .map(|t| SourceRangeMixin::new(t, 0..0))
        .collect();

    let id11 = ObjId::try_from("11", false).unwrap();
    let id22 = ObjId::try_from("22", false).unwrap();
    let id33 = ObjId::try_from("33", false).unwrap();
    let id44 = ObjId::try_from("44", false).unwrap();
    let id55 = ObjId::try_from("55", false).unwrap();

    // RNG = 3 -> first branch (Key5:55)
    let bms = Bms::from_token_stream(
        &tokens_wrapped,
        default_config_with_rng(RngMock([BigUint::from(3u64)])),
    )
    .unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(
                    1,
                    0,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1),)
                        .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    2,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(5),)
                        .to_channel_id(),
                wav_id: id55,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    3,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4),)
                        .to_channel_id(),
                wav_id: id44,
            },
        ]
    );

    // RNG = 5 -> else-if branch (Key3:33) after mutation
    let bms = Bms::from_token_stream(
        &tokens_wrapped,
        default_config_with_rng(RngMock([BigUint::from(5u64)])),
    )
    .unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(
                    1,
                    0,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1),)
                        .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    2,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3),)
                        .to_channel_id(),
                wav_id: id33,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    3,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4),)
                        .to_channel_id(),
                wav_id: id44,
            },
        ]
    );

    // RNG = 6 -> else branch (Key2:22)
    let bms = Bms::from_token_stream(
        &tokens_wrapped,
        default_config_with_rng(RngMock([BigUint::from(6u64)])),
    )
    .unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(
                    1,
                    0,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1),)
                        .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    1,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2),)
                        .to_channel_id(),
                wav_id: id22,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    3,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4),)
                        .to_channel_id(),
                wav_id: id44,
            },
        ]
    );
}

#[test]
fn into_tokens_basic_switch() {
    // Outside of switch
    let mut tokens: Vec<Token<'static>> = vec![msg(
        1,
        Channel::Note {
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
        },
        "11000000",
    )];

    // Build switch block: max=2, cond 1 -> Key2:22, cond 2 -> Key3:33
    let switch = Switch::new(ControlFlowValue::GenMax(BigUint::from(2u64)))
        .case_with_skip(
            BigUint::from(1u64),
            vec![TokenUnit::from_tokens(vec![msg(
                1,
                Channel::Note {
                    channel_id: KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        NoteKind::Visible,
                        Key::Key(2),
                    )
                    .to_channel_id(),
                },
                "00220000",
            )])],
        )
        .case_with_skip(
            BigUint::from(2u64),
            vec![TokenUnit::from_tokens(vec![msg(
                1,
                Channel::Note {
                    channel_id: KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        NoteKind::Visible,
                        Key::Key(3),
                    )
                    .to_channel_id(),
                },
                "00003300",
            )])],
        )
        .build();

    tokens.extend(switch.into_tokens());

    // After switch
    tokens.push(msg(
        1,
        Channel::Note {
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                .to_channel_id(),
        },
        "00000044",
    ));

    let id11 = ObjId::try_from("11", false).unwrap();
    let id22 = ObjId::try_from("22", false).unwrap();
    let id33 = ObjId::try_from("33", false).unwrap();
    let id44 = ObjId::try_from("44", false).unwrap();

    let tokens_wrapped: Vec<TokenWithRange<'static>> = tokens
        .into_iter()
        .map(|t| SourceRangeMixin::new(t, 0..0))
        .collect();

    // RNG = 1 -> branch cond 1
    let bms = Bms::from_token_stream(
        &tokens_wrapped,
        default_config_with_rng(RngMock([BigUint::from(1u64)])),
    )
    .unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(
                    1,
                    0,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1),)
                        .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    1,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2),)
                        .to_channel_id(),
                wav_id: id22,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    3,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4),)
                        .to_channel_id(),
                wav_id: id44,
            },
        ]
    );

    // RNG = 2 -> branch cond 2
    let tokens_wrapped2: Vec<TokenWithRange<'static>> = tokens_wrapped.to_vec();
    let bms = Bms::from_token_stream(
        &tokens_wrapped2,
        default_config_with_rng(RngMock([BigUint::from(2u64)])),
    )
    .unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(
                    1,
                    0,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1),)
                        .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    2,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3),)
                        .to_channel_id(),
                wav_id: id33,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    3,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4),)
                        .to_channel_id(),
                wav_id: id44,
            },
        ]
    );
}

#[test]
fn into_tokens_basic_setswitch() {
    // Outside of setswitch
    let mut tokens: Vec<Token<'static>> = vec![msg(
        1,
        Channel::Note {
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
        },
        "11000000",
    )];

    // Build setswitch block: value=2, same branches
    let setswitch = Switch::new(ControlFlowValue::Set(BigUint::from(2u64)))
        .case_with_skip(
            BigUint::from(1u64),
            vec![TokenUnit::from_tokens(vec![msg(
                1,
                Channel::Note {
                    channel_id: KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        NoteKind::Visible,
                        Key::Key(2),
                    )
                    .to_channel_id(),
                },
                "00220000",
            )])],
        )
        .case_with_skip(
            BigUint::from(2u64),
            vec![TokenUnit::from_tokens(vec![msg(
                1,
                Channel::Note {
                    channel_id: KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        NoteKind::Visible,
                        Key::Key(3),
                    )
                    .to_channel_id(),
                },
                "00003300",
            )])],
        )
        .build();

    tokens.extend(setswitch.into_tokens());
    tokens.push(msg(
        1,
        Channel::Note {
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                .to_channel_id(),
        },
        "00000044",
    ));

    let id11 = ObjId::try_from("11", false).unwrap();
    let id33 = ObjId::try_from("33", false).unwrap();
    let id44 = ObjId::try_from("44", false).unwrap();

    let tokens_wrapped: Vec<TokenWithRange<'static>> = tokens
        .into_iter()
        .map(|t| SourceRangeMixin::new(t, 0..0))
        .collect();

    // Because SETSWITCH=2, cond 2 branch is selected regardless of RNG
    let bms = Bms::from_token_stream(
        &tokens_wrapped,
        default_config_with_rng(RngMock([BigUint::from(1u64)])),
    )
    .unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(
                    1,
                    0,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1),)
                        .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    2,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3),)
                        .to_channel_id(),
                wav_id: id33,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    3,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4),)
                        .to_channel_id(),
                wav_id: id44,
            },
        ]
    );
}

#[test]
fn builder_basic_switch() {
    // Build with builder: GenMax=3, case 1 -> Key2:22, default -> Key3:33
    let switch = Switch::new(ControlFlowValue::GenMax(BigUint::from(3u64)))
        .case_with_skip(
            BigUint::from(1u64),
            vec![TokenUnit::from_tokens(vec![msg(
                1,
                Channel::Note {
                    channel_id: KeyLayoutBeat::new(
                        PlayerSide::Player1,
                        NoteKind::Visible,
                        Key::Key(2),
                    )
                    .to_channel_id(),
                },
                "00220000",
            )])],
        )
        .def(vec![TokenUnit::from_tokens(vec![msg(
            1,
            Channel::Note {
                channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3))
                    .to_channel_id(),
            },
            "00003300",
        )])])
        .build();

    let mut tokens: Vec<Token<'static>> = vec![msg(
        1,
        Channel::Note {
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1))
                .to_channel_id(),
        },
        "11000000",
    )];
    tokens.extend(switch.into_tokens());
    tokens.push(msg(
        1,
        Channel::Note {
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4))
                .to_channel_id(),
        },
        "00000044",
    ));

    let tokens_wrapped: Vec<TokenWithRange<'static>> = tokens
        .into_iter()
        .map(|t| SourceRangeMixin::new(t, 0..0))
        .collect();

    let id11 = ObjId::try_from("11", false).unwrap();
    let id22 = ObjId::try_from("22", false).unwrap();
    let id33 = ObjId::try_from("33", false).unwrap();
    let id44 = ObjId::try_from("44", false).unwrap();

    // RNG = 1 -> case branch
    let bms = Bms::from_token_stream(
        &tokens_wrapped,
        default_config_with_rng(RngMock([BigUint::from(1u64)])),
    )
    .unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(
                    1,
                    0,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1),)
                        .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    1,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2),)
                        .to_channel_id(),
                wav_id: id22,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    3,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4),)
                        .to_channel_id(),
                wav_id: id44,
            },
        ]
    );

    // RNG = 3 -> default branch
    let bms = Bms::from_token_stream(
        &tokens_wrapped,
        default_config_with_rng(RngMock([BigUint::from(3u64)])),
    )
    .unwrap();
    assert_eq!(
        bms.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![
            WavObj {
                offset: ObjTime::new(
                    1,
                    0,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(1),)
                        .to_channel_id(),
                wav_id: id11,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    2,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3),)
                        .to_channel_id(),
                wav_id: id33,
            },
            WavObj {
                offset: ObjTime::new(
                    1,
                    3,
                    NonZeroU64::new(4).expect("4 should be a valid NonZeroU64"),
                ),
                channel_id:
                    KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(4),)
                        .to_channel_id(),
                wav_id: id44,
            },
        ]
    );
}
