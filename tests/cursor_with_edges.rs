use bms_rs::bms::lex::{BmsLexOutput, parse_lex_tokens, token::Token};

#[test]
fn test_cursor_with_no_ending_return_and_newline() {
    // With no "\r\n"
    let text = r"#TITLE Sample";

    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse_lex_tokens(text);
    assert_eq!(warnings, vec![]);
    let mut tokens_iter = tokens.into_iter();
    assert_eq!(tokens_iter.next().unwrap(), Token::Title("Sample"));
    assert_eq!(tokens_iter.next(), None);
}
