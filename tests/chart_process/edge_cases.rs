//! Edge case tests for `chart_process` module.
//!
//! Tests for boundary conditions and edge cases:
//! - Very long elapsed time (30 days)
//! - Zero-length sections
//! - Position zero edge cases

#![cfg(feature = "bmson")]

use std::time::Duration;

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive};

use bms_rs::bms::Decimal;
use bms_rs::bms::prelude::*;
use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::prelude::*;

// ============================================================================
// Helper Functions (reused from existing test files)
// ============================================================================

/// Setup a BMS processor for testing
fn setup_bms_processor_with_config<T, P, R, M>(
    source: &str,
    config: ParseConfig<T, P, R, M>,
    reaction_time: TimeSpan,
) -> ChartPlayer
where
    T: KeyLayoutMapper,
    P: Prompter,
    R: Rng,
    M: TokenModifier,
{
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms: bms_res,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, config);
    assert_eq!(parse_warnings, vec![]);
    let bms = match bms_res {
        Ok(bms) => bms,
        Err(err) => panic!("Failed to parse BMS in test setup: {err:?}"),
    };

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<T>(&bms);
    let start_time = TimeStamp::now();
    ChartPlayer::start(chart, visible_range_per_bpm, start_time)
}

/// Setup a BMS processor with `AlwaysWarnAndUseNewer` prompter
fn setup_bms_processor_with_newer_prompter(source: &str, reaction_time: TimeSpan) -> ChartPlayer {
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    setup_bms_processor_with_config(source, config, reaction_time)
}

/// Setup a BMSON processor for testing
fn setup_bmson_processor(json: &str, reaction_time: TimeSpan) -> ChartPlayer {
    let output = parse_bmson(json);
    let Some(bmson) = output.bmson else {
        panic!(
            "Failed to parse BMSON in test setup. Errors: {:?}",
            output.errors
        );
    };

    let Some(base_bpm) = StartBpmGenerator.generate(&bmson) else {
        panic!(
            "Failed to generate base BPM in test setup. Info: {:?}",
            bmson.info
        );
    };
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let start_time = TimeStamp::now();
    ChartPlayer::start(chart, visible_range_per_bpm, start_time)
}

// ============================================================================
// Test 1: Very Long Elapsed Time (30 days)
// ============================================================================

#[test]
fn test_very_long_elapsed_time_no_errors() {
    // Use simple BMSON for long-duration playback
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Long Time Test",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240
        },
        "sound_channels": [
            {
                "name": "note.wav",
                "notes": [
                    { "x": 1, "y": 0, "l": 0, "c": false },
                    { "x": 1, "y": 240, "l": 0, "c": false },
                    { "x": 1, "y": 480, "l": 0, "c": false }
                ]
            }
        ],
        "bpm_events": [
            { "y": 240, "bpm": 180.0 }
        ]
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::start();

    // Simulate 30 days of playback time
    let thirty_days = TimeSpan::from_duration(Duration::from_secs(2592000));
    let after_long_time = start_time + thirty_days;

    // Should not panic or return errors
    let _ = processor.update(after_long_time);

    // Verify playback state is still valid
    let state = processor.playback_state();
    // BPM should be 180 after the BPM change event at y=240
    let expected_bpm = Decimal::from(180);
    assert_eq!(
        *state.current_bpm(),
        expected_bpm,
        "BPM should be {} after 30 days, got {}",
        expected_bpm,
        state.current_bpm()
    );

    // Verify visible_events returns valid results
    let events = processor.visible_events();
    for (ev, ratio_range) in events {
        // activate_time should be finite
        let activate_secs = ev.activate_time().as_secs_f64();
        assert!(
            activate_secs.is_finite(),
            "activate_time should be finite after 30 days, got {}",
            activate_secs
        );

        // display_ratio should be finite
        let ratio_start = ratio_range.start().value().to_f64().unwrap_or(0.0);
        let ratio_end = ratio_range.end().value().to_f64().unwrap_or(0.0);
        assert!(
            ratio_start.is_finite(),
            "display_ratio start should be finite, got {}",
            ratio_start
        );
        assert!(
            ratio_end.is_finite(),
            "display_ratio end should be finite, got {}",
            ratio_end
        );
    }
}

// ============================================================================
// Test 2a: BMS Zero-Length Section (verify parser rejects zero-length sections)
// ============================================================================

