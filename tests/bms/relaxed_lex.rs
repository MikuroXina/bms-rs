use bms_rs::bms::prelude::*;

fn parse_single_line(source: &str) -> LexOutput<'_> {
    TokenStream::parse_lex(source, default_relaxers())
}

#[test]
fn lex_relaxed_rondam_as_random() {
    let LexOutput {
        tokens,
        lex_warnings,
    } = parse_single_line("#RONDAM 5\n");
    assert!(lex_warnings.is_empty());
    let rendered = format!("{}", tokens.tokens[0].content());
    assert_eq!(rendered, "#RANDOM 5");
}

#[test]
fn lex_relaxed_end_space_if_as_endif() {
    let LexOutput {
        tokens,
        lex_warnings,
    } = parse_single_line("#END IF\n");
    assert!(lex_warnings.is_empty());
    let rendered = format!("{}", tokens.tokens[0].content());
    assert_eq!(rendered, "#ENDIF");
}

#[test]
fn lex_relaxed_fullwidth_hash_endif() {
    // Fullwidth '#': U+FF03
    let LexOutput {
        tokens,
        lex_warnings,
    } = parse_single_line("＃ENDIF\n");
    assert!(lex_warnings.is_empty());
    let rendered = format!("{}", tokens.tokens[0].content());
    assert_eq!(rendered, "#ENDIF");
}

#[test]
fn lex_relaxed_random_no_space_number() {
    let LexOutput {
        tokens,
        lex_warnings,
    } = parse_single_line("#RANDOM5\n");
    assert!(lex_warnings.is_empty());
    let rendered = format!("{}", tokens.tokens[0].content());
    assert_eq!(rendered, "#RANDOM 5");
}

#[test]
fn lex_relaxed_if_no_space_number() {
    let LexOutput {
        tokens,
        lex_warnings,
    } = parse_single_line("#IF3\n");
    assert!(lex_warnings.is_empty());
    let rendered = format!("{}", tokens.tokens[0].content());
    assert_eq!(rendered, "#IF 3");
}

#[test]
fn lex_relaxed_ifend_as_endif() {
    let LexOutput {
        tokens,
        lex_warnings,
    } = parse_single_line("#IFEND\n");
    assert!(lex_warnings.is_empty());
    let rendered = format!("{}", tokens.tokens[0].content());
    assert_eq!(rendered, "#ENDIF");
}

#[test]
fn lex_relaxed_base_36_header() {
    let LexOutput {
        tokens,
        lex_warnings,
    } = parse_single_line("#BASE 36\n");
    assert!(lex_warnings.is_empty());
    let rendered = format!("{}", tokens.tokens[0].content());
    assert_eq!(rendered, "#BASE 36");
}
