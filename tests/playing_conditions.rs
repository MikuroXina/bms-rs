use bms_rs::{
    lex::{BmsLexOutput, parse},
    parse::{
        BmsParseOutput, PlayingError, PlayingWarning, model::Bms, prompt::AlwaysWarn,
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
    } = parse(source);
    assert_eq!(lex_warnings, vec![]);

    let rng = RngMock([BigUint::from(1u64)]);
    let BmsParseOutput {
        bms: _,
        parse_warnings,
        playing_warnings,
        playing_errors,
    } = Bms::from_token_stream(&tokens, rng, AlwaysWarn);

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
    } = parse(source);
    assert_eq!(lex_warnings, vec![]);

    let rng = RngMock([BigUint::from(1u64)]);
    let BmsParseOutput {
        bms: _,
        parse_warnings,
        playing_warnings,
        playing_errors,
    } = Bms::from_token_stream(&tokens, rng, AlwaysWarn);

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
    } = parse(source);
    assert_eq!(lex_warnings, vec![]);

    let rng = RngMock([BigUint::from(1u64)]);
    let BmsParseOutput {
        bms: _,
        parse_warnings,
        playing_warnings,
        playing_errors,
    } = Bms::from_token_stream(&tokens, rng, AlwaysWarn);

    assert_eq!(parse_warnings, vec![]);

    // Should have StartBpmUndefined warning but no BpmUndefined error
    assert!(playing_warnings.contains(&PlayingWarning::StartBpmUndefined));
    assert!(!playing_errors.contains(&PlayingError::BpmUndefined));
}

#[test]
fn test_playing_conditions_invisible_notes_only() {
    // Create a BMS content with only invisible notes
    let source =
        "#PLAYER 1\n#GENRE Test\n#TITLE Test\n#ARTIST Test\n#BPM 120\n#TOTAL 100\n#00131:0101";
    let BmsLexOutput {
        tokens,
        lex_warnings,
    } = parse(source);
    assert_eq!(lex_warnings, vec![]);

    let rng = RngMock([BigUint::from(1u64)]);
    let BmsParseOutput {
        bms: _,
        parse_warnings,
        playing_warnings,
        playing_errors,
    } = Bms::from_token_stream(&tokens, rng, AlwaysWarn);

    assert_eq!(parse_warnings, vec![]);

    // Should have both NoDisplayableNotes and NoPlayableNotes warnings
    assert!(playing_warnings.contains(&PlayingWarning::NoDisplayableNotes));
    assert!(playing_warnings.contains(&PlayingWarning::NoPlayableNotes));
    assert_eq!(playing_errors, vec![]);
}
