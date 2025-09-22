#![cfg(feature = "bmson")]

use bms_rs::bmson::{
    BgaEvent, BgaHeader, BgaId, Bmson, BpmEvent, fin_f64::FinF64, parse_bmson, pulse::PulseNumber,
};

#[test]
fn test_bmson100_lostokens() {
    let data = include_str!("files_bmson/lostokens.bmson");
    let bmson: Bmson = serde_json::from_str(data).expect("failed to parse bmson json");
    // Basic fields assertion
    assert_eq!(bmson.info.title.as_ref(), "lostokens");
    assert_eq!(bmson.info.level, 5);
    assert!(!bmson.sound_channels.is_empty());
}

#[test]
fn test_bmson100_bemusic_story_48key() {
    let data = include_str!("files_bmson/bemusicstory_483_48K_ANOTHER.bmson");
    let bmson: Bmson = serde_json::from_str(data).expect("failed to parse bmson json");
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
                bpm: FinF64::new(199.0).unwrap()
            },
            BpmEvent {
                y: PulseNumber(3500640),
                bpm: FinF64::new(200.0).unwrap()
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

    let bmson = parse_bmson(json).expect("Failed to parse BMSON");
    assert_eq!(bmson.info.title.as_ref(), "Test Song");
    assert_eq!(bmson.info.artist.as_ref(), "Test Artist");
    assert_eq!(bmson.info.level, 5);
    assert_eq!(
        bmson.info.resolution,
        std::num::NonZeroU64::new(240).unwrap()
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

    let bmson = parse_bmson(json).expect("Failed to parse BMSON");
    assert_eq!(
        bmson.info.resolution,
        std::num::NonZeroU64::new(240).unwrap()
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

    let bmson = parse_bmson(json).expect("Failed to parse BMSON");
    assert_eq!(
        bmson.info.resolution,
        std::num::NonZeroU64::new(480).unwrap()
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

    let bmson = parse_bmson(json).expect("Failed to parse BMSON");
    assert_eq!(
        bmson.info.resolution,
        std::num::NonZeroU64::new(240).unwrap()
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
            "resolution": {}
        }},
        "sound_channels": []
    }}"#,
        large_value
    );

    let bmson = parse_bmson(&json).expect("Failed to parse BMSON");
    assert_eq!(
        bmson.info.resolution,
        std::num::NonZeroU64::new(large_value).unwrap()
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

    let bmson = parse_bmson(json).expect("Failed to parse BMSON");
    assert_eq!(
        bmson.info.resolution,
        std::num::NonZeroU64::new(480).unwrap()
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

    let result = parse_bmson(invalid_json);
    assert!(result.is_err());

    let err = result.unwrap_err();
    // The error should contain path information about the invalid field "level"
    let path = err.path().to_string();
    let inner_err = err.into_inner();
    assert!(!path.is_empty());

    // Check that the error message contains information about the invalid type
    let error_string = format!("{}", inner_err);
    assert!(
        error_string.contains("invalid type") || error_string.contains("expected"),
        "Error message should indicate invalid type. Got: {}",
        error_string
    );

    // The path should contain information about the problematic field
    assert!(
        path.contains("info.level"),
        "Error path should contain 'level' field information. Got path: {}",
        path
    );

    println!("Error path: {}", path);
    println!("Error message: {}", inner_err);
}

#[test]
fn test_parse_bmson_with_missing_required_field() {
    let incomplete_json = r#"{
        "version": "1.0.0",
        "sound_channels": []
    }"#;

    let result = parse_bmson(incomplete_json);
    assert!(result.is_err());

    let err = result.unwrap_err();
    // Should indicate missing "info" field
    let path = err.path().to_string();
    let inner_err = err.into_inner();
    assert!(!path.is_empty());

    // Check that the error message contains information about the missing field
    let error_string = format!("{}", inner_err);
    assert!(
        error_string.contains("missing field") || error_string.contains("info"),
        "Error message should indicate missing 'info' field. Got: {}",
        error_string
    );

    // The path may be empty for missing fields, but the error message should contain field info
    // Note: serde_path_to_error may not always provide path info for missing fields

    println!("Error path: {}", path);
    println!("Error message: {}", inner_err);
}
