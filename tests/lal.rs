use bms_rs::lex::parse;

#[test]
fn test_lal() {
    let source = include_str!("lilith_mx.bms");
    let ts = parse(source).expect("must be parsed");

    eprintln!("{:?}", ts.into_iter());
}
