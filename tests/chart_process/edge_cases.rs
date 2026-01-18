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

use bms_rs::bms::command::channel::Key;
use bms_rs::bms::prelude::*;
use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::prelude::*;

use super::assert_time_close;

/// Parse BMS source and return the BMS struct, asserting no warnings.
fn parse_bms_no_warnings<T, P, R, M>(source: &str, config: ParseConfig<T, P, R, M>) -> Bms
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
    bms_res.expect("Failed to parse BMS in test setup")
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
    let start_time = TimeStamp::start();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

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
// Test 2a: BMS Zero-Length Section Handling
// Comprehensive test for zero-length sections, covering parser, BmsProcessor,
// and ChartPlayer handling
// ============================================================================

#[test]
fn test_bms_zero_length_section_comprehensive() {
    let bms_source = r#"
#TITLE Zero Length Section Test
#ARTIST Test
#BPM 120
#PLAYER 1
#WAV01 test.wav

// Section 2 is zero-length
#00202:0

// Multiple events in zero-length section (all at different fractional positions)
#00211:01
#00212:02
#00213:03

// Events in normal sections for comparison
#00111:01
#00311:01
"#;

    // Part 1: Verify parser allows zero-length sections
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(bms_source);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms: bms_res,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert!(
        parse_warnings.is_empty(),
        "Parser should allow zero-length sections without warnings"
    );
    let bms = bms_res.expect("Failed to parse BMS with zero-length section");

    // Part 2: Verify BmsProcessor successfully parsed zero-length section
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let _config = default_config().prompter(AlwaysWarnAndUseNewer);
    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);

    assert!(
        !chart.events().as_events().is_empty(),
        "Should have parsed some events"
    );

    // Part 3: Verify ChartPlayer handles zero-length sections without errors
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);
    let _ = processor.update(start_time + TimeSpan::SECOND * 3);

    // Verify no panics or errors occurred
    let state = processor.playback_state();
    assert!(
        state.current_bpm().to_f64().is_some_and(f64::is_finite),
        "BPM should be finite"
    );

    // Verify visible_events returns valid results
    let events = processor.visible_events();
    for (_ev, ratio_range) in events {
        let ratio_start = ratio_range.start().value().to_f64().unwrap_or(0.0);
        assert!(
            ratio_start.is_finite(),
            "display_ratio should be finite with zero-length section"
        );
    }
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
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

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

        // display_ratio should be a valid number in [0.0, 1.0] range
        assert!(
            (0.0..=1.0).contains(&ratio_start),
            "display_ratio start should be in [0.0, 1.0] range with very small section length, got {}",
            ratio_start
        );
        assert!(
            (0.0..=1.0).contains(&ratio_end),
            "display_ratio end should be in [0.0, 1.0] range with very small section length, got {}",
            ratio_end
        );

        // ratio_start and ratio_end should be very close (for short notes)
        // The difference should be negligible for single notes without long keysounds
        let ratio_diff = (ratio_start - ratio_end).abs();
        assert!(
            ratio_diff < 1e-6,
            "display_ratio range should be very small for short notes, got start={}, end={}, diff={}",
            ratio_start,
            ratio_end,
            ratio_diff
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
// Test 2a-3: BMS Consecutive Zero-Length Sections
// Test that ChartPlayer correctly handles multiple consecutive zero-length sections
// ============================================================================

#[test]
fn test_bms_consecutive_zero_length_sections() {
    let bms_source = r#"
#TITLE Consecutive Zero Length Sections
#ARTIST Test
#BPM 120
#PLAYER 1
#WAV01 test.wav

// Multiple consecutive zero-length sections
#00202:0
#00302:0
#00402:0

// Notes in zero-length sections
#00211:01
#00311:01
#00411:01

// Note in normal section after
#00511:01
"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    // Process all sections
    let _ = processor.update(start_time + TimeSpan::SECOND * 5);

    // Verify no errors
    let events = processor.visible_events();
    for (_ev, ratio_range) in events {
        let ratio_start = ratio_range.start().value().to_f64().unwrap_or(0.0);
        assert!(
            ratio_start.is_finite(),
            "Should handle consecutive zero-length sections without errors"
        );
    }
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
    let start_time = TimeStamp::start();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    // Start playback from y=0
    let _ = processor.update(start_time);

    // Verify compute_display_ratio doesn't cause errors when event_y == current_y
    let events = processor.visible_events();
    for (_ev, ratio_range) in events {
        let ratio_start = ratio_range.start().value().to_f64().unwrap_or(0.0);
        let ratio_end = ratio_range.end().value().to_f64().unwrap_or(0.0);
        // When event_y == current_y (note at y=0, current_y=0), display_ratio should be 1.0
        assert_time_close(
            1.0,
            ratio_start,
            "display_ratio start when event_y == current_y",
        );
        assert_time_close(
            1.0,
            ratio_end,
            "display_ratio end when event_y == current_y",
        );
    }
}

#[test]
fn test_parsed_chart_tracks_have_correct_y_coordinates_and_wav_ids() {
    let bms_source = r#"
#WAV01 test1.wav
#WAV02 test2.wav
#WAV03 test3.wav
#WAV04 test4.wav
#00202:0.0
#00211:01
#00212:02
#00213:0103
#00314:04
"#;

    let config = default_config().prompter(AlwaysUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);

    let note_events: Vec<_> = chart
        .events()
        .as_events()
        .iter()
        .filter_map(|ev| {
            if let ChartEvent::Note { key, wav_id, .. } = ev.event() {
                Some((ev.position().clone(), *key, *wav_id))
            } else {
                None
            }
        })
        .collect();

    let expected_events = vec![
        (YCoordinate::from(1.0), Key::Key(1), Some(WavId::new(1))),
        (YCoordinate::from(1.0), Key::Key(3), Some(WavId::new(1))),
        (YCoordinate::from(1.0), Key::Key(2), Some(WavId::new(2))),
        (YCoordinate::from(1.5), Key::Key(3), Some(WavId::new(3))),
        (YCoordinate::from(2.0), Key::Key(4), Some(WavId::new(4))),

        // FIXME:
        // https://github.com/MikuroXina/bms-rs/pull/311
        // I want to select the 4th: Treat `#xxx02:0.0` as zero length section, preserving objects in the whole section, and treat them as in the same time position.
        // WAV11 -> WavId::new(1), WAV12 -> WavId::new(2), WAV13 -> WavId::new(3), WAV14 -> WavId::new(4)
        // The result should be
        // (YCoordinate::from(1.0), Key::Key(1), Some(WavId::new(1))),
        // (YCoordinate::from(1.0), Key::Key(2), Some(WavId::new(2))),
        // (YCoordinate::from(1.0), Key::Key(3), Some(WavId::new(3))),
        // (YCoordinate::from(1.0), Key::Key(4), Some(WavId::new(4))),
    ];

    assert_eq!(note_events, expected_events);
}
