use bms_rs::{
    lex::{BmsLexOutput, LexWarning, parse},
    parse::{Bms, BmsParseOutput, prompt::AlwaysHalt, rng::RngMock},
};

#[test]
fn test_lal() {
    let source = include_str!("lilith_mx.bms");
    let BmsLexOutput { tokens, warnings } = parse(source);
    assert_eq!(warnings, vec![]);
    let BmsParseOutput { bms, warnings } =
        Bms::from_token_stream(&mut tokens.iter().peekable(), RngMock([1]), AlwaysHalt);
    assert_eq!(warnings, vec![]);
    eprintln!("{:?}", bms);
}

#[test]
fn test_nc() {
    let source = include_str!("nc_mx.bme");
    let BmsLexOutput { tokens, warnings } = parse(source);
    assert_eq!(warnings, vec![]);
    let BmsParseOutput { bms, warnings } =
        Bms::from_token_stream(&mut tokens.iter().peekable(), RngMock([1]), AlwaysHalt);
    assert_eq!(warnings, vec![]);
    eprintln!("{:?}", bms);
}

#[test]
fn test_j219() {
    let source = include_str!("J219_7key.bms");
    let BmsLexOutput { tokens, warnings } = parse(source);
    assert_eq!(warnings, vec![]);
    let BmsParseOutput { bms, warnings } =
        Bms::from_token_stream(&mut tokens.iter().peekable(), RngMock([1]), AlwaysHalt);
    assert_eq!(warnings, vec![]);
    eprintln!("{:?}", bms);
}

#[test]
fn test_blank() {
    let source = include_str!("dive_withblank.bme");
    let BmsLexOutput {
        tokens: _,
        warnings,
    } = parse(source);
    assert_eq!(
        warnings,
        vec![
            LexWarning::ExpectedToken {
                line: 19,
                col: 8,
                message: "key audio filename"
            },
            LexWarning::ExpectedToken {
                line: 22,
                col: 7,
                message: "key audio filename"
            }
        ]
    );
}

#[test]
fn test_bemuse_ext() {
    let source = include_str!("bemuse_ext.bms");
    let BmsLexOutput { tokens, warnings } = parse(source);
    assert_eq!(warnings, vec![]);
    let BmsParseOutput { bms, warnings } =
        Bms::from_token_stream(&mut tokens.iter().peekable(), RngMock([1]), AlwaysHalt);
    assert_eq!(warnings, vec![]);
    eprintln!("{:?}", bms);
}
