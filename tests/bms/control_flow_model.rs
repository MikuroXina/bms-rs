use bms_rs::bms::prelude::*;

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
    } = Bms::from_token_stream(&tokens, default_config_with_rng(RngMock([1u64])));
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    assert_eq!(bms.randomized.len(), 1);
    let random_obj = bms
        .randomized
        .first()
        .expect("expected exactly 1 randomized block");

    // Check generating
    assert_eq!(random_obj.generating, Some(ControlFlowValue::GenMax(2u64)));

    // Check branches
    assert_eq!(random_obj.branches.len(), 2);

    // Branch 1
    let branch1 = random_obj
        .branches
        .get(&1u64)
        .expect("expected random branch 1");
    assert_eq!(branch1.condition, 1u64);
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
    let nested_random = branch1
        .sub
        .randomized
        .first()
        .expect("expected nested randomized block");
    assert_eq!(
        nested_random.generating,
        Some(ControlFlowValue::GenMax(2u64))
    );
    assert_eq!(nested_random.branches.len(), 2);

    // Nested Branch 1
    let nested_branch1 = nested_random
        .branches
        .get(&1u64)
        .expect("expected nested random branch 1");
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
    let nested_branch2 = nested_random
        .branches
        .get(&2u64)
        .expect("expected nested random branch 2");
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
    let branch2 = random_obj
        .branches
        .get(&2u64)
        .expect("expected random branch 2");
    assert_eq!(
        branch2.sub.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 2, 4).unwrap(),
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3))
                .to_channel_id(),
            wav_id: ObjId::try_from("33", false).unwrap(),
        }]
    );

    let rnd_strings_outer = random_obj
        .export_as_random::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        rnd_strings_outer,
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

    let sw_strings_outer = random_obj
        .export_as_switch::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        sw_strings_outer,
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

    let rnd_strings_nested = nested_random
        .export_as_random::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        rnd_strings_nested,
        vec![
            "#RANDOM 2",
            "#IF 1",
            "#00115:00550000",
            "#ELSEIF 2",
            "#00116:0066",
            "#ENDIF",
            "#ENDRANDOM",
        ]
    );

    let sw_strings_nested = nested_random
        .export_as_switch::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        sw_strings_nested,
        vec![
            "#SWITCH 2",
            "#CASE 1",
            "#00115:00550000",
            "#SKIP",
            "#CASE 2",
            "#00116:0066",
            "#SKIP",
            "#ENDSW",
        ]
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
    } = Bms::from_token_stream(&tokens, default_config_with_rng(RngMock([1u64])));
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    assert_eq!(bms.randomized.len(), 1);
    let switch_obj = bms
        .randomized
        .first()
        .expect("expected exactly 1 randomized block");

    assert_eq!(switch_obj.generating, Some(ControlFlowValue::GenMax(2u64)));
    assert_eq!(switch_obj.branches.len(), 2);

    // Case 1
    let case1 = switch_obj
        .branches
        .get(&1u64)
        .expect("expected switch case 1");
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
    let nested_switch = case1
        .sub
        .randomized
        .first()
        .expect("expected nested switch");
    assert_eq!(
        nested_switch.generating,
        Some(ControlFlowValue::GenMax(2u64))
    );
    assert_eq!(nested_switch.branches.len(), 2);

    // Nested Case 1
    let nested_case1 = nested_switch
        .branches
        .get(&1u64)
        .expect("expected nested case 1");
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
    let nested_case2 = nested_switch
        .branches
        .get(&2u64)
        .expect("expected nested case 2");
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
    let case2 = switch_obj
        .branches
        .get(&2u64)
        .expect("expected switch case 2");
    assert_eq!(
        case2.sub.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 2, 4).unwrap(),
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3))
                .to_channel_id(),
            wav_id: ObjId::try_from("33", false).unwrap(),
        }]
    );

    let sw_strings_outer = switch_obj
        .export_as_switch::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        sw_strings_outer,
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

    let rnd_strings_outer = switch_obj
        .export_as_random::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        rnd_strings_outer,
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

    let nested_switch_strings = nested_switch
        .export_as_switch::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        nested_switch_strings,
        vec![
            "#SWITCH 2",
            "#CASE 1",
            "#00115:00550000",
            "#SKIP",
            "#CASE 2",
            "#00116:0066",
            "#SKIP",
            "#ENDSW",
        ]
    );

    let nested_rnd_strings = nested_switch
        .export_as_random::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        nested_rnd_strings,
        vec![
            "#RANDOM 2",
            "#IF 1",
            "#00115:00550000",
            "#ELSEIF 2",
            "#00116:0066",
            "#ENDIF",
            "#ENDRANDOM",
        ]
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
    } = Bms::from_token_stream(&tokens, default_config_with_rng(RngMock([1u64])));
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    let rnd = bms
        .randomized
        .first()
        .expect("expected exactly 1 randomized block");
    let rnd_tokens = rnd.export_as_random::<KeyLayoutBeat>();
    let strings = rnd_tokens
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

    let sw_strings = rnd
        .export_as_switch::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        sw_strings,
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
    } = Bms::from_token_stream(&tokens, default_config_with_rng(RngMock([1u64])));
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    let sw = bms
        .randomized
        .first()
        .expect("expected exactly 1 randomized block");
    let sw_tokens = sw.export_as_switch::<KeyLayoutBeat>();
    let strings = sw_tokens
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

    let rnd_strings = sw
        .export_as_random::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        rnd_strings,
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
fn test_switch_fallthrough_one_skip() {
    const SRC: &str = r"
        #00111:11000000

        #SWITCH 2

        #CASE 1
            #00112:00220000

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
    } = Bms::from_token_stream(&tokens, default_config_with_rng(RngMock([1u64])));
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    assert_eq!(bms.randomized.len(), 1);
    let sw = bms
        .randomized
        .first()
        .expect("expected exactly 1 randomized block");
    assert_eq!(sw.branches.len(), 2);

    let case1 = sw.branches.get(&1u64).expect("expected switch case 1");
    assert_eq!(
        case1.sub.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 1, 4).unwrap(),
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                .to_channel_id(),
            wav_id: ObjId::try_from("22", false).unwrap(),
        }]
    );

    let case2 = sw.branches.get(&2u64).expect("expected switch case 2");
    assert_eq!(
        case2.sub.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 2, 4).unwrap(),
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3))
                .to_channel_id(),
            wav_id: ObjId::try_from("33", false).unwrap(),
        }]
    );

    let sw_tokens = sw.export_as_switch::<KeyLayoutBeat>();
    let strings = sw_tokens
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

    let rnd_strings = sw
        .export_as_random::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        rnd_strings,
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
fn test_switch_default_then_case_override() {
    const SRC: &str = r"
        #00111:11000000

        #SWITCH 2

        #DEF
            #00112:00220000
        #SKIP

        #CASE 2
            #00113:00003300

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
    } = Bms::from_token_stream(&tokens, default_config_with_rng(RngMock([2u64])));
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    assert_eq!(bms.randomized.len(), 1);
    let sw = bms
        .randomized
        .first()
        .expect("expected exactly 1 randomized block");
    assert_eq!(sw.branches.len(), 2);

    let case1 = sw.branches.get(&1u64).expect("expected switch case 1");
    assert_eq!(
        case1.sub.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 1, 4).unwrap(),
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(2))
                .to_channel_id(),
            wav_id: ObjId::try_from("22", false).unwrap(),
        }]
    );

    let case2 = sw.branches.get(&2u64).expect("expected switch case 2");
    assert_eq!(
        case2.sub.notes().all_notes().cloned().collect::<Vec<_>>(),
        vec![WavObj {
            offset: ObjTime::new(1, 2, 4).unwrap(),
            channel_id: KeyLayoutBeat::new(PlayerSide::Player1, NoteKind::Visible, Key::Key(3))
                .to_channel_id(),
            wav_id: ObjId::try_from("33", false).unwrap(),
        }]
    );

    let sw_tokens = sw.export_as_switch::<KeyLayoutBeat>();
    let strings = sw_tokens
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

    let rnd_strings = sw
        .export_as_random::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        rnd_strings,
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
fn test_export_both_and_compare() {
    const SRC: &str = r"
        #SWITCH 2

        #CASE 1
            #00112:00220000
        #SKIP

        #CASE 2
            #00113:00003300
        #SKIP

        #ENDSW

        #RANDOM 2

        #IF 1
            #00112:00220000

        #ELSEIF 2
            #00113:00003300

        #ENDIF

        #ENDRANDOM
    ";

    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config_with_rng(RngMock([1u64])));
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    assert!(bms.randomized.len() >= 2);
    let sw = bms
        .randomized
        .first()
        .expect("expected at least 2 randomized blocks");
    let rnd = bms
        .randomized
        .get(1)
        .expect("expected at least 2 randomized blocks");

    let sw_tokens = sw.export_as_switch::<KeyLayoutBeat>();
    let sw_strings = sw_tokens
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        sw_strings,
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

    let rnd_tokens = rnd.export_as_random::<KeyLayoutBeat>();
    let rnd_strings = rnd_tokens
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        rnd_strings,
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

    let sw_contents: Vec<_> = sw_strings
        .iter()
        .filter(|s| s.starts_with("#00"))
        .cloned()
        .collect();
    let rnd_contents: Vec<_> = rnd_strings
        .iter()
        .filter(|s| s.starts_with("#00"))
        .cloned()
        .collect();
    assert_eq!(sw_contents, rnd_contents);
}

