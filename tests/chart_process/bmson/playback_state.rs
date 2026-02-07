#![cfg(feature = "bmson")]

use gametime::{TimeSpan, TimeStamp};

use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::prelude::*;
use strict_num_extended::FinF64;

use super::assert_time_close;

#[test]
fn test_bmson_restart_resets_scroll_to_one() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240
        },
        "sound_channels": [
            {
                "name": "test.wav",
                "notes": [
                    { "x": 1, "y": 480, "l": 0, "c": false }
                ]
            }
        ],
        "scroll_events": [
            { "y": 240, "rate": 0.5 }
        ]
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON in test setup");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM in test setup");
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let processor_start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, processor_start_time);
    let start_time = processor.started_at();

    let after_scroll = start_time + TimeSpan::MILLISECOND * 600;
    let _ = processor.update(after_scroll);
    let state = processor.playback_state();
    assert_ne!(*state.current_scroll(), FinF64::try_from(1.0).unwrap());

    let output2 = parse_bmson(json);
    let bmson2 = output2.bmson.expect("Failed to parse BMSON in test setup");

    let base_bpm2 = StartBpmGenerator
        .generate(&bmson2)
        .expect("Failed to generate base BPM in test setup");
    let visible_range_per_bpm2 = VisibleRangePerBpm::new(&base_bpm2, reaction_time);
    let chart2 = BmsonProcessor::parse(&bmson2);
    let start_time2 = TimeStamp::now();
    let restarted_processor = ChartPlayer::start(chart2, visible_range_per_bpm2, start_time2);
    let reset_state = restarted_processor.playback_state();
    assert_eq!(
        *reset_state.current_scroll(),
        FinF64::try_from(1.0).unwrap()
    );
}

#[test]
fn test_bmson_edge_cases_no_division_by_zero() {
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

    let _ = processor.update(start_time);

    let events = processor.visible_events();
    for (_ev, ratio_range) in events {
        let ratio_start = ratio_range.start().value().as_f64();
        let ratio_end = ratio_range.end().value().as_f64();
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
fn test_very_long_elapsed_time_no_errors() {
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

    use std::time::Duration;

    let thirty_days = TimeSpan::from_duration(Duration::from_secs(2592000));
    let after_long_time = start_time + thirty_days;

    let _ = processor.update(after_long_time);

    let state = processor.playback_state();
    let expected_bpm = FinF64::try_from(180.0).unwrap();
    assert_eq!(
        *state.current_bpm(),
        expected_bpm,
        "BPM should be {} after 30 days",
        expected_bpm,
    );

    let events = processor.visible_events();
    for (ev, ratio_range) in events {
        let activate_secs = ev.activate_time().as_secs_f64();
        assert!(
            activate_secs.is_finite(),
            "activate_time should be finite after 30 days"
        );

        let ratio_start = ratio_range.start().value().as_f64();
        let ratio_end = ratio_range.end().value().as_f64();
        assert!(
            ratio_start.is_finite(),
            "display_ratio start should be finite"
        );
        assert!(ratio_end.is_finite(), "display_ratio end should be finite");
    }
}
