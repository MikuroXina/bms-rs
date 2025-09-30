use bms_rs::bms::lex::{LexOutput, TokenStream, token::Token};

#[test]
fn test_comment() {
    let text = r"
    #Comment This is a comment
    This is another comment
    This is the third comment💖

    This is the fourth comment";

    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(text);
    assert_eq!(warnings, vec![]);
    let mut ts_iter = tokens.tokens.into_iter();
    assert_eq!(
        ts_iter.next().unwrap().content(),
        &Token::NonCommand("This is a comment")
    );
    assert_eq!(
        ts_iter.next().unwrap().content(),
        &Token::NotACommand("This is another comment")
    );
    assert_eq!(
        ts_iter.next().unwrap().content(),
        &Token::NotACommand("This is the third comment💖")
    );
    assert_eq!(
        ts_iter.next().unwrap().content(),
        &Token::NotACommand("This is the fourth comment")
    );
    assert_eq!(ts_iter.next(), None);
}
