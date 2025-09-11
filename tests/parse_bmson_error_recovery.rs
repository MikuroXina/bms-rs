#![cfg(feature = "bmson")]

use bms_rs::bmson::{parse_bmson, parser::BmsonWarning};

#[test]
fn test_missing_comma_in_array() {
    // Test the case where comma is missing in array
    let json_with_missing_comma = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song",
            "artist": "Test Artist",
            "genre": "Test",
            "level": 5,
            "init_bpm": 120.0
        },
        "sound_channels": [
            {"name": "sound1.wav", "notes": [{"y": 0, "x": 1, "l": 0, "c": false}]}
            {"name": "sound2.wav", "notes": [{"y": 240, "x": 2, "l": 0, "c": false}]}
        ]
    }"#;

    let output = parse_bmson(json_with_missing_comma);

    // Should produce JSON syntax error warning along with other warnings
    assert!(!output.warnings.is_empty(), "Should have warnings");
    let has_json_syntax_error = output
        .warnings
        .iter()
        .any(|w| matches!(w, BmsonWarning::JsonSyntaxError(_)));
    assert!(
        has_json_syntax_error,
        "Should produce JSON syntax error warning"
    );

    // Verify the syntax error details
    let json_syntax_warnings: Vec<&BmsonWarning> = output
        .warnings
        .iter()
        .filter(|w| matches!(w, BmsonWarning::JsonSyntaxError(_)))
        .collect();

    assert!(
        !json_syntax_warnings.is_empty(),
        "Should have at least one JSON syntax error"
    );
    // Note: Parser may produce multiple syntax errors for complex malformed structures

    // Verify that at least one syntax error contains expected error information
    let has_valid_syntax_error = json_syntax_warnings.iter().any(|w| {
        if let BmsonWarning::JsonSyntaxError(rich) = w {
            let error_msg = rich.to_string();
            error_msg.contains("expected") || error_msg.contains("unexpected")
        } else {
            false
        }
    });
    assert!(
        has_valid_syntax_error,
        "At least one syntax error should indicate syntax issue"
    );

    // Despite errors, should be able to parse basic structure (if parser supports error recovery)
    // Mainly verify that parser doesn't crash and produces appropriate warnings
    assert!(!output.bmson.version.is_empty());
}

#[test]
fn test_missing_comma_in_object() {
    // Test the case where comma is missing in object
    let json_with_missing_comma = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song"
            "artist": "Test Artist",
            "genre": "Test",
            "level": 5,
            "init_bpm": 120.0
        },
        "sound_channels": []
    }"#;

    let output = parse_bmson(json_with_missing_comma);

    // Should produce JSON syntax error warning along with other warnings
    assert!(!output.warnings.is_empty(), "Should have warnings");
    let has_json_syntax_error = output
        .warnings
        .iter()
        .any(|w| matches!(w, BmsonWarning::JsonSyntaxError(_)));
    assert!(
        has_json_syntax_error,
        "Should produce JSON syntax error warning"
    );

    // Verify the syntax error details
    let json_syntax_warnings: Vec<&BmsonWarning> = output
        .warnings
        .iter()
        .filter(|w| matches!(w, BmsonWarning::JsonSyntaxError(_)))
        .collect();

    assert!(
        !json_syntax_warnings.is_empty(),
        "Should have at least one JSON syntax error"
    );
    // Note: Parser may produce multiple syntax errors for complex malformed structures

    // Verify that at least one syntax error contains expected error information
    let has_valid_syntax_error = json_syntax_warnings.iter().any(|w| {
        if let BmsonWarning::JsonSyntaxError(rich) = w {
            let error_msg = rich.to_string();
            error_msg.contains("expected") || error_msg.contains("unexpected")
        } else {
            false
        }
    });
    assert!(
        has_valid_syntax_error,
        "At least one syntax error should indicate syntax issue"
    );

    // Due to syntax errors, may not correctly parse info object, actual behavior may differ from expectations
    // Here we mainly verify that the parser doesn't crash and produces appropriate warnings
    assert!(!output.bmson.version.is_empty());
}

