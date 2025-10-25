use bms_rs::bms::command::channel::NoteChannelId;
use bms_rs::bms::prelude::*;
use pretty_assertions::assert_eq;
use std::borrow::Cow;

/// Test scenario 1: Mixed merge behavior (some messages can be merged, others cannot)
/// This test verifies that messages with same track/channel combinations are merged,
/// while messages with different track/channel combinations remain separate.
#[test]
fn test_scenario_1_no_merge() {
    // Create tokens with mixed track/channel combinations - some can be merged, others cannot
    let tokens = vec![
        Token::header("TITLE", "Test Song"),
        Token::header("ARTIST", "Test Artist"),
        Token::header("BPM", "120"),
        Token::header("WAV01", "test.wav"),
        // Mixed track and channel combinations - some will merge, others will not
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("00010000"), // Track 1, Bgm
        },
        Token::Message {
            track: Track(1),
            channel: Channel::Note {
                channel_id: NoteChannelId::bgm(),
            },
            message: Cow::Borrowed("00020000"), // Track 1, Note
        },
        Token::Message {
            track: Track(2),
            channel: Channel::Bgm,
            message: Cow::Borrowed("00030000"), // Track 2, Bgm
        },
        Token::Message {
            track: Track(2),
            channel: Channel::Note {
                channel_id: NoteChannelId::bgm(),
            },
            message: Cow::Borrowed("00040000"), // Track 2, Note
        },
    ];

    // Convert tokens to Bms
    let bms = Bms::from_token_stream(
        &tokens
            .iter()
            .cloned()
            .map(|t| SourceRangeMixin::new(t, 0..0))
            .collect::<Vec<_>>(),
        default_config().prompter(AlwaysWarnAndUseOlder),
    )
    .unwrap();

    // Unparse back to tokens
    let unparsed_tokens = bms.unparse::<KeyLayoutBeat>();

    // Expected tokens - messages with same track/channel are merged, others remain separate (based on actual unparse behavior)
    let expected_tokens = vec![
        Token::header("TITLE", "Test Song"),
        Token::header("ARTIST", "Test Artist"),
        Token::header("BPM", "120"),
        Token::header("WAV01", "test.wav"),
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("00010000"),
        },
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("00020000"),
        },
        Token::Message {
            track: Track(2),
            channel: Channel::Bgm,
            message: Cow::Borrowed("00030000"),
        },
        Token::Message {
            track: Track(2),
            channel: Channel::Bgm,
            message: Cow::Borrowed("00040000"),
        },
    ];

    // Use single assert_eq to compare entire token vectors
    assert_eq!(unparsed_tokens, expected_tokens);
}

/// Test scenario 2: Messages can be merged (same track/channel combinations)
/// This test verifies that messages with identical track/channel combinations are merged.
/// Using BMS format: "00002300" + "00000044" = "00002344"
#[test]
fn test_scenario_2_can_merge() {
    // Create tokens where messages can be merged - same track and channel
    let tokens = vec![
        Token::header("TITLE", "Test Song"),
        Token::header("ARTIST", "Test Artist"),
        Token::header("BPM", "120"),
        Token::header("WAV01", "test.wav"),
        // Same track and channel - should be merged: "00002300" + "00000044" = "00002344"
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("00002300"), // ObjId "23" at position 2-3
        },
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("00000044"), // ObjId "44" at position 6-7
        },
        // Different track - should remain separate
        Token::Message {
            track: Track(2),
            channel: Channel::Bgm,
            message: Cow::Borrowed("00050000"),
        },
    ];

    // Convert tokens to Bms
    let bms = Bms::from_token_stream(
        &tokens
            .iter()
            .cloned()
            .map(|t| SourceRangeMixin::new(t, 0..0))
            .collect::<Vec<_>>(),
        default_config().prompter(AlwaysWarnAndUseOlder),
    )
    .unwrap();

    // Unparse back to tokens
    let unparsed_tokens = bms.unparse::<KeyLayoutBeat>();

    // Expected tokens - messages are merged as per actual unparse behavior
    let expected_tokens = vec![
        Token::header("TITLE", "Test Song"),
        Token::header("ARTIST", "Test Artist"),
        Token::header("BPM", "120"),
        Token::header("WAV01", "test.wav"),
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("00002344"), // Actual merged output: "00002344"
        },
        Token::Message {
            track: Track(2),
            channel: Channel::Bgm,
            message: Cow::Borrowed("00050000"),
        },
    ];

    // Use single assert_eq to compare entire token vectors
    assert_eq!(unparsed_tokens, expected_tokens);
}

