//! Example: Parse BMSON files with fault tolerance
//!
//! This example demonstrates how to use the `parse_bmson` function to parse
//! BMSON files with fault tolerance. The parser will continue parsing even
//! when encountering JSON format errors or missing/invalid fields, issuing
//! warnings instead of failing.

use bms_rs::bmson::{BmsonOutput, parse_bmson};

fn main() {
    println!("BMSON Parser Example");
    println!("===================");
    println!();

    // Example 1: Valid BMSON
    println!("1. Parsing valid BMSON:");
    let valid_bmson = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Example Song",
            "subtitle": "Example Subtitle",
            "artist": "Example Artist",
            "genre": "Example Genre",
            "mode_hint": "beat-7k",
            "chart_name": "NORMAL",
            "level": 5,
            "init_bpm": 120.0,
            "judge_rank": 100.0,
            "total": 100.0,
            "resolution": 240
        },
        "sound_channels": []
    }"#;

    let BmsonOutput { bmson, warnings } = parse_bmson(valid_bmson);

    if warnings.is_empty() {
        println!("✅ Parsed successfully without warnings");
        println!("   Title: {}", bmson.info.title);
        println!("   Artist: {}", bmson.info.artist);
        println!("   Level: {}", bmson.info.level);
        println!("   BPM: {}", bmson.info.init_bpm.as_f64());
    } else {
        println!("⚠️  Parsed with warnings:");
        for warning in &warnings {
            println!("   - {}", warning);
        }
    }
    println!();

    // Example 2: Malformed JSON
    println!("2. Parsing malformed JSON:");
    let malformed_json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Broken Song",
            "artist": "Broken Artist"
        }
        "sound_channels": []
    }"#; // Missing comma after info object

    let BmsonOutput { bmson, warnings } = parse_bmson(malformed_json);

    println!("⚠️  Parsed with warnings:");
    for warning in &warnings {
        println!("   - {}", warning);
    }
    println!("   Using default values:");
    println!("   Title: {}", bmson.info.title);
    println!("   Artist: {}", bmson.info.artist);
    println!();

    // Example 3: Missing required fields
    println!("3. Parsing with missing required fields:");
    let incomplete_json = r#"{
        "version": "1.0.0"
    }"#; // Missing info field

    let BmsonOutput { bmson, warnings } = parse_bmson(incomplete_json);

    println!("⚠️  Parsed with warnings:");
    for warning in &warnings {
        println!("   - {}", warning);
    }
    println!("   Using default values:");
    println!("   Title: {}", bmson.info.title);
    println!("   Artist: {}", bmson.info.artist);
    println!("   Level: {}", bmson.info.level);
    println!();

    // Example 4: Invalid field types
    println!("4. Parsing with invalid field types:");
    let invalid_types = r#"{
        "version": "1.0.0",
        "info": {
            "title": 123,
            "artist": "Valid Artist",
            "genre": "Valid Genre",
            "level": "invalid",
            "init_bpm": 120.0,
            "judge_rank": 100.0,
            "total": 100.0,
            "resolution": 240
        },
        "sound_channels": []
    }"#; // title is number, level is string

    let BmsonOutput { bmson, warnings } = parse_bmson(invalid_types);

    println!("⚠️  Parsed with warnings:");
    for warning in &warnings {
        println!("   - {}", warning);
    }
    println!("   Using default values for invalid fields:");
    println!("   Title: {}", bmson.info.title); // Should be "Unknown"
    println!("   Level: {}", bmson.info.level); // Should be 1
    println!();

    println!("Example completed successfully!");
}
