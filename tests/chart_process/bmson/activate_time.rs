#![cfg(feature = "bmson")]

use gametime::{TimeSpan, TimeStamp};

use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::prelude::*;

use super::assert_time_close;

#[test]
fn test_bmson_visible_event_activate_time_prediction() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "AT",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240
        },
        "sound_channels": [
            { "name": "a.wav", "notes": [ { "x": 1, "y": 240, "l": 0, "c": false } ] }
        ]
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON in test setup");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM in test setup");
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let _ = processor.update(start_time);
    let events = processor.visible_events();
    assert!(!events.is_empty(), "Should have visible events at start");

    let mut checked = false;
    for (ev, _) in events {
        if let ChartEvent::Note { .. } = ev.event() {
            let secs = ev.activate_time().as_secs_f64();
            assert_time_close(0.5, secs, "activate_time");
            checked = true;
            break;
        }
    }
    assert!(checked, "Expected to find a note visible event");
}

#[test]
fn test_bmson_visible_event_activate_time_with_bpm_change() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "AT-BPM",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240
        },
        "sound_channels": [
            { "name": "a.wav", "notes": [ { "x": 1, "y": 960, "l": 0, "c": false } ] }
        ],
        "bpm_events": [ { "y": 480, "bpm": 240.0 } ]
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 2000;
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON in test setup");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM in test setup");
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);
    let _ = processor.update(start_time);

    let events = processor.visible_events();
    assert!(!events.is_empty(), "Should have visible events at start");

    let mut checked = false;
    for (ev, _) in events {
        if let ChartEvent::Note { .. } = ev.event() {
            let secs = ev.activate_time().as_secs_f64();
            assert_time_close(1.5, secs, "activate_time with BPM change");
            checked = true;
            break;
        }
    }
    assert!(checked, "Expected to find a note visible event");
}

#[test]
fn test_bmson_visible_event_activate_time_with_stop_inside_interval() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "AT-STOP",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240
        },
        "sound_channels": [
            { "name": "a.wav", "notes": [ { "x": 1, "y": 960, "l": 0, "c": false } ] }
        ],
        "stop_events": [ { "y": 480, "duration": 240 } ]
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 3000;
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON in test setup");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM in test setup");
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);
    let _ = processor.update(start_time);

    let events = processor.visible_events();
    assert!(!events.is_empty(), "Should have visible events at start");

    let mut checked = false;
    for (ev, _) in events {
        if let ChartEvent::Note { .. } = ev.event() {
            let secs = ev.activate_time().as_secs_f64();
            assert_time_close(2.5, secs, "activate_time with stop");
            checked = true;
            break;
        }
    }
    assert!(checked, "Expected to find a note visible event");
}