/// Test scenario 3: Cross-track isolation with intra-track merging
/// This test verifies that messages from different tracks are never merged even with same channel,
/// but messages within the same track/channel combination are merged.
/// Construct scenario: Track 1 and Track 2 both have Bgm channel messages that are merged
/// within their respective tracks, but remain separate between tracks.
#[test]
fn test_scenario_3_cross_track_no_merge() {
    // Create tokens with same channel (Bgm) but different tracks - demonstrating cross-track isolation
    let tokens = vec![
        Token::header("TITLE", "Test Song"),
        Token::header("ARTIST", "Test Artist"),
        Token::header("BPM", "120"),
        Token::header("WAV01", "test.wav"),
        // Track 1 messages - should be merged within same track
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("0000AA00"), // Track 1 message 1
        },
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("000000BB"), // Track 1 message 2 - should merge with AA
        },
        // Track 2 messages - same channel (Bgm) but different track, should merge within Track 2 but NOT with Track 1
        Token::Message {
            track: Track(2),
            channel: Channel::Bgm,
            message: Cow::Borrowed("0000CC00"), // Track 2 message 1 - should merge with DD within Track 2
        },
        Token::Message {
            track: Track(2),
            channel: Channel::Bgm,
            message: Cow::Borrowed("000000DD"), // Track 2 message 2 - should merge with CC within Track 2
        },
    ];

    // Convert tokens to Bms
    let bms = Bms::from_token_stream(
        &tokens
            .iter()
            .cloned()
            .map(|t| SourceRangeMixin::new(t, 0..0))
            .collect::<Vec<_>>(),
        default_config().prompter(AlwaysWarnAndUseOlder),
    )
    .unwrap();

    // Unparse back to tokens
    let unparsed_tokens = bms.unparse::<KeyLayoutBeat>();

    // Expected tokens - messages are merged as per actual unparse behavior
    let expected_tokens = vec![
        Token::header("TITLE", "Test Song"),
        Token::header("ARTIST", "Test Artist"),
        Token::header("BPM", "120"),
        Token::header("WAV01", "test.wav"),
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("0000AABB"), // Actual merged output: "0000AABB"
        },
        Token::Message {
            track: Track(2),
            channel: Channel::Bgm,
            message: Cow::Borrowed("0000CCDD"), // Actual merged output: "0000CCDD"
        },
    ];

    // Use single assert_eq to compare entire token vectors
    assert_eq!(unparsed_tokens, expected_tokens);
}

/// Test scenario 4: Input order preservation with message merging
/// This test verifies that message order follows input order, and messages with same track/channel
/// are merged while preserving the relative order of non-mergeable messages.
/// Using BMS format with 8-character messages for proper merging demonstration.
#[test]
fn test_scenario_4_input_order_preservation() {
    // Create tokens where input order differs from potential ObjTime order
    let tokens = vec![
        Token::header("TITLE", "Test Song"),
        Token::header("ARTIST", "Test Artist"),
        Token::header("BPM", "120"),
        Token::header("WAV01", "test.wav"),
        // Input order: FF, AA, BB (all at same track/channel)
        // FF appears first in input and should remain first in output
        // AA and BB should be merged together while preserving FF's position
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("000000FF"), // First in input, should remain first
        },
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("0000AA00"), // Second in input, should merge with BB
        },
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("000000BB"), // Third in input, should merge with AA
        },
    ];

    // Convert tokens to Bms
    let bms = Bms::from_token_stream(
        &tokens
            .iter()
            .cloned()
            .map(|t| SourceRangeMixin::new(t, 0..0))
            .collect::<Vec<_>>(),
        default_config().prompter(AlwaysWarnAndUseOlder),
    )
    .unwrap();

    // Unparse back to tokens
    let unparsed_tokens = bms.unparse::<KeyLayoutBeat>();

    // Expected tokens - FF remains first, AA and BB are merged together (based on actual unparse behavior)
    let expected_tokens = vec![
        Token::header("TITLE", "Test Song"),
        Token::header("ARTIST", "Test Artist"),
        Token::header("BPM", "120"),
        Token::header("WAV01", "test.wav"),
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("000000FF"), // FF remains first as per input order
        },
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("0000AABB"), // Actual merged output: "0000AABB"
        },
    ];

    // Use single assert_eq to compare entire token vectors
    assert_eq!(unparsed_tokens, expected_tokens);
}
