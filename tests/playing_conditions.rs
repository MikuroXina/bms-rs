use bms_rs::bms::{parse_bms_with_tokens, prelude::*};
use num::BigUint;

#[test]
fn test_playing_conditions_empty_bms() {
    // Create an empty BMS content
    let source = "#PLAYER 1\n#GENRE Test\n#TITLE Test\n#ARTIST Test\n";
    let BmsLexOutput {
        tokens,
        lex_warnings,
    } = parse_lex_tokens(source);
    assert_eq!(lex_warnings, vec![]);

    let BmsOutput {
        bms: _, warnings, ..
    } = parse_bms_with_tokens(&tokens, RngMock([BigUint::from(1u64)]));
    assert_eq!(
        &warnings
            .iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingError(_) | BmsWarning::PlayingWarning(_)
            ))
            .map(ToOwned::to_owned)
            .collect::<Vec<BmsWarning>>(),
        &vec![]
    );

    // Should have warnings and errors for empty BMS
    assert!(warnings.contains(&BmsWarning::PlayingWarning(PlayingWarning::TotalUndefined)));
    assert!(warnings.contains(&BmsWarning::PlayingError(PlayingError::BpmUndefined)));
    assert!(warnings.contains(&BmsWarning::PlayingError(PlayingError::NoNotes)));
    // NoDisplayableNotes and NoPlayableNotes are not checked when there are no notes at all
}

#[test]
fn test_playing_conditions_with_bpm_and_notes() {
    // Create a BMS content with BPM and notes
    let source =
        "#PLAYER 1\n#GENRE Test\n#TITLE Test\n#ARTIST Test\n#BPM 120\n#TOTAL 100\n#00111:0101";
    let BmsLexOutput {
        tokens,
        lex_warnings,
    } = parse_lex_tokens(source);
    assert_eq!(lex_warnings, vec![]);

    let BmsOutput {
        bms: _, warnings, ..
    } = parse_bms_with_tokens(&tokens, RngMock([BigUint::from(1u64)]));
    assert_eq!(
        &warnings
            .iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<BmsWarning>>(),
        &vec![]
    );
}

#[test]
fn test_playing_conditions_with_bpm_change_only() {
    // Create a BMS content with only BPM change (no STARTBPM)
    let source = "#PLAYER 1\n#GENRE Test\n#TITLE Test\n#ARTIST Test\n#BPM08 120\n#00111:0101";
    let BmsLexOutput {
        tokens,
        lex_warnings,
    } = parse_lex_tokens(source);
    assert_eq!(lex_warnings, vec![]);

    assert!(
        !tokens
            .iter()
            .any(|t| matches!(&t.content, Token::Bpm(bpm) if bpm == &Decimal::from(120)))
    );
    let obj_id = ObjId::try_from("08").unwrap();
    assert!(tokens.iter().any(
        |t| matches!(&t.content, Token::BpmChange(id, bpm) if id == &obj_id && bpm == &Decimal::from(120))
    ));

    let BmsOutput { bms, warnings, .. } =
        parse_bms_with_tokens(&tokens, RngMock([BigUint::from(1u64)]));
    assert_eq!(
        &warnings
            .iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingError(_) | BmsWarning::PlayingWarning(_)
            ))
            .map(ToOwned::to_owned)
            .collect::<Vec<BmsWarning>>(),
        &vec![]
    );

    // Should have StartBpmUndefined warning but no BpmUndefined error
    assert_eq!(bms.arrangers.bpm, None);
    assert_eq!(bms.scope_defines.bpm_defs.len(), 1);
    assert_eq!(bms.arrangers.bpm_changes.len(), 0);
    assert!(warnings.contains(&BmsWarning::PlayingError(PlayingError::BpmUndefined)));
    assert!(!warnings.contains(&BmsWarning::PlayingWarning(
        PlayingWarning::StartBpmUndefined
    )));
}

#[test]
fn test_playing_conditions_invisible_notes_only() {
    // Create a BMS content with only invisible notes
    let source =
        "#PLAYER 1\n#GENRE Test\n#TITLE Test\n#ARTIST Test\n#BPM 120\n#TOTAL 100\n#00131:0101";
    let BmsLexOutput {
        tokens,
        lex_warnings,
    } = parse_lex_tokens(source);
    assert_eq!(lex_warnings, vec![]);

    let BmsOutput {
        bms: _, warnings, ..
    } = parse_bms_with_tokens(&tokens, RngMock([BigUint::from(1u64)]));
    assert_eq!(
        &warnings
            .iter()
            .filter(|w| !matches!(
                w,
                BmsWarning::PlayingError(_) | BmsWarning::PlayingWarning(_)
            ))
            .map(ToOwned::to_owned)
            .collect::<Vec<BmsWarning>>(),
        &vec![]
    );

    // Should have both NoDisplayableNotes and NoPlayableNotes warnings
    assert!(warnings.contains(&BmsWarning::PlayingWarning(
        PlayingWarning::NoDisplayableNotes
    )));
    assert!(warnings.contains(&BmsWarning::PlayingWarning(PlayingWarning::NoPlayableNotes)));
    assert!(
        !warnings
            .iter()
            .any(|w| matches!(w, BmsWarning::PlayingError(_)))
    );
}
