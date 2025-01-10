use bms_rs::lex::{parse, token::Token};

#[test]
fn test_cursor_with_no_ending_return_and_newline() {
    // With no "\r\n"
    let text = r"#TITLE Sample";

    let tokens = parse(text).expect("must be parsed");
    let mut tokens_iter = tokens.into_iter();
    assert_eq!(tokens_iter.next().unwrap(), Token::Title("Sample"));
    assert_eq!(tokens_iter.next(), None);
}
