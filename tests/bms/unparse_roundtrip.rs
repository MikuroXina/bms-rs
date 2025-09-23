use bms_rs::bms::prelude::*;

fn roundtrip_source_bms_tokens_bms(source: &str) {
    // file -> tokens
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    // Allow warnings for files with empty resource definitions
    let _ = lex_warnings;

    // tokens -> Bms
    let ParseOutput {
        bms: bms1,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    // Allow warnings for files with empty resource definitions
    let _ = parse_warnings;

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
    // Allow warnings for files with empty resource definitions
    let _ = parse_warnings2;

    assert_eq!(bms2, bms1);
}

#[test]
fn roundtrip_lilith_mx_file_bms_tokens_bms() {
    let source = include_str!("files/lilith_mx.bms");
    roundtrip_source_bms_tokens_bms(source);
}

#[test]
fn roundtrip_bemuse_ext_file_bms_tokens_bms() {
    let source = include_str!("files/bemuse_ext.bms");
    roundtrip_source_bms_tokens_bms(source);
}

#[test]
fn roundtrip_dive_withblank_file_bms_tokens_bms() {
    let source = include_str!("files/dive_withblank.bme");
    roundtrip_source_bms_tokens_bms(source);
}

#[test]
fn roundtrip_j219_7key_file_bms_tokens_bms() {
    let source = include_str!("files/J219_7key.bms");
    roundtrip_source_bms_tokens_bms(source);
}

#[test]
fn roundtrip_nc_mx_file_bms_tokens_bms() {
    let source = include_str!("files/nc_mx.bme");
    roundtrip_source_bms_tokens_bms(source);
}
