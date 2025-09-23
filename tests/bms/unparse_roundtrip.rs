use bms_rs::bms::prelude::*;

#[test]
fn roundtrip_lilith_mx_file_bms_tokens_bms() {
    let source = include_str!("files/lilith_mx.bms");

    // file -> tokens
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);

    // tokens -> Bms
    let ParseOutput {
        bms: bms1,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    // Bms -> tokens (unparse)
    let tokens2 = bms1.unparse();
    let tokens2_wrapped: Vec<TokenWithRange<'_>> = tokens2
        .into_iter()
        .map(|t| SourceRangeMixin::new(t, 0..0))
        .collect();

    // tokens -> Bms
    let ParseOutput {
        bms: bms2,
        parse_warnings: parse_warnings2,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens2_wrapped, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings2, vec![]);

    assert_eq!(bms2, bms1);
}