#[test]
fn test_missing_required_fields() {
    // Test the case where required fields are missing
    let json_with_missing_fields = r#"{
        "info": {
            "title": "Test Song",
            "artist": "Test Artist"
        },
        "sound_channels": []
    }"#;

    let output = parse_bmson(json_with_missing_fields);

    // Should produce exactly 4 missing field warnings
    assert_eq!(
        output.warnings.len(),
        4,
        "Should have exactly 4 warnings for missing fields"
    );

    // Collect all missing field warnings
    let missing_fields: Vec<&str> = output
        .warnings
        .iter()
        .filter_map(|w| {
            if let BmsonWarning::MissingField(field) = w {
                Some(field.as_ref())
            } else {
                None
            }
        })
        .collect();

    // Verify all expected fields are reported as missing
    assert!(
        missing_fields.contains(&"version"),
        "Should warn about missing version field"
    );
    assert!(
        missing_fields.contains(&"info.genre"),
        "Should warn about missing info.genre field"
    );
    assert!(
        missing_fields.contains(&"info.level"),
        "Should warn about missing info.level field"
    );
    assert!(
        missing_fields.contains(&"info.init_bpm"),
        "Should warn about missing info.init_bpm field"
    );

    // Verify no other types of warnings
    assert!(
        output
            .warnings
            .iter()
            .all(|w| matches!(w, BmsonWarning::MissingField(_))),
        "All warnings should be MissingField type"
    );

    // Should use default values to fill missing fields
    assert_eq!(output.bmson.version, "1.0.0");
    assert_eq!(output.bmson.info.genre, "");
    assert_eq!(output.bmson.info.level, 0);
    assert_eq!(output.bmson.info.init_bpm.as_f64(), 120.0);

    // Existing fields should be parsed correctly
    assert_eq!(output.bmson.info.title, "Test Song");
    assert_eq!(output.bmson.info.artist, "Test Artist");
}

#[test]
fn test_missing_info_object() {
    // Test the case where the entire info object is missing
    let json_with_missing_info = r#"{
        "version": "1.0.0",
        "sound_channels": []
    }"#;

    let output = parse_bmson(json_with_missing_info);

    // Should produce warnings for missing info object and its required fields
    assert!(!output.warnings.is_empty(), "Should have warnings");

    // Find the missing field warnings
    let missing_fields: Vec<&str> = output
        .warnings
        .iter()
        .filter_map(|w| {
            if let BmsonWarning::MissingField(field) = w {
                Some(field.as_ref())
            } else {
                None
            }
        })
        .collect();

    // Should at least warn about missing info object
    assert!(
        missing_fields.contains(&"info"),
        "Should warn about missing info object"
    );

    // May also warn about missing fields within the default info object
    // This depends on how the parser handles missing objects vs missing fields

    // Should use default values to fill all fields of the info object
    assert_eq!(output.bmson.info.title, "");
    assert_eq!(output.bmson.info.artist, "");
    assert_eq!(output.bmson.info.genre, "");
    assert_eq!(output.bmson.info.level, 0);
    assert_eq!(output.bmson.info.init_bpm.as_f64(), 120.0);
}

#[test]
fn test_duplicate_fields_in_object() {
    // Test the case where fields are duplicated in object
    let json_with_duplicate_fields = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song",
            "artist": "Test Artist",
            "genre": "Test",
            "level": 5,
            "init_bpm": 120.0,
            "title": "Duplicate Title",
            "artist": "Duplicate Artist"
        },
        "sound_channels": []
    }"#;

    let output = parse_bmson(json_with_duplicate_fields);

    // JSON parser usually keeps the value of the last duplicate field
    // Here we mainly verify that parsing doesn't crash and can read content correctly
    assert_eq!(output.bmson.info.title, "Duplicate Title");
    assert_eq!(output.bmson.info.artist, "Duplicate Artist");
    assert_eq!(output.bmson.info.genre, "Test");
    assert_eq!(output.bmson.info.level, 5);

    // Should not produce any warnings for duplicate fields (parser handles them gracefully)
    assert!(
        output.warnings.is_empty(),
        "Should not produce warnings for duplicate fields"
    );
}

#[test]
fn test_malformed_json_with_valid_parts() {
    // Test severely malformed JSON but with some valid parts
    let malformed_json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song",
            "artist": "Test Artist",
            "genre": "Test"
            "level": 5,
            "init_bpm": 120.0
        },
        "sound_channels": [
            {"name": "sound1.wav", "notes": [{"y": 0, "x": 1, "l": 0, "c": false}]},
            {"name": "sound2.wav", "notes": [{"y": 240, "x": 2, "l": 0, "c": false}]}
        ]
    "#; // Note: missing comma after genre

    let output = parse_bmson(malformed_json);

    // Should produce JSON syntax error warning
    assert!(!output.warnings.is_empty(), "Should have warnings");
    let has_json_syntax_error = output
        .warnings
        .iter()
        .any(|w| matches!(w, BmsonWarning::JsonSyntaxError(_)));
    assert!(
        has_json_syntax_error,
        "Should produce JSON syntax error warning"
    );

    // Verify the specific syntax error
    let json_syntax_warnings: Vec<&BmsonWarning> = output
        .warnings
        .iter()
        .filter(|w| matches!(w, BmsonWarning::JsonSyntaxError(_)))
        .collect();

    assert!(
        !json_syntax_warnings.is_empty(),
        "Should have at least one JSON syntax error"
    );

    // Despite errors, should be able to parse some valid content (if parser supports error recovery)
    // Mainly verify that parser doesn't crash and produces appropriate warnings
    assert!(!output.bmson.version.is_empty());
}

