use bms_rs::bmson::{BmsonOutput, BmsonWarning, parse::parse_bmson};

#[test]
fn test_parse_bmson_malformed_json() {
    // Test with malformed JSON
    let malformed_data = r#"{"version": "1.0.0", "info": {"title": "Test", "artist": "Test", "genre": "Test", "level": 5, "init_bpm": 120.0, "total": 100.0}, "sound_channels": []"#; // Missing closing brace

    let BmsonOutput { bmson, warnings } = parse_bmson(malformed_data);

    // Should have warnings about JSON parsing
    assert!(!warnings.is_empty());
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, BmsonWarning::JsonParsing(_)))
    );

    // Should return default BMSON
    assert_eq!(bmson.info.title, "Unknown");
    assert_eq!(bmson.info.artist, "Unknown");
    assert_eq!(bmson.info.genre, "Unknown");
    assert_eq!(bmson.info.level, 1);
}

#[test]
fn test_parse_bmson_missing_required_fields() {
    // Test with missing required fields
    let incomplete_data = r#"{"version": "1.0.0"}"#; // Missing info field

    let BmsonOutput { bmson, warnings } = parse_bmson(incomplete_data);

    // Should have warnings about missing required fields
    assert!(!warnings.is_empty());
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, BmsonWarning::MissingRequiredField(_)))
    );

    // Should use default values for missing fields
    assert_eq!(bmson.info.title, "Unknown");
    assert_eq!(bmson.info.artist, "Unknown");
    assert_eq!(bmson.info.genre, "Unknown");
    assert_eq!(bmson.info.level, 1);
}

#[test]
fn test_parse_bmson_invalid_field_types() {
    // Test with invalid field types
    let invalid_data = r#"{"version": "1.0.0", "info": {"title": 123, "artist": "Test", "genre": "Test", "level": "invalid", "init_bpm": 120.0, "total": 100.0}, "sound_channels": []}"#;

    let BmsonOutput { bmson, warnings } = parse_bmson(invalid_data);

    // Should have warnings about invalid field types
    assert!(!warnings.is_empty());
    assert!(
        warnings
            .iter()
            .any(|w| matches!(w, BmsonWarning::InvalidFieldType(_)))
    );

    // Should use default values for invalid fields
    assert_eq!(bmson.info.title, "Unknown"); // title should be default due to invalid type
    assert_eq!(bmson.info.level, 1); // level should be default due to invalid type
}

#[test]
fn test_parse_bmson_invalid_field_values() {
    // Test with invalid field values (negative BPM which might be invalid)
    let invalid_data = r#"{"version": "1.0.0", "info": {"title": "Test", "subtitle": "", "artist": "Test", "genre": "Test", "mode_hint": "beat-7k", "chart_name": "", "level": 5, "init_bpm": -120.0, "judge_rank": 100.0, "total": 100.0, "resolution": 240}, "sound_channels": []}"#;

    let BmsonOutput { bmson, warnings } = parse_bmson(invalid_data);

    // Should parse successfully (negative BPM is valid)
    assert_eq!(warnings, vec![]);
    assert_eq!(bmson.info.init_bpm.as_f64(), -120.0);
}

#[test]
fn test_parse_bmson_valid_json() {
    // Test with valid JSON
    let valid_data = r#"{"version": "1.0.0", "info": {"title": "Test Song", "subtitle": "", "artist": "Test Artist", "genre": "Test Genre", "mode_hint": "beat-7k", "chart_name": "", "level": 5, "init_bpm": 120.0, "judge_rank": 100.0, "total": 100.0, "resolution": 240}, "sound_channels": []}"#;

    let BmsonOutput { bmson, warnings } = parse_bmson(valid_data);

    // Should have no warnings
    assert_eq!(warnings, vec![]);

    // Should parse correctly
    assert_eq!(bmson.version, "1.0.0");
    assert_eq!(bmson.info.title, "Test Song");
    assert_eq!(bmson.info.artist, "Test Artist");
    assert_eq!(bmson.info.genre, "Test Genre");
    assert_eq!(bmson.info.level, 5);
    assert_eq!(bmson.info.init_bpm.as_f64(), 120.0);
    assert_eq!(bmson.info.total.as_f64(), 100.0);
}
