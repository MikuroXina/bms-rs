use bms_rs::bms::{
    Decimal,
    command::ObjId,
    lex::{BmsLexOutput, parse_lex_tokens, token::Token},
    parse::{
        BmsParseOutput,
        check_playing::{PlayingError, PlayingWarning},
        model::Bms,
        prompt::AlwaysWarnAndUseOlder,
        random::rng::RngMock,
    },
};
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

    let rng = RngMock([BigUint::from(1u64)]);
    let BmsParseOutput {
        bms: _,
        parse_warnings,
        playing_warnings,
        playing_errors,
    } = Bms::from_token_stream(&tokens, rng, AlwaysWarnAndUseOlder);

    assert_eq!(parse_warnings, vec![]);

    // Should have warnings and errors for empty BMS
    assert!(playing_warnings.contains(&PlayingWarning::TotalUndefined));
    assert!(playing_errors.contains(&PlayingError::BpmUndefined));
    assert!(playing_errors.contains(&PlayingError::NoNotes));
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

    let rng = RngMock([BigUint::from(1u64)]);
    let BmsParseOutput {
        bms: _,
        parse_warnings,
        playing_warnings,
        playing_errors,
    } = Bms::from_token_stream(&tokens, rng, AlwaysWarnAndUseOlder);

    assert_eq!(parse_warnings, vec![]);

    // Should not have any warnings or errors for valid BMS
    assert_eq!(playing_warnings, vec![]);
    assert_eq!(playing_errors, vec![]);
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

    let rng = RngMock([BigUint::from(1u64)]);
    let BmsParseOutput {
        bms,
        parse_warnings,
        playing_warnings,
        playing_errors,
    } = Bms::from_token_stream(&tokens, rng, AlwaysWarnAndUseOlder);

    assert_eq!(parse_warnings, vec![]);

    // Should have StartBpmUndefined warning but no BpmUndefined error
    assert_eq!(bms.arrangers.bpm, None);
    assert_eq!(bms.scope_defines.bpm_defs.len(), 1);
    assert_eq!(bms.arrangers.bpm_changes.len(), 0);
    assert!(playing_errors.contains(&PlayingError::BpmUndefined));
    assert!(!playing_warnings.contains(&PlayingWarning::StartBpmUndefined));
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

    let rng = RngMock([BigUint::from(1u64)]);
    let BmsParseOutput {
        bms: _,
        parse_warnings,
        playing_warnings,
        playing_errors,
    } = Bms::from_token_stream(&tokens, rng, AlwaysWarnAndUseOlder);

    assert_eq!(parse_warnings, vec![]);

    // Should have both NoDisplayableNotes and NoPlayableNotes warnings
    assert!(playing_warnings.contains(&PlayingWarning::NoDisplayableNotes));
    assert!(playing_warnings.contains(&PlayingWarning::NoPlayableNotes));
    assert_eq!(playing_errors, vec![]);
}
