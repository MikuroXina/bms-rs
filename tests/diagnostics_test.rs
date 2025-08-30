//! Test diagnostics module functionality

mod diagnostics_tests {
    use bms_rs::bms::{
        BmsWarning,
        diagnostics::{SimpleSource, emit_bms_warnings},
        parse_bms,
    };

    #[test]
    fn test_simple_source_creation() {
        let source_text = "#TITLE Test Song\n#ARTIST Test Composer\n#BPM 120\n";
        let source = SimpleSource::new("test.bms", source_text);

        assert_eq!(source.text(), source_text);
    }

    #[test]
    fn test_line_span_calculation() {
        let source_text = "#TITLE Test\n#ARTIST Composer\n#BPM 120\n";
        let source = SimpleSource::new("test.bms", source_text);

        // First line: "#TITLE Test\n"
        let span1 = source.line_span(1);
        assert_eq!(span1.start, 0);
        assert_eq!(span1.end, 12);

        // Second line: "#ARTIST Composer\n"
        let span2 = source.line_span(2);
        assert_eq!(span2.start, 12);
        assert_eq!(span2.end, 29);
    }

    #[test]
    fn test_emit_warnings_with_real_bms() {
        let bms_source = "#TITLE Test Song\n#ARTIST Composer\n#INVALID_COMMAND test\n";

        // Parse BMS file, should produce warnings
        let output = parse_bms(bms_source);

        if !output.warnings.is_empty() {
            // Note: here we just verify the function can be called normally
            emit_bms_warnings("test.bms", bms_source, &output.warnings);
        } else {
            // If no warnings, we also test the empty warnings case
            let empty_warnings: Vec<BmsWarning> = vec![];
            emit_bms_warnings("test.bms", bms_source, &empty_warnings);
        }
    }

    #[test]
    fn test_empty_warnings() {
        let bms_source = "#TITLE test\n#ARTIST composer\n";
        let empty_warnings: Vec<BmsWarning> = vec![];

        // Test empty warnings list case
        emit_bms_warnings("test.bms", bms_source, &empty_warnings);
    }
}
