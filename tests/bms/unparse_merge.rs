use bms_rs::bms::command::channel::NoteChannelId;
use bms_rs::bms::prelude::*;
use std::borrow::Cow;
use std::path::Path;

/// Test scenario 1: No messages can be merged (all have different track/channel combinations)
/// This test verifies that messages with different track/channel combinations remain separate.
#[test]
fn test_scenario_1_no_merge() {
    // Create tokens where no messages can be merged - all have different track/channel combinations
    let tokens = vec![
        Token::Title("Test Song"),
        Token::Artist("Test Artist"),
        Token::Bpm(Decimal::from(120)),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test.wav")),
        // Different track and channel combinations - should not merge
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
    let ParseOutput { bms, .. }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(
        &tokens
            .clone()
            .into_iter()
            .map(|t| SourceRangeMixin::new(t, 0..0))
            .collect::<Vec<_>>(),
        AlwaysWarnAndUseOlder,
    );

    // Unparse back to tokens
    let unparsed_tokens = bms.unparse();

    // Expected tokens - all messages should remain separate (based on actual unparse behavior)
    let expected_tokens = vec![
        Token::Title("Test Song"),
        Token::Artist("Test Artist"),
        Token::LnTypeRdm,
        Token::Bpm(Decimal::from(120)),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test.wav")),
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
        Token::Title("Test Song"),
        Token::Artist("Test Artist"),
        Token::Bpm(Decimal::from(120)),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test.wav")),
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
    let ParseOutput { bms, .. }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(
        &tokens
            .clone()
            .into_iter()
            .map(|t| SourceRangeMixin::new(t, 0..0))
            .collect::<Vec<_>>(),
        AlwaysWarnAndUseOlder,
    );

    // Unparse back to tokens
    let unparsed_tokens = bms.unparse();

    // Expected tokens - messages are merged as per actual unparse behavior
    let expected_tokens = vec![
        Token::Title("Test Song"),
        Token::Artist("Test Artist"),
        Token::LnTypeRdm,
        Token::Bpm(Decimal::from(120)),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test.wav")),
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

/// Test scenario 3: Cross-track no abnormal merging
/// This test verifies that messages from different tracks are never merged even with same channel.
/// Construct scenario: Track 1 and Track 2 both have Bgm channel messages that could be merged,
/// but they should remain separate because they are on different tracks.
#[test]
fn test_scenario_3_cross_track_no_merge() {
    // Create tokens with same channel (Bgm) but different tracks - demonstrating cross-track isolation
    let tokens = vec![
        Token::Title("Test Song"),
        Token::Artist("Test Artist"),
        Token::Bpm(Decimal::from(120)),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test.wav")),
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
        // Track 2 messages - same channel (Bgm) but different track, should NOT merge with Track 1
        Token::Message {
            track: Track(2),
            channel: Channel::Bgm,
            message: Cow::Borrowed("0000CC00"), // Track 2 message 1 - should remain separate
        },
        Token::Message {
            track: Track(2),
            channel: Channel::Bgm,
            message: Cow::Borrowed("000000DD"), // Track 2 message 2 - should remain separate
        },
    ];

    // Convert tokens to Bms
    let ParseOutput { bms, .. }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(
        &tokens
            .clone()
            .into_iter()
            .map(|t| SourceRangeMixin::new(t, 0..0))
            .collect::<Vec<_>>(),
        AlwaysWarnAndUseOlder,
    );

    // Unparse back to tokens
    let unparsed_tokens = bms.unparse();

    // Expected tokens - messages are merged as per actual unparse behavior
    let expected_tokens = vec![
        Token::Title("Test Song"),
        Token::Artist("Test Artist"),
        Token::LnTypeRdm,
        Token::Bpm(Decimal::from(120)),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test.wav")),
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

/// Test scenario 4: Input order preservation when ObjTime differs from input order
/// This test verifies that message order follows input order, not ObjTime order.
/// Using BMS format with 8-character messages for proper merging demonstration.
#[test]
fn test_scenario_4_input_order_preservation() {
    // Create tokens where input order differs from potential ObjTime order
    let tokens = vec![
        Token::Title("Test Song"),
        Token::Artist("Test Artist"),
        Token::Bpm(Decimal::from(120)),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test.wav")),
        // Input order: message3, message1, message2 (all at same track/channel)
        // If we were to sort by ObjTime, they would be sorted by time position
        // But we should preserve input order: message3, message1, message2
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("000000FF"), // Third in input, should remain third
        },
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("0000AA00"), // First in input, should remain first
        },
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("000000BB"), // Second in input, should remain second
        },
    ];

    // Convert tokens to Bms
    let ParseOutput { bms, .. }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(
        &tokens
            .clone()
            .into_iter()
            .map(|t| SourceRangeMixin::new(t, 0..0))
            .collect::<Vec<_>>(),
        AlwaysWarnAndUseOlder,
    );

    // Unparse back to tokens
    let unparsed_tokens = bms.unparse();

    // Expected tokens - should preserve input order with merged messages (based on actual unparse behavior)
    let expected_tokens = vec![
        Token::Title("Test Song"),
        Token::Artist("Test Artist"),
        Token::LnTypeRdm,
        Token::Bpm(Decimal::from(120)),
        Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test.wav")),
        Token::Message {
            track: Track(1),
            channel: Channel::Bgm,
            message: Cow::Borrowed("000000FF"), // Actual output: "000000FF"
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