#[test]
fn test_invalid_root_type() {
    // Test the case where root node is not an object
    let invalid_root_json = r#"["invalid", "root", "type"]"#;

    let output = parse_bmson(invalid_root_json);

    // Should produce NonObjectRoot warning along with other warnings
    assert!(!output.warnings.is_empty(), "Should have warnings");

    // Verify there's a NonObjectRoot warning
    let has_non_object_root = output
        .warnings
        .iter()
        .any(|w| matches!(w, BmsonWarning::NonObjectRoot));
    assert!(
        has_non_object_root,
        "Should warn that root node is not an object"
    );

    // Verify the specific NonObjectRoot warning
    let non_object_warnings: Vec<&BmsonWarning> = output
        .warnings
        .iter()
        .filter(|w| matches!(w, BmsonWarning::NonObjectRoot))
        .collect();

    assert_eq!(
        non_object_warnings.len(),
        1,
        "Should have exactly one NonObjectRoot warning"
    );

    // Should use default values
    assert_eq!(output.bmson.version, "1.0.0");
    assert_eq!(output.bmson.info.title, "");
    assert_eq!(output.bmson.info.artist, "");
}

#[test]
fn test_empty_json() {
    // Test empty JSON
    let empty_json = r#"{}"#;

    let output = parse_bmson(empty_json);

    // Should produce multiple missing field warnings for empty JSON
    assert!(
        !output.warnings.is_empty(),
        "Should have warnings for missing fields"
    );

    // Count the number of missing field warnings
    let missing_field_count = output
        .warnings
        .iter()
        .filter(|w| matches!(w, BmsonWarning::MissingField(_)))
        .count();

    assert!(
        missing_field_count > 0,
        "Should have at least one missing field warning"
    );

    // Collect all missing field names
    let missing_fields: Vec<&str> = output
        .warnings
        .iter()
        .filter_map(|w| {
            if let BmsonWarning::MissingField(field) = w {
                Some(field.as_ref())
            } else {
                None
            }
        })
        .collect();

    // Should warn about critical missing fields
    assert!(
        missing_fields.contains(&"version"),
        "Should warn about missing version"
    );
    assert!(
        missing_fields.contains(&"sound_channels"),
        "Should warn about missing sound_channels"
    );

    // All required fields should use default values
    assert_eq!(output.bmson.version, "1.0.0");
    assert_eq!(output.bmson.info.title, "");
    assert_eq!(output.bmson.info.artist, "");
    assert_eq!(output.bmson.info.genre, "");
    assert_eq!(output.bmson.info.level, 0);
    assert_eq!(output.bmson.info.init_bpm.as_f64(), 120.0);
    assert!(output.bmson.sound_channels.is_empty());
}

#[test]
fn test_partial_recovery_with_notes() {
    // Test whether notes data can be correctly recovered in case of errors
    let json_with_partial_error = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test Song"
            "artist": "Test Artist",
            "genre": "Test",
            "level": 5,
            "init_bpm": 120.0
        },
        "sound_channels": [
            {
                "name": "sound1.wav",
                "notes": [
                    {"y": 0, "x": 1, "l": 0, "c": false},
                    {"y": 240, "x": 2, "l": 480, "c": false},
                    {"y": 960, "x": 3, "l": 0, "c": true}
                ]
            }
        ]
    }"#; // Note: missing comma in info object

    let output = parse_bmson(json_with_partial_error);

    // Should produce syntax error warning
    assert!(!output.warnings.is_empty(), "Should have warnings");

    // Verify there's at least one syntax error warning
    let has_syntax_error = output
        .warnings
        .iter()
        .any(|w| matches!(w, BmsonWarning::JsonSyntaxError(_)));
    assert!(has_syntax_error, "Should have JSON syntax error warning");

    // Despite errors, sound_channels should be parsed correctly (if parser supports partial recovery)
    // Note: actual behavior depends on chumsky's error recovery capability
    assert_eq!(output.bmson.version, "1.0.0");

    // If sound_channels can be parsed, verify its content
    if !output.bmson.sound_channels.is_empty() {
        let channel = &output.bmson.sound_channels[0];
        assert_eq!(channel.name, "sound1.wav");
        assert_eq!(channel.notes.len(), 3);
    }
}
