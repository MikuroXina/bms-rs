//! Test diagnostics module functionality

#![cfg(feature = "diagnostics")]

use bms_rs::{
    bms::{BmsWarning, default_config, parse_bms},
    diagnostics::{SimpleSource, collect_bms_reports},
};

#[test]
fn test_simple_source_creation() {
    let source_text = "#TITLE Test Song\n#ARTIST Test Composer\n#BPM 120\n";
    let source = SimpleSource::new("test.bms", source_text);

    assert_eq!(source.text(), source_text);
}

#[test]
fn test_simple_source_basic_functionality() {
    let source_text = "#TITLE Test\n#ARTIST Composer\n#BPM 120\n";
    let source = SimpleSource::new("test.bms", source_text);

    // Test that we can create a SimpleSource and access its text
    assert_eq!(source.text(), source_text);

    // Test that the source contains the expected content
    assert!(source.text().contains("#TITLE"));
    assert!(source.text().contains("#ARTIST"));
    assert!(source.text().contains("#BPM"));
}

#[test]
fn test_emit_warnings_with_real_bms() {
    let bms_source = "#TITLE Test Song\n#ARTIST Composer\n#INVALID_COMMAND test\n";

    // Parse BMS file, should produce warnings
    let output = parse_bms(bms_source, default_config());

    if !output.warnings.is_empty() {
        // Verify diagnostics can be generated without printing to terminal
        let reports = collect_bms_reports("test.bms", bms_source, &output.warnings);
        assert_eq!(reports.len(), output.warnings.len());
    } else {
        // If no warnings, also test empty warnings case
        let empty_warnings: Vec<BmsWarning> = vec![];
        let reports = collect_bms_reports("test.bms", bms_source, &empty_warnings);
        assert_eq!(reports.len(), 0);
    }
}

#[test]
fn test_empty_warnings() {
    let bms_source = "#TITLE test\n#ARTIST composer\n";
    let empty_warnings: Vec<BmsWarning> = vec![];

    // Test empty warnings list case without printing
    let reports = collect_bms_reports("test.bms", bms_source, &empty_warnings);
    assert!(reports.is_empty());
}

#[test]
fn test_unknown_command_warning() {
    use bms_rs::bms::prelude::*;

    // Test BMS with unknown command
    let bms_source = "#TITLE Test\n#UNKNOWN_COMMAND value\n#ARTIST Composer\n";

    let output = TokenStream::parse_lex(bms_source);

    // Should have tokens including UnknownCommand
    assert!(output.tokens.iter().next().is_some());

    // Should not have warnings
    assert!(output.lex_warnings.is_empty());

    // Check if there's an UnknownCommand token
    let has_unknown_command_token = output.tokens.iter().any(|t| {
        matches!(
            t.content(),
            Token::Header {
                name,
                ..
            } if name == "UNKNOWN_COMMAND"
        )
    });
    assert!(
        has_unknown_command_token,
        "Should have UnknownCommand token"
    );
}