#[test]
fn test_bms_zero_length_section_rejected_by_parser() {
    let bms_source = r#"
#TITLE Zero Length Section Test
#ARTIST Test
#BPM 120
#PLAYER 1
#WAV01 test.wav

// Set section 2 length to 0
#00202:0.000000

// Place note inside zero-length section
#00211:01

// Note in normal section for comparison
#00111:01
"#;

    // Verify parser rejects zero-length sections
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(bms_source);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms: _bms_res,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    // Should have syntax error because section length cannot be zero
    assert!(
        !parse_warnings.is_empty(),
        "Parser should reject zero-length sections with an error"
    );
    // Check warning string representation contains relevant information
    let warning_text = format!("{:?}", parse_warnings);
    assert!(
        warning_text.contains("section length must be greater than zero")
            || warning_text.contains("SyntaxError"),
        "Expected error about section length, got: {}",
        warning_text
    );
}

// ============================================================================
// Test 2a-alt: BMS Edge Cases (very small but non-zero section length)
// Test that ChartPlayer correctly handles events before, during, and after
// a very small section to simulate zero-length section edge cases
// ============================================================================

#[test]
fn test_bms_very_small_section_no_division_by_zero() {
    let bms_source = r#"
#TITLE Very Small Section Test
#ARTIST Test
#BPM 120
#PLAYER 1
#WAV01 test.wav

// Set section 2 length to very small value (but greater than zero)
// This simulates a near-zero-length section
#00202:0.000001

// Note BEFORE the very small section (in section 1)
#00111:01

// Note INSIDE the very small section (section 2)
#00211:01

// Note AFTER the very small section (in section 3)
#00311:01
"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let mut processor = setup_bms_processor_with_newer_prompter(bms_source, reaction_time);
    let start_time = TimeStamp::start();

    // Progress through multiple update calls to ensure we pass through all sections
    // First update: process initial section
    let _ = processor.update(start_time + TimeSpan::SECOND);
    let events1 = processor.visible_events();
    let count1 = events1.len();

    // Second update: move past the very small section
    let _ = processor.update(start_time + TimeSpan::SECOND * 5);
    let events2 = processor.visible_events();
    let count2 = events2.len();

    // Third update: ensure we've processed all sections
    let _ = processor.update(start_time + TimeSpan::SECOND * 10);
    let events3 = processor.visible_events();

    // Verify that we processed events without panicking
    // The exact count may vary, but we should have some events
    assert!(
        count1 + count2 + events3.len() > 0,
        "Should have processed some events across all sections"
    );

    // Verify all visible_events return valid results (no division by zero)
    for (_ev, ratio_range) in events1.iter().chain(events2.iter()).chain(events3.iter()) {
        // Ensure no NaN or infinite values caused by division by zero
        let ratio_start = ratio_range.start().value().to_f64().unwrap_or(0.0);
        let ratio_end = ratio_range.end().value().to_f64().unwrap_or(0.0);
        assert!(
            ratio_start.is_finite(),
            "display_ratio start should be finite with very small section length, got {}",
            ratio_start
        );
        assert!(
            ratio_end.is_finite(),
            "display_ratio end should be finite with very small section length, got {}",
            ratio_end
        );
    }

    // Additional verification: check that playback state remains consistent
    let state = processor.playback_state();
    // BPM should be 120 (the initial BPM in the BMS)
    let expected_bpm = Decimal::from(120);
    assert_eq!(
        *state.current_bpm(),
        expected_bpm,
        "BPM should be {} after processing very small section, got {}",
        expected_bpm,
        state.current_bpm()
    );
    // Speed should be 1.0 (initial speed)
    let expected_speed = Decimal::one();
    assert_eq!(
        *state.current_speed(),
        expected_speed,
        "Speed should be {} after processing very small section, got {}",
        expected_speed,
        state.current_speed()
    );
}

// ============================================================================
// Test 2b: BMSON Edge Cases
// ============================================================================

#[test]
fn test_bmson_edge_cases_no_division_by_zero() {
    // Test edge case with note at y=0
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Edge Cases Test",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240
        },
        "sound_channels": [
            {
                "name": "note.wav",
                "notes": [
                    { "x": 1, "y": 0, "l": 0, "c": false }
                ]
            }
        ]
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::start();

    // Start playback from y=0
    let _ = processor.update(start_time);

    // Verify compute_display_ratio doesn't cause errors when event_y == current_y
    let events = processor.visible_events();
    for (_ev, ratio_range) in events {
        let ratio_start = ratio_range.start().value().to_f64().unwrap_or(0.0);
        assert!(
            ratio_start.is_finite(),
            "display_ratio should be finite when event_y == current_y, got {}",
            ratio_start
        );
    }
}
