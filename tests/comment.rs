use bms_rs::lex::{parse, token::Token};

#[test]
fn test_comment() {
    let text = r"
    #Comment This is a comment
    This is another comment
    This is the third comment💖

    This is the fourth comment";

    let ts = parse(text).expect("must be parsed");
    let mut ts_iter = ts.into_iter();
    assert_eq!(ts_iter.next().unwrap(), Token::Comment("This is a comment"));
    assert_eq!(
        ts_iter.next().unwrap(),
        Token::NotACommand("This is another comment".to_string())
    );
    assert_eq!(
        ts_iter.next().unwrap(),
        Token::NotACommand("This is the third comment💖".to_string())
    );
    assert_eq!(
        ts_iter.next().unwrap(),
        Token::NotACommand("This is the fourth comment".to_string())
    );
    assert_eq!(ts_iter.next(), None);
}
