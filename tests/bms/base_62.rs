use bms_rs::bms::prelude::*;

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
        default_relaxers(),
    );
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysUseNewer);
    assert_eq!(parse_warnings, vec![]);
    eprintln!("{bms:?}");
    assert_eq!(bms.notes().wav_files.len(), 1);
    assert_eq!(
        bms.notes().wav_files.iter().next().unwrap().1,
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
        default_relaxers(),
    );
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysUseNewer);
    assert_eq!(parse_warnings, vec![]);
    eprintln!("{bms:?}");
    assert_eq!(bms.notes().wav_files.len(), 2);
}
