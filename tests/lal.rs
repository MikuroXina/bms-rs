use bms_rs::{
    lex::parse,
    parse::{rng::RngMock, Bms},
};

#[test]
fn test_lal() {
    let source = include_str!("lilith_mx.bms");
    let ts = parse(source).expect("must be parsed");
    let bms = Bms::from_token_stream(&ts, RngMock([1]));
    eprintln!("{:?}", bms);
}
