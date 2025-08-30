use bms_rs::bms::prelude::*;

#[test]
fn test_lnobj_parsing() {
    let source = r#"
#TITLE Test BMS
#00111:0102
#00211:0202
#LNOBJ 02
"#;

    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);

    let ParseOutput::<BeatKey> {
        bms,
        parse_warnings,
        ..
    } = Bms::<BeatKey>::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    // Check that there are exactly 2 long notes
    let total_long_notes: usize = bms.notes.objs.values()
        .flatten()
        .filter(|note| note.kind == NoteKind::Long)
        .count();
    assert_eq!(total_long_notes, 2, "Expected 2 long notes, found {}", total_long_notes);

    // Both long notes should have the same side, key, and obj_id
    let long_notes: Vec<_> = bms.notes.objs.values()
        .flatten()
        .filter(|note| note.kind == NoteKind::Long)
        .collect();

    assert_eq!(long_notes.len(), 2);

    // Verify both long notes have the same properties
    let note1 = long_notes[0];
    let note2 = long_notes[1];

    assert_eq!(note1.side, note2.side);
    assert_eq!(note1.key, note2.key);
    assert_eq!(note1.obj, note2.obj);

    // Verify times are different (one should be from track 1, one from track 2)
    assert_ne!(note1.offset, note2.offset);

    // One should be at track 1 (1, 2), one at track 2 (1, 2)
    // This is because #00111:0101 places notes at positions 1/2 in measure 1
    let track1_time = ObjTime::new(1, 1, 2);
    let track2_time = ObjTime::new(2, 1, 2);

    let times: Vec<ObjTime> = long_notes.iter().map(|note| note.offset).collect();
    println!("Expected track1_time: {:?}", track1_time);
    println!("Expected track2_time: {:?}", track2_time);
    println!("Actual times: {:?}", times);
    assert!(times.contains(&track1_time));
    assert!(times.contains(&track2_time));
}

#[test]
fn test_lnobj_no_matching_begin_note() {
    let source = r#"
#TITLE Test BMS
#LNOBJ 02
#00211:02
"#;

    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);

    let ParseOutput::<BeatKey> {
        bms: _,
        parse_warnings,
        ..
    } = Bms::<BeatKey>::from_token_stream(&tokens, AlwaysWarnAndUseOlder);

    // Should have a parse warning for missing begin note
    assert_eq!(parse_warnings.len(), 1);
    match &parse_warnings[0].content() {
        ParseWarning::SyntaxError(msg) => {
            assert!(msg.contains("No matching begin note found"));
        }
        other => panic!("Expected SyntaxError, got: {:?}", other),
    }
}

#[test]
fn test_lnobj_multiple_candidates() {
    let source = r#"
#TITLE Test BMS
#LNOBJ 02
#00111:0101
#00111:0303
#00211:0202
"#;

    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);

    let ParseOutput::<BeatKey> {
        bms,
        parse_warnings,
        ..
    } = Bms::<BeatKey>::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    // LNOBJ should pick the most recent (closest) note before the end note
    // In this case, it should pick the note from track 1 with id 03 (0303), not 01 (0101)
    let long_notes: Vec<_> = bms.notes.objs.values()
        .flatten()
        .filter(|note| note.kind == NoteKind::Long)
        .collect();

    assert_eq!(long_notes.len(), 2);

    // The begin note should be the one with id 03 (from #00111:0303)
    let begin_note = long_notes.iter()
        .find(|note| note.offset.track.0 == 1 && note.offset.numerator == 1)
        .expect("Should find begin note at track 1, position 1");

    assert_eq!(begin_note.obj, ObjId::try_from(['0', '3']).unwrap());
}

#[test]
fn test_lnobj_with_different_keys() {
    let source = r#"
#TITLE Test BMS
#LNOBJ 02
#00111:01
#00212:02
"#;

    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);

    let ParseOutput::<BeatKey> {
        bms: _,
        parse_warnings,
        ..
    } = Bms::<BeatKey>::from_token_stream(&tokens, AlwaysWarnAndUseOlder);

    // Should have a parse warning because keys don't match
    assert_eq!(parse_warnings.len(), 1);
    match &parse_warnings[0].content() {
        ParseWarning::SyntaxError(msg) => {
            assert!(msg.contains("No matching begin note found"));
        }
        other => panic!("Expected SyntaxError, got: {:?}", other),
    }
}

#[test]
fn test_lnobj_with_different_sides() {
    let source = r#"
#TITLE Test BMS
#00111:01
#00221:02
#LNOBJ 02
"#;

    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);

    let ParseOutput::<BeatKey> {
        bms: _,
        parse_warnings,
        ..
    } = Bms::<BeatKey>::from_token_stream(&tokens, AlwaysWarnAndUseOlder);

    // Should have a parse warning because sides don't match
    assert_eq!(parse_warnings.len(), 1);
    match &parse_warnings[0].content() {
        ParseWarning::SyntaxError(msg) => {
            assert!(msg.contains("No matching begin note found"));
        }
        other => panic!("Expected SyntaxError, got: {:?}", other),
    }
}
