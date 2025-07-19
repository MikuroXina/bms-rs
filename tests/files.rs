use std::borrow::Cow;

use bms_rs::{
    lex::{BmsLexOutput, LexWarning, parse},
    parse::{Bms, BmsParseOutput, prompt::AlwaysWarn, rng::RngMock},
};

#[test]
fn test_lal() {
    let source = include_str!("lilith_mx.bms");
    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse(source);
    assert_eq!(warnings, vec![]);
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([1]), AlwaysWarn);
    assert_eq!(parse_warnings, vec![]);
    eprintln!("{bms:?}");
}

#[test]
fn test_nc() {
    let source = include_str!("nc_mx.bme");
    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse(source);
    assert_eq!(warnings, vec![]);
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([1]), AlwaysWarn);
    assert_eq!(parse_warnings, vec![]);
    eprintln!("{bms:?}");
}

#[test]
fn test_j219() {
    let source = include_str!("J219_7key.bms");
    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse(source);
    assert_eq!(warnings, vec![]);
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([1]), AlwaysWarn);
    assert_eq!(parse_warnings, vec![]);
    eprintln!("{bms:?}");
}

#[test]
fn test_blank() {
    let source = include_str!("dive_withblank.bme");
    let BmsLexOutput {
        tokens: _,
        lex_warnings: warnings,
    } = parse(source);
    assert_eq!(
        warnings,
        vec![
            LexWarning::ExpectedToken {
                line: 19,
                col: 8,
                message: Cow::Borrowed("key audio filename")
            },
            LexWarning::ExpectedToken {
                line: 22,
                col: 7,
                message: Cow::Borrowed("key audio filename")
            }
        ]
    );
}

#[test]
fn test_bemuse_ext() {
    let source = include_str!("bemuse_ext.bms");
    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse(source);
    assert_eq!(warnings, vec![]);
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([1]), AlwaysWarn);
    assert_eq!(parse_warnings, vec![]);
    eprintln!("{bms:?}");
}
