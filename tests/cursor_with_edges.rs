use bms_rs::lex::{BmsLexOutput, parse, token::Token};

#[test]
fn test_cursor_with_no_ending_return_and_newline() {
    // With no "\r\n"
    let text = r"#TITLE Sample";

    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse(text);
    assert_eq!(warnings, vec![]);
    let mut tokens_iter = tokens.into_iter();
    assert_eq!(tokens_iter.next().unwrap(), Token::Title("Sample"));
    assert_eq!(tokens_iter.next(), None);
}
