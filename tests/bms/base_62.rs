use bms_rs::bms::{parse::prompt::PanicAndUseNewer, prelude::*};
use pretty_assertions::assert_eq;

#[test]
fn test_not_base_62() {
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(
        r"
        #WAVaa hoge.wav
        #WAVAA fuga.wav
    ",
    );
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_errors,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _, _>(
        &tokens,
        default_config().prompter(PanicAndUseNewer),
    );
    assert_eq!(parse_errors, vec![]);
    assert_eq!(parse_warnings, vec![]);
    eprintln!("{bms:?}");
    assert_eq!(bms.wav.wav_files.len(), 1);
    assert_eq!(
        bms.wav.wav_files.iter().next().unwrap().1,
        &std::path::Path::new("fuga.wav").to_path_buf()
    );
}

#[test]
fn test_base_62() {
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(
        r"
        #WAVaa hoge.wav
        #WAVAA fuga.wav

        #BASE 62
    ",
    );
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_errors,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _, _>(
        &tokens,
        default_config().prompter(PanicAndUseNewer),
    );
    assert_eq!(parse_errors, vec![]);
    assert_eq!(parse_warnings, vec![]);
    eprintln!("{bms:?}");
    assert_eq!(bms.wav.wav_files.len(), 2);
}
