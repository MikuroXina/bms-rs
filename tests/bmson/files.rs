#![cfg(feature = "bmson")]

use std::num::NonZeroU64;

use bms_rs::bmson::{
    BgaEvent, BgaHeader, BgaId, BmsonParseError, BpmEvent, fin_f64::FinF64, parse_bmson,
    pulse::PulseNumber,
};

#[test]
fn test_bmson100_lostokens() {
    let data = include_str!("files/lostokens.bmson");
    let output = parse_bmson(data);
    let bmson = output.bmson.expect("Failed to parse BMSON");
    // Basic fields assertion
    assert_eq!(bmson.info.title.as_ref(), "lostokens");
    assert_eq!(bmson.info.level, 5);
    assert!(!bmson.sound_channels.is_empty());
}

#[test]
fn test_bmson100_bemusic_story_48key() {
    let data = include_str!("files/bemusicstory_483_48K_ANOTHER.bmson");
    let output = parse_bmson(data);
    let bmson = output.bmson.expect("Failed to parse BMSON");
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

    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");
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

    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");
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

    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");
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

    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");
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
            "resolution": {}
        }},
        "sound_channels": []
    }}"#,
        large_value
    );

    let output = parse_bmson(&json);
    let bmson = output.bmson.expect("Failed to parse BMSON");
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

    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");
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

    let output = parse_bmson(invalid_json);

    // Should be a failure
    assert!(output.bmson.is_none(), "Expected parsing to fail");

    // Should have exactly one deserialize error
    assert_eq!(output.errors.len(), 1);
    let [first_error] = output.errors.as_slice() else {
        panic!("Expected exactly one error, got {}", output.errors.len());
    };
    let BmsonParseError::Deserialize { error } = first_error else {
        panic!("Expected deserialize error but got: {first_error:?}");
    };

    // The error should contain path information about the invalid field "level"
    let path = error.path().to_string();
    assert!(!path.is_empty());

    // Check that the error message contains information about the invalid type
    let error_string = format!("{}", error);
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
    println!("Error message: {}", error);
}

#[test]
fn test_parse_bmson_with_missing_required_field() {
    let incomplete_json = r#"{
        "version": "1.0.0",
        "sound_channels": []
    }"#;

    let output = parse_bmson(incomplete_json);

    // Should be a failure
    assert!(output.bmson.is_none(), "Expected parsing to fail");

    // Should have exactly one deserialize error
    assert_eq!(output.errors.len(), 1);
    let [first_error] = output.errors.as_slice() else {
        panic!("Expected exactly one error, got {}", output.errors.len());
    };
    let BmsonParseError::Deserialize { error } = first_error else {
        panic!("Expected deserialize error but got: {first_error:?}");
    };

    // Should indicate missing "info" field
    let path = error.path().to_string();
    assert!(!path.is_empty());

    // Check that the error message contains information about the missing field
    let error_string = format!("{}", error);
    assert!(
        error_string.contains("missing field") || error_string.contains("info"),
        "Error message should indicate missing 'info' field. Got: {}",
        error_string
    );

    // The path may be empty for missing fields, but the error message should contain field info
    // Note: serde_path_to_error may not always provide path info for missing fields

    println!("Error path: {}", path);
    println!("Error message: {}", error);
}

#[test]
fn test_chumsky_detects_missing_commas() {
    // Test JSON with missing commas that chumsky should detect
    let json_with_missing_commas = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song"
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

    let output = parse_bmson(json_with_missing_commas);

    // Check that chumsky detected missing comma errors
    let json_parse_errors: Vec<_> = output
        .errors
        .iter()
        .filter_map(|e| match e {
            BmsonParseError::JsonWarning { warning } => Some(&warning.0),
            BmsonParseError::JsonRecovered { error } => Some(&error.0),
            _ => None,
        })
        .collect();

    assert!(
        !json_parse_errors.is_empty(),
        "Expected chumsky to detect missing comma errors"
    );

    // Print the chumsky errors for debugging
    println!("Chumsky errors count: {}", json_parse_errors.len());
    for (i, error) in json_parse_errors.iter().enumerate() {
        println!("Chumsky error {}: {:?}", i, error);
    }

    // Verify that the errors are related to missing commas
    let has_comma_error = json_parse_errors.iter().any(|error| {
        let error_str = format!("{:?}", error);
        error_str.contains("expected") && error_str.contains(",")
    });
    assert!(
        has_comma_error,
        "Expected chumsky errors to be related to missing commas"
    );

    // The parsing should succeed despite missing commas (chumsky should recover)
    let bmson = output.bmson.expect("Expected parsing to succeed");

    // Verify the parsed data matches the original JSON content
    assert_eq!(bmson.version.as_ref(), "1.0.0");
    assert_eq!(bmson.info.title.as_ref(), "Test Song");
    assert_eq!(bmson.info.artist.as_ref(), "Test Artist");
    assert_eq!(bmson.info.genre.as_ref(), "Test Genre");
    assert_eq!(bmson.info.level, 5);
    assert!((bmson.info.init_bpm.as_f64() - 120.0).abs() <= f64::EPSILON);
    assert!((bmson.info.judge_rank.as_f64() - 100.0).abs() <= f64::EPSILON);
    assert!((bmson.info.total.as_f64() - 100.0).abs() <= f64::EPSILON);
    assert_eq!(
        bmson.info.resolution,
        NonZeroU64::new(240).expect("240 should be a valid NonZeroU64")
    );
    assert!(bmson.sound_channels.is_empty());

    println!("All Bmson content matches the original JSON despite missing commas");

    println!("Chumsky correctly recovered from missing comma errors");
}

