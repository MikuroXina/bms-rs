#![cfg(feature = "bmson")]

use gametime::{TimeSpan, TimeStamp};

use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::prelude::*;

use super::{MICROSECOND_EPSILON, assert_time_close};

#[test]
fn test_bmson_continue_duration_references_bpm_and_stop() {
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
                    { "x": 1, "y": 480, "l": 0, "c": true }
                ]
            }
        ],
        "stop_events": [
            { "y": 960, "duration": 240 }
        ]
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 5000;
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON in test setup");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM in test setup");
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let t = start_time + TimeSpan::MILLISECOND * 100;
    let _ = processor.update(t);

    let mut found = false;
    for (ev, _) in processor.visible_events() {
        if let ChartEvent::Note {
            continue_play: Some(dur),
            ..
        } = ev.event()
        {
            let secs = dur.as_secs_f64();
            assert_time_close(1.0, secs, "continue timepoint");
            found = true;
            break;
        }
    }
    assert!(found, "Expected to find a note with continue duration");
}

#[test]
fn test_bmson_continue_duration_with_bpm_scroll_and_stop() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Test2",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240
        },
        "sound_channels": [
            {
                "name": "test2.wav",
                "notes": [
                    { "x": 1, "y": 240, "l": 0, "c": true }
                ]
            }
        ],
        "bpm_events": [
            { "y": 720, "bpm": 180.0 }
        ],
        "scroll_events": [
            { "y": 960, "rate": 2.0 }
        ],
        "stop_events": [
            { "y": 1200, "duration": 240 }
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

    let t = start_time + TimeSpan::MILLISECOND * 100;
    let _ = processor.update(t);

    let mut found = false;
    for (ev, _) in processor.visible_events() {
        if let ChartEvent::Note {
            continue_play: Some(dur),
            ..
        } = ev.event()
        {
            let secs = dur.as_secs_f64();
            assert_time_close(0.5, secs, "continue timepoint");
            found = true;
            break;
        }
    }
    assert!(found, "Expected to find a note with continue duration");
}

#[test]
fn test_bmson_multiple_continue_and_noncontinue_in_same_channel() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "MixedContinue",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240
        },
        "sound_channels": [
            {
                "name": "mix.wav",
                "notes": [
                    { "x": 1, "y": 240, "l": 0, "c": false },
                    { "x": 1, "y": 360, "l": 0, "c": true },
                    { "x": 1, "y": 480, "l": 0, "c": true },
                    { "x": 1, "y": 600, "l": 0, "c": false }
                ]
            }
        ]
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 5000;
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON in test setup");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM in test setup");
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let t = start_time + TimeSpan::MILLISECOND * 100;
    let _ = processor.update(t);

    let mut some_count = 0;
    let mut none_count = 0;
    let mut durations = Vec::new();
    for (ev, _) in processor.visible_events() {
        if let ChartEvent::Note { continue_play, .. } = ev.event() {
            match continue_play {
                Some(d) => {
                    some_count += 1;
                    durations.push(d.as_secs_f64());
                }
                None => none_count += 1,
            }
        }
    }

    assert_eq!(none_count, 2, "Expected two non-continue notes with None");
    assert_eq!(some_count, 2, "Expected two continue notes with Some");
    durations.sort_by(f64::total_cmp);
    let [a, b] = durations.as_slice() else {
        panic!(
            "Expected two continue durations, got {}: {:?}",
            durations.len(),
            durations
        );
    };
    let a = *a;
    let b = *b;
    assert_time_close(0.25, a, "continue timepoint (first note)");
    assert_time_close(0.50, b, "continue timepoint (second note)");
}

#[test]
fn test_bmson_continue_accumulates_multiple_stops_between_notes() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "StopsAccumulation",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240
        },
        "sound_channels": [
            {
                "name": "stops.wav",
                "notes": [
                    { "x": 1, "y": 240, "l": 0, "c": true },
                    { "x": 1, "y": 1200, "l": 0, "c": true }
                ]
            }
        ],
        "stop_events": [
            { "y": 480, "duration": 240 },
            { "y": 960, "duration": 240 }
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

    let t = start_time + TimeSpan::MILLISECOND * 2400;
    let _ = processor.update(t);

    let mut found = false;
    for (ev, _) in processor.visible_events() {
        if let ChartEvent::Note {
            continue_play: Some(dur),
            ..
        } = ev.event()
        {
            let secs = dur.as_secs_f64();
            if (secs - 3.5).abs() < MICROSECOND_EPSILON {
                found = true;
                break;
            }
        }
    }
    assert!(
        found,
        "Expected to find the note at y=1.25 with continue timepoint"
    );
}

#[test]
fn test_bmson_continue_independent_across_sound_channels() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "ChannelsIndependent",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240
        },
        "sound_channels": [
            {
                "name": "A.wav",
                "notes": [
                    { "x": 1, "y": 240, "l": 0, "c": true },
                    { "x": 1, "y": 480, "l": 0, "c": false }
                ]
            },
            {
                "name": "B.wav",
                "notes": [
                    { "x": 2, "y": 360, "l": 0, "c": true },
                    { "x": 2, "y": 960, "l": 0, "c": false }
                ]
            }
        ]
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 5000;
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON in test setup");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM in test setup");
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);
    let t = start_time + TimeSpan::MILLISECOND * 100;
    let _ = processor.update(t);

    let mut durations = Vec::new();
    let mut none_count = 0;
    for (ev, _) in processor.visible_events() {
        if let ChartEvent::Note { continue_play, .. } = ev.event() {
            match continue_play {
                Some(d) => durations.push(d.as_secs_f64()),
                None => none_count += 1,
            }
        }
    }

    assert_eq!(none_count, 2, "Expected two non-continue notes with None");
    assert_eq!(durations.len(), 2, "Expected two continue durations");
    durations.sort_by(f64::total_cmp);
    let [a, b] = durations.as_slice() else {
        panic!(
            "Expected two continue durations, got {}: {:?}",
            durations.len(),
            durations
        );
    };
    let a = *a;
    let b = *b;
    assert_time_close(0.5, a, "continue timepoint (channel A)");
    assert_time_close(0.75, b, "continue timepoint (channel B)");
}
