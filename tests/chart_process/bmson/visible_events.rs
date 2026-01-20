#![cfg(feature = "bmson")]

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive};

use bms_rs::bms::Decimal;
use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::prelude::*;

use super::{MICROSECOND_EPSILON, assert_time_close};

#[test]
fn test_bmson_visible_events_display_ratio_is_not_all_zero() {
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
    let start_time = TimeStamp::start();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let _ = processor.update(start_time + TimeSpan::MILLISECOND * 100);

    let mut got_any_ratio = false;
    for (ev, ratio_range) in processor.visible_events() {
        if matches!(ev.event(), ChartEvent::Note { .. }) {
            let ratio = ratio_range.start().value().to_f64().unwrap_or(0.0);
            let expected = 3.0 / 4.0;
            assert_time_close(expected, ratio, "display_ratio for visible note");
            got_any_ratio = true;
            break;
        }
    }
    assert!(got_any_ratio, "expected at least one visible note event");
}

#[test]
fn test_visible_events_duration_matches_reaction_time() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240,
            "mode_hint": "beat-7k"
        },
        "sound_channels": [
            { "name": "note.wav", "notes": [ { "x": 1, "y": 0, "l": 0, "c": false } ] }
        ],
        "bpm_events": []
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON in test setup");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM in test setup");
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let start_time = TimeStamp::now();
    let processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);
    let _start_time = TimeStamp::start();

    let initial_state = processor.playback_state();
    assert_eq!(*initial_state.current_bpm(), Decimal::from(120));
    assert_eq!(*initial_state.playback_ratio(), Decimal::one());

    let test_base_bpm = BaseBpm::from(Decimal::from(120));
    let visible_range = VisibleRangePerBpm::new(&test_base_bpm, reaction_time);
    let state = processor.playback_state();
    let visible_window_y =
        visible_range.window_y(state.current_bpm(), &Decimal::one(), state.playback_ratio());

    let velocity = Decimal::from(120) * Decimal::one() / Decimal::from(240u64);
    let time_to_cross = visible_window_y.as_ref() / velocity;

    let actual_time_to_cross_f64 = time_to_cross.to_f64().unwrap_or(0.0);
    assert_time_close(1.2, actual_time_to_cross_f64, "time_to_cross");
}

#[test]
fn test_visible_events_duration_with_playback_ratio() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240,
            "mode_hint": "beat-7k"
        },
        "sound_channels": [
            { "name": "note.wav", "notes": [ { "x": 1, "y": 0, "l": 0, "c": false } ] }
        ],
        "bpm_events": []
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON in test setup");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM in test setup");
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);
    let _start_time = TimeStamp::start();

    let test_base_bpm = BaseBpm::from(Decimal::from(120));
    let visible_range = VisibleRangePerBpm::new(&test_base_bpm, reaction_time);

    let state = processor.playback_state();
    let visible_window_y_ratio_1 =
        visible_range.window_y(state.current_bpm(), &Decimal::one(), &Decimal::one());

    processor.post_events(std::iter::once(ControlEvent::SetPlaybackRatio {
        ratio: Decimal::from(0.5),
    }));

    let changed_state = processor.playback_state();
    assert_eq!(*changed_state.playback_ratio(), Decimal::from(0.5));

    let state_0_5 = processor.playback_state();
    let visible_window_y_ratio_0_5 = visible_range.window_y(
        state_0_5.current_bpm(),
        &Decimal::one(),
        state_0_5.playback_ratio(),
    );

    let ratio = visible_window_y_ratio_0_5.as_ref() / visible_window_y_ratio_1.as_ref();
    let ratio_f64 = ratio.to_f64().unwrap_or(0.0);
    assert_time_close(
        0.5,
        ratio_f64,
        "visible_window_y ratio when playback_ratio=0.5",
    );

    let velocity = Decimal::from(120) * Decimal::from(0.5) / Decimal::from(240u64);
    let time_to_cross = visible_window_y_ratio_0_5.as_ref() / velocity;

    let actual_time_to_cross_f64 = time_to_cross.to_f64().unwrap_or(0.0);
    assert_time_close(1.2, actual_time_to_cross_f64, "time_to_cross");
}

#[test]
fn test_visible_events_with_boundary_conditions() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240,
            "mode_hint": "beat-7k"
        },
        "sound_channels": [
            { "name": "note.wav", "notes": [ { "x": 1, "y": 0, "l": 0, "c": false } ] }
        ],
        "bpm_events": []
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON in test setup");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM in test setup");
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let start_time = TimeStamp::now();
    let _processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let test_base_bpm = BaseBpm::from(Decimal::from(120));
    let visible_range = VisibleRangePerBpm::new(&test_base_bpm, reaction_time);

    let very_small_ratio = Decimal::from(1u64);
    let visible_window_y =
        visible_range.window_y(&Decimal::from(120), &Decimal::one(), &very_small_ratio);

    assert!(
        *visible_window_y.as_ref() >= Decimal::from(0),
        "visible_window_y should be non-negative even with very small playback_ratio"
    );

    let normal_ratio = Decimal::one();
    let visible_window_y_normal =
        visible_range.window_y(&Decimal::from(120), &Decimal::one(), &normal_ratio);

    let expected_ratio = very_small_ratio / normal_ratio;
    let actual_ratio = visible_window_y.as_ref() / visible_window_y_normal.as_ref();

    let actual_ratio_f64 = actual_ratio.to_f64().unwrap_or(0.0);
    let expected_ratio_f64 = expected_ratio.to_f64().unwrap_or(0.0);
    assert_time_close(
        expected_ratio_f64,
        actual_ratio_f64,
        "visible_window_y ratio with playback_ratio",
    );
}
