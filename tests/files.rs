use bms_rs::{
    lex::{parse, LexError},
    parse::{rng::RngMock, Bms, ParseError},
};

#[test]
fn test_lal() {
    let source = include_str!("lilith_mx.bms");
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1])).expect("must be parsed");
    eprintln!("{:?}", bms);
}

#[test]
fn test_nc() {
    let source = include_str!("nc_mx.bme");
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1])).expect("must be parsed");
    eprintln!("{:?}", bms);
}

#[test]
fn test_j219() {
    let source = include_str!("J219_7key.bms");
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1])).expect("must be parsed");
    eprintln!("{:?}", bms);
}

#[test]
fn test_blank() {
    let source = include_str!("dive_withblank.bme");
    let ts = parse(source);
    assert_eq!(ts.is_err(), true);
}