#[test]
fn test_export_both_and_compare_different_contents() {
    const SRC: &str = r"
        #SWITCH 2

        #CASE 1
            #00112:00220000
        #SKIP

        #CASE 2
            #00113:00003300
        #SKIP

        #ENDSW

        #RANDOM 2

        #IF 1
            #00115:00550000

        #ELSEIF 2
            #00116:00006600

        #ENDIF

        #ENDRANDOM
    ";

    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(SRC);
    assert_eq!(lex_warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config_with_rng(RngMock([1u64])));
    assert_eq!(parse_warnings, vec![]);
    let bms = bms.unwrap();

    assert!(bms.randomized.len() >= 2);
    let sw = bms
        .randomized
        .first()
        .expect("expected at least 2 randomized blocks");
    let rnd = bms
        .randomized
        .get(1)
        .expect("expected at least 2 randomized blocks");

    let sw_strings = sw
        .export_as_switch::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        sw_strings,
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

    let rnd_strings = rnd
        .export_as_random::<KeyLayoutBeat>()
        .into_iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        rnd_strings,
        vec![
            "#RANDOM 2",
            "#IF 1",
            "#00115:00550000",
            "#ELSEIF 2",
            "#00116:0066",
            "#ENDIF",
            "#ENDRANDOM",
        ]
    );

    let sw_contents: Vec<_> = sw_strings
        .iter()
        .filter(|s| s.starts_with("#00"))
        .cloned()
        .collect();
    let rnd_contents: Vec<_> = rnd_strings
        .iter()
        .filter(|s| s.starts_with("#00"))
        .cloned()
        .collect();
    assert!(!sw_contents.is_empty());
    assert!(!rnd_contents.is_empty());
    assert_ne!(sw_contents, rnd_contents);
}
