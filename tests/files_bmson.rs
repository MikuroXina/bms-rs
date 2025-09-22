#![cfg(feature = "bmson")]

use bms_rs::bmson::{
    BgaEvent, BgaHeader, BgaId, BmsonParseError, BpmEvent, fin_f64::FinF64, parse_bmson,
    pulse::PulseNumber,
};

#[test]
fn test_bmson100_lostokens() {
    let data = include_str!("files_bmson/lostokens.bmson");
    let output = parse_bmson(data);
    let bmson = output.bmson.expect("Failed to parse BMSON");
    // Basic fields assertion
    assert_eq!(bmson.info.title.as_ref(), "lostokens");
    assert_eq!(bmson.info.level, 5);
    assert!(!bmson.sound_channels.is_empty());
}

#[test]
fn test_bmson100_bemusic_story_48key() {
    let data = include_str!("files_bmson/bemusicstory_483_48K_ANOTHER.bmson");
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
    assert_eq!(bmson.info.resolution, 240);
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
    let BmsonParseError::Deserialize { error } = &output.errors[0] else {
        panic!("Expected deserialize error but got: {:?}", output.errors[0]);
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
    let BmsonParseError::Deserialize { error } = &output.errors[0] else {
        panic!("Expected deserialize error but got: {:?}", output.errors[0]);
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
            BmsonParseError::JsonParse { error } => Some(error),
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
    assert_eq!(bmson.info.init_bpm.as_f64(), 120.0);
    assert_eq!(bmson.info.judge_rank.as_f64(), 100.0);
    assert_eq!(bmson.info.total.as_f64(), 100.0);
    assert_eq!(bmson.info.resolution, 240);
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
            BmsonParseError::JsonParse { error } => Some(error),
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
    assert_eq!(bmson.info.init_bpm.as_f64(), 120.0);
    assert_eq!(bmson.info.judge_rank.as_f64(), 100.0);
    assert_eq!(bmson.info.total.as_f64(), 100.0);
    assert_eq!(bmson.info.resolution, 240);
    assert!(bmson.sound_channels.is_empty());

    println!("All Bmson content matches the original JSON despite trailing commas");

    // If there were chumsky errors, verify they were about trailing commas
    if !json_parse_errors.is_empty() {
        println!("Chumsky detected errors but serde_json fallback succeeded");
    }
}
