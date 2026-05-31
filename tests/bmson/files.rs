#![cfg(feature = "bmson")]

use std::num::NonZeroU64;

use bms_rs::bmson::{BgaEvent, BgaHeader, BgaId, Bmson, BpmEvent, pulse::PulseNumber};

use strict_num_extended::PositiveF64;

#[test]
fn test_bmson100_lostokens() {
    let data = include_str!("files/lostokens.bmson");
    let bmson = serde_json::from_str::<Bmson>(data).expect("Failed to parse BMSON");
    // Basic fields assertion
    assert_eq!(bmson.info.title.as_ref(), "lostokens");
    assert_eq!(bmson.info.level, 5);
    assert!(!bmson.sound_channels.is_empty());
}

#[test]
fn test_bmson100_bemusic_story_48key() {
    let data = include_str!("files/bemusicstory_483_48K_ANOTHER.bmson");
    let bmson = serde_json::from_str::<Bmson>(data).expect("Failed to parse BMSON");
    // Basic fields assertion
    assert_eq!(bmson.info.title.as_ref(), "BE-MUSiC⇒STORY");
    // Bga
    assert_eq!(
        bmson.bga.bga_header,
        vec![BgaHeader {
            id: BgaId(1),
            name: std::borrow::Cow::Borrowed("_BGA.mp4")
        }]
    );
    assert_eq!(
        bmson.bga.bga_events,
        vec![BgaEvent {
            y: PulseNumber(31680),
            id: BgaId(1)
        }]
    );
    // Bpm Events
    assert_eq!(
        bmson.bpm_events,
        vec![
            BpmEvent {
                y: PulseNumber(31680),
                bpm: PositiveF64::new(199.0).unwrap()
            },
            BpmEvent {
                y: PulseNumber(3500640),
                bpm: PositiveF64::new(200.0).unwrap()
            }
        ]
    );
}

#[test]
fn test_parse_bmson_success() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song",
            "artist": "Test Artist",
            "genre": "Test Genre",
            "level": 5,
            "init_bpm": 120.0,
            "judge_rank": 100.0,
            "total": 100.0,
            "resolution": 240
        },
        "sound_channels": []
    }"#;

    let bmson = serde_json::from_str::<Bmson>(json).expect("Failed to parse BMSON");
    assert_eq!(bmson.info.title.as_ref(), "Test Song");
    assert_eq!(bmson.info.artist.as_ref(), "Test Artist");
    assert_eq!(bmson.info.level, 5);
    assert_eq!(
        bmson.info.resolution,
        NonZeroU64::new(240).expect("240 should be a valid NonZeroU64")
    );
}

#[test]
fn test_parse_bmson_with_zero_resolution() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song",
            "artist": "Test Artist",
            "genre": "Test",
            "level": 1,
            "init_bpm": 120,
            "resolution": 0
        },
        "sound_channels": []
    }"#;

    let bmson = serde_json::from_str::<Bmson>(json).expect("Failed to parse BMSON");
    assert_eq!(
        bmson.info.resolution,
        NonZeroU64::new(240).expect("240 should be a valid NonZeroU64")
    );
}

#[test]
fn test_parse_bmson_with_negative_resolution() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song",
            "artist": "Test Artist",
            "genre": "Test",
            "level": 1,
            "init_bpm": 120,
            "resolution": -480
        },
        "sound_channels": []
    }"#;

    let bmson = serde_json::from_str::<Bmson>(json).expect("Failed to parse BMSON");
    assert_eq!(
        bmson.info.resolution,
        NonZeroU64::new(480).expect("480 should be a valid NonZeroU64")
    );
}

#[test]
fn test_parse_bmson_with_missing_resolution() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song",
            "artist": "Test Artist",
            "genre": "Test",
            "level": 1,
            "init_bpm": 120
        },
        "sound_channels": []
    }"#;

    let bmson = serde_json::from_str::<Bmson>(json).expect("Failed to parse BMSON");
    assert_eq!(
        bmson.info.resolution,
        NonZeroU64::new(240).expect("240 should be a valid NonZeroU64")
    );
}

#[test]
fn test_parse_bmson_with_large_resolution() {
    // Test with a value larger than i64::MAX but within u64::MAX
    let large_value = 10000000000000000000u64; // 10^19, larger than i64::MAX
    let json = format!(
        r#"{{
        "version": "1.0.0",
        "info": {{
            "title": "Test Song",
            "artist": "Test Artist",
            "genre": "Test",
            "level": 1,
            "init_bpm": 120,
            "resolution": {large_value}
        }},
        "sound_channels": []
    }}"#
    );

    let bmson = serde_json::from_str::<Bmson>(&json).expect("Failed to parse BMSON");
    assert_eq!(
        bmson.info.resolution,
        NonZeroU64::new(large_value).expect("large_value should be a valid NonZeroU64")
    );
}

#[test]
fn test_parse_bmson_with_float_resolution() {
    // Test with a float value that represents a whole number
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song",
            "artist": "Test Artist",
            "genre": "Test",
            "level": 1,
            "init_bpm": 120,
            "resolution": 480.0
        },
        "sound_channels": []
    }"#;

    let bmson = serde_json::from_str::<Bmson>(json).expect("Failed to parse BMSON");
    assert_eq!(
        bmson.info.resolution,
        NonZeroU64::new(480).expect("480 should be a valid NonZeroU64")
    );
}

#[test]
fn test_parse_bmson_with_invalid_json() {
    let invalid_json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song",
            "artist": "Test Artist",
            "genre": "Test Genre",
            "level": "invalid_level",
            "init_bpm": 120.0,
            "judge_rank": 100.0,
            "total": 100.0,
            "resolution": 240
        },
        "sound_channels": []
    }"#;

    let result = serde_json::from_str::<Bmson>(invalid_json);

    // Should be a failure
    assert!(result.is_err(), "Expected parsing to fail");

    // The error should mention the problematic field
    let error_string = format!("{}", result.unwrap_err());
    assert!(
        error_string.contains("invalid type") || error_string.contains("expected"),
        "Error message should indicate invalid type. Got: {error_string}"
    );
}

#[test]
fn test_parse_bmson_with_missing_required_field() {
    let incomplete_json = r#"{
        "version": "1.0.0",
        "sound_channels": []
    }"#;

    let result = serde_json::from_str::<Bmson>(incomplete_json);

    // Should be a failure
    assert!(result.is_err(), "Expected parsing to fail");

    // Check that the error message contains information about the missing field
    let error_string = format!("{}", result.unwrap_err());
    assert!(
        error_string.contains("missing field") || error_string.contains("info"),
        "Error message should indicate missing 'info' field. Got: {error_string}"
    );
}
