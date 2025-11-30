use bms_rs::bms::prelude::*;
use num::BigUint;

#[test]
fn test_nested_random_structure() {
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

    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(RngMock([BigUint::from(1u64)])),
    );
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    assert_eq!(bms.randomized.len(), 1);
    let random_obj = &bms.randomized[0];

    // Check generating
    assert_eq!(
        random_obj.generating,
        Some(ControlFlowValue::GenMax(BigUint::from(2u64)))
    );

    // Check branches
    assert_eq!(random_obj.branches.len(), 2);

    // Branch 1
    let branch1 = &random_obj.branches[&BigUint::from(1u64)];
    assert_eq!(branch1.condition, BigUint::from(1u64));
    // Check content of branch 1 (should have note 112 and a nested random)
    assert_eq!(
        branch1.sub.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 1, 4).unwrap(),
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                .to_channel_id(),
            wav_id: ObjId::try_from("22", false).unwrap(),
        }]
    );
    assert_eq!(branch1.sub.randomized.len(), 1);

    // Nested Random
    let nested_random = &branch1.sub.randomized[0];
    assert_eq!(
        nested_random.generating,
        Some(ControlFlowValue::GenMax(BigUint::from(2u64)))
    );
    assert_eq!(nested_random.branches.len(), 2);

    // Nested Branch 1
    let nested_branch1 = &nested_random.branches[&BigUint::from(1u64)];
    assert_eq!(
        nested_branch1
            .sub
            .notes()
            .all_notes()
            .cloned()
            .collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 1, 4).unwrap(),
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(5))
                .to_channel_id(),
            wav_id: ObjId::try_from("55", false).unwrap(),
        }]
    );

    // Nested Branch 2
    let nested_branch2 = &nested_random.branches[&BigUint::from(2u64)];
    assert_eq!(
        nested_branch2
            .sub
            .notes()
            .all_notes()
            .cloned()
            .collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 2, 4).unwrap(),
            channel_id: NoteChannelId::try_from(['1', '6']).unwrap(),
            wav_id: ObjId::try_from("66", false).unwrap(),
        }]
    );

    // Branch 2 of outer random
    let branch2 = &random_obj.branches[&BigUint::from(2u64)];
    assert_eq!(
        branch2.sub.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 2, 4).unwrap(),
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3))
                .to_channel_id(),
            wav_id: ObjId::try_from("33", false).unwrap(),
        }]
    );
}

#[test]
fn test_nested_switch_structure() {
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

    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(RngMock([BigUint::from(1u64)])),
    );
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    assert_eq!(bms.randomized.len(), 1);
    let switch_obj = &bms.randomized[0];

    assert_eq!(
        switch_obj.generating,
        Some(ControlFlowValue::GenMax(BigUint::from(2u64)))
    );
    assert_eq!(switch_obj.branches.len(), 2);

    // Case 1
    let case1 = &switch_obj.branches[&BigUint::from(1u64)];
    assert_eq!(case1.sub.randomized.len(), 1);
    assert_eq!(
        case1.sub.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 1, 4).unwrap(),
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                .to_channel_id(),
            wav_id: ObjId::try_from("22", false).unwrap(),
        }]
    );

    // Nested Switch
    let nested_switch = &case1.sub.randomized[0];
    assert_eq!(
        nested_switch.generating,
        Some(ControlFlowValue::GenMax(BigUint::from(2u64)))
    );
    assert_eq!(nested_switch.branches.len(), 2);

    // Nested Case 1
    let nested_case1 = &nested_switch.branches[&BigUint::from(1u64)];
    assert_eq!(
        nested_case1
            .sub
            .notes()
            .all_notes()
            .cloned()
            .collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 1, 4).unwrap(),
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(5))
                .to_channel_id(),
            wav_id: ObjId::try_from("55", false).unwrap(),
        }]
    );

    // Nested Case 2
    let nested_case2 = &nested_switch.branches[&BigUint::from(2u64)];
    assert_eq!(
        nested_case2
            .sub
            .notes()
            .all_notes()
            .cloned()
            .collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 2, 4).unwrap(),
            channel_id: NoteChannelId::try_from(['1', '6']).unwrap(),
            wav_id: ObjId::try_from("66", false).unwrap(),
        }]
    );

    // Case 2
    let case2 = &switch_obj.branches[&BigUint::from(2u64)];
    assert_eq!(
        case2.sub.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 2, 4).unwrap(),
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3))
                .to_channel_id(),
            wav_id: ObjId::try_from("33", false).unwrap(),
        }]
    );
}

#[test]
fn test_export_as_random_tokens() {
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

    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(RngMock([BigUint::from(1u64)])),
    );
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    let rnd = &bms.randomized[0];
    let tokens = rnd.export_as_random::<KeyLayoutBeat>();
    let strings = tokens
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        strings,
        vec![
            "#RANDOM 2",
            "#IF 1",
            "#00112:00220000",
            "#ELSEIF 2",
            "#00113:0033",
            "#ENDIF",
            "#ENDRANDOM",
        ]
    );
}

#[test]
fn test_export_as_switch_tokens() {
    const SRC: &str = r"
        #00111:11000000

        #SWITCH 2

        #CASE 1
            #00112:00220000
        #SKIP

        #CASE 2
            #00113:00003300
        #SKIP

        #ENDSW

        #00114:00000044
    ";

    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(
        &tokens,
        default_config_with_rng(RngMock([BigUint::from(1u64)])),
    );
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    let sw = &bms.randomized[0];
    let tokens = sw.export_as_switch::<KeyLayoutBeat>();
    let strings = tokens
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();

    assert_eq!(
        strings,
        vec![
            "#SWITCH 2",
            "#CASE 1",
            "#00112:00220000",
            "#SKIP",
            "#CASE 2",
            "#00113:0033",
            "#SKIP",
            "#ENDSW",
        ]
    );
}
