use bms_rs::bms::{
    command::channel::{Channel, ChannelIdParseWarning},
    error::ParseWarning,
    prelude::*,
};
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

        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _, _>(
        &tokens,
        default_config().prompter(AlwaysUseNewer),
    );
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

        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _, _>(
        &tokens,
        default_config().prompter(AlwaysUseNewer),
    );
    assert_eq!(parse_warnings, vec![]);
    eprintln!("{bms:?}");
    assert_eq!(bms.wav.wav_files.len(), 2);
}

#[test]
fn test_channel_id_parse_warning() {
    // Test read_channel_with_warning with invalid channel ID containing '@'
    let result = read_channel_with_warning("@B");
    assert!(
        result.is_err(),
        "read_channel_with_warning(\"@B\") should return an error"
    );
    if let Err(ParseWarning::ChannelId(ChannelIdParseWarning::InvalidAsBase62(s))) = result {
        assert_eq!(s, "@B");
    } else {
        panic!(
            "Expected ChannelIdParseWarning::InvalidAsBase62(\"@B\"), got {:?}",
            result
        );
    }

    // Test read_channel_with_warning with channel ID that's too short
    let result = read_channel_with_warning("A");
    assert!(
        result.is_err(),
        "read_channel_with_warning(\"A\") should return an error"
    );
    if let Err(ParseWarning::ChannelId(ChannelIdParseWarning::ExpectedTwoAsciiChars(s))) = result {
        assert_eq!(s, "A");
    } else {
        panic!(
            "Expected ChannelIdParseWarning::ExpectedTwoAsciiChars(\"A\"), got {:?}",
            result
        );
    }

    // Test read_channel_with_warning with valid channel ID
    let result = read_channel_with_warning("AB");
    assert!(
        result.is_ok(),
        "read_channel_with_warning(\"AB\") should succeed"
    );
    assert!(matches!(result.unwrap(), Channel::Note { .. }));
}
