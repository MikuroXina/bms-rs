use bms_rs::{
    lex::{LexError, parse},
    parse::{Bms, prompt::AlwaysHalt, rng::RngMock},
};

#[test]
fn test_lal() {
    let source = include_str!("lilith_mx.bms");
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysHalt).expect("must be parsed");
    eprintln!("{:?}", bms);
}

#[test]
fn test_nc() {
    let source = include_str!("nc_mx.bme");
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysHalt).expect("must be parsed");
    eprintln!("{:?}", bms);
}

#[test]
fn test_j219() {
    let source = include_str!("J219_7key.bms");
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysHalt).expect("must be parsed");
    eprintln!("{:?}", bms);
}

#[test]
fn test_blank() {
    let source = include_str!("dive_withblank.bme");
    let ts = parse(source);
    assert_eq!(
        ts.err(),
        Some(LexError::ExpectedToken {
            line: 19,
            col: 8,
            message: "key audio filename"
        })
    );
}

#[test]
fn test_bemuse_ext() {
    let source = include_str!("bemuse_ext.bms");
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]), AlwaysHalt).expect("must be parsed");
    eprintln!("{:?}", bms);
}