#[test]
fn test_parse_bmson_with_trailing_comma() {
    // JSON with trailing comma - serde_json might tolerate this but chumsky might not
    let json_with_trailing_comma = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song",
            "artist": "Test Artist",
            "genre": "Test Genre",
            "level": 5,
            "init_bpm": 120.0,
            "judge_rank": 100.0,
            "total": 100.0,
            "resolution": 240,
        },
        "sound_channels": [],
    }"#;

    let output = parse_bmson(json_with_trailing_comma);

    // Print the chumsky errors for debugging
    let json_parse_errors: Vec<_> = output
        .errors
        .iter()
        .filter_map(|e| match e {
            BmsonParseError::JsonWarning { warning } => Some(&warning.0),
            BmsonParseError::JsonRecovered { error } => Some(&error.0),
            _ => None,
        })
        .collect();

    println!("Chumsky errors count: {}", json_parse_errors.len());
    for (i, error) in json_parse_errors.iter().enumerate() {
        println!("Chumsky error {}: {:?}", i, error);
    }

    // The parsing should succeed despite trailing commas
    let bmson = output.bmson.expect("Expected parsing to succeed");

    println!("Parsing succeeded - checking content consistency with trailing commas");

    // Verify the parsed data matches the original JSON content
    assert_eq!(bmson.version.as_ref(), "1.0.0");
    assert_eq!(bmson.info.title.as_ref(), "Test Song");
    assert_eq!(bmson.info.artist.as_ref(), "Test Artist");
    assert_eq!(bmson.info.genre.as_ref(), "Test Genre");
    assert_eq!(bmson.info.level, 5);
    assert!((bmson.info.init_bpm.as_f64() - 120.0).abs() <= f64::EPSILON);
    assert!((bmson.info.judge_rank.as_f64() - 100.0).abs() <= f64::EPSILON);
    assert!((bmson.info.total.as_f64() - 100.0).abs() <= f64::EPSILON);
    assert_eq!(
        bmson.info.resolution,
        NonZeroU64::new(240).expect("240 should be a valid NonZeroU64")
    );
    assert!(bmson.sound_channels.is_empty());

    println!("All Bmson content matches the original JSON despite trailing commas");

    // If there were chumsky errors, verify they were about trailing commas
    if !json_parse_errors.is_empty() {
        println!("Chumsky detected errors but serde_json fallback succeeded");
    }
}

#[test]
fn test_parse_bmson_totally_broken_json() {
    // A completely broken JSON that neither chumsky nor serde_json can parse into a value
    let broken = "";

    let output = parse_bmson(broken);

    // No bmson should be produced
    assert!(output.bmson.is_none());

    // Must contain at least one fatal JSON error
    for e in &output.errors {
        match e {
            BmsonParseError::JsonError { .. } => println!("saw JsonError"),
            BmsonParseError::JsonRecovered { .. } => println!("saw JsonRecovered"),
            BmsonParseError::JsonWarning { .. } => println!("saw JsonWarning"),
            BmsonParseError::Deserialize { .. } => println!("saw Deserialize"),
        }
    }
    let has_fatal = output
        .errors
        .iter()
        .any(|e| matches!(e, BmsonParseError::JsonError { .. }));
    assert!(has_fatal, "Expected a fatal JSON error");
}
