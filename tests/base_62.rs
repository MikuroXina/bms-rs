use bms_rs::bms::{
    lex::{BmsLexOutput, parse_lex_tokens},
    parse::{BmsParseOutput, model::Bms, prompt::AlwaysUseNewer, random::rng::RngMock},
};
use num::BigUint;

#[test]
fn test_not_base_62() {
    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse_lex_tokens(
        r"
        #WAVaa hoge.wav
        #WAVAA fuga.wav
    ",
    );
    assert_eq!(warnings, vec![]);
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysUseNewer);
    assert_eq!(parse_warnings, vec![]);
    eprintln!("{bms:?}");
    assert_eq!(bms.notes.wav_files.len(), 1);
    assert_eq!(
        bms.notes.wav_files.iter().next().unwrap().1,
        &std::path::Path::new("fuga.wav").to_path_buf()
    );
}

#[test]
fn test_base_62() {
    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse_lex_tokens(
        r"
        #WAVaa hoge.wav
        #WAVAA fuga.wav

        #BASE 62
    ",
    );
    assert_eq!(warnings, vec![]);
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysUseNewer);
    assert_eq!(parse_warnings, vec![]);
    eprintln!("{bms:?}");
    assert_eq!(bms.notes.wav_files.len(), 2);
}
