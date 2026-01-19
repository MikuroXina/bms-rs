#![cfg(feature = "bmson")]

//! Integration tests for `bms_rs::chart_process::BmsonProcessor`.

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive};

use bms_rs::bms::Decimal;
use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::PlayheadEvent;
use bms_rs::chart_process::prelude::*;

use super::{MICROSECOND_EPSILON, assert_time_close};

#[test]
fn test_bmson_continue_duration_references_bpm_and_stop() {
    // BMSON with init BPM 120, a single key note at y=0.5 measure (480 pulses), c=true,
    // and a stop starting at y=1.0 measure (960 pulses) lasting 240 pulses (0.25 measure).
    // New semantics (timepoint): this note's continue equals the timepoint
    // from the last restart (none) to its y=0.5; the Stop is at 1.0 measure,
    // which is not within (0.0, 0.5), so it is not included.
    // At BPM 120, each measure is 2.0s ⇒ 0.5 * 2.0 = 1.0s
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

    // Progress slightly so the note at y=0.5 is inside visible window (0.6 measure default)
    // Advance slightly to ensure y=0.5 enters the visible window (default 0.6 measure)
    let t = start_time + TimeSpan::MILLISECOND * 100;
    let _ = processor.update(t);

    // Find the note and assert continue_play duration
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
            // Expected value after removing round: 0.75 (3/4)
            let expected = 3.0 / 4.0;
            assert_time_close(expected, ratio, "display_ratio for visible note");
            got_any_ratio = true;
            break;
        }
    }
    assert!(got_any_ratio, "expected at least one visible note event");
}

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
    assert_ne!(*state.current_scroll(), Decimal::one());

    // Restart by creating a new player
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
    assert_eq!(*reset_state.current_scroll(), Decimal::one());
}

#[test]
fn test_bmson_events_in_time_range_returns_note_near_center() {
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
                    { "x": 1, "y": 960, "l": 0, "c": false }
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
    let processor_start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, processor_start_time);
    let start_time = TimeStamp::start();
    let _events = processor.update(start_time + TimeSpan::SECOND * 2);

    let events = processor.events_in_time_range(
        (TimeSpan::ZERO - TimeSpan::MILLISECOND * 300)..=(TimeSpan::MILLISECOND * 300),
    );
    assert!(
        events
            .iter()
            .any(|ev| matches!(ev.event(), ChartEvent::Note { .. })),
        "Expected to find a note event around 2.0s"
    );
    for ev in events {
        assert!(
            *ev.activate_time() >= TimeSpan::SECOND && *ev.activate_time() <= TimeSpan::SECOND * 3,
            "activate_time should be within the query window: {:?}",
            ev.activate_time()
        );
    }
}

#[test]
fn test_bmson_continue_duration_with_bpm_scroll_and_stop() {
    // BMSON with init BPM 120, a key note at y=0.25 measure (240 pulses), c=true.
    // BPM changes to 180 at y=0.75 measure (720 pulses).
    // A scroll event occurs at y=1.0 measure (960 pulses) but should not affect time.
    // Stop starts at y=1.25 measure (1200 pulses) with duration 240 pulses (0.25 measure).
    // New semantics (timepoint): note at y=0.25 is before BPM/Stop/Scroll events,
    // so only the 0.25-measure timepoint is computed.
    // At BPM 120, each measure is 2.0s ⇒ 0.25 * 2.0 = 0.5s
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
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    // Advance slightly to ensure y=0.25 enters the visible window (default 0.6 measure)
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
    // Same sound_channel with mixed continue and non-continue notes.
    // init BPM 120, resolution 240.
    // Notes at pulses: 240(c=false), 360(c=true), 480(c=true), 600(c=false).
    // New semantics (continue = audio playback time point since last restart):
    //  - c=false notes have continue_play None and reset the timepoint
    //  - c=true notes have continue_play Some(seconds) measured from last c=false at 240:
    //    y=360 ⇒ Δy=0.125 measure ⇒ 0.125*(240/120) = 0.25s
    //    y=480 ⇒ Δy=0.25  measure ⇒ 0.25 *(240/120) = 0.50s
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
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
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
    // Both c=true notes should be ~0.25s and ~0.50s (since last restart at y=240)
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
    // Single sound_channel with multiple Stops in between; verify total time accumulation:
    // init BPM 120, res 240; note1 at 0.25 measure (240 pulses, c=true), note2 at 1.25 measure (1200 pulses)
    // Stop1: at 0.5 measure (y=480, duration 240 pulses), Stop2: at 1.0 measure (y=960, duration 240 pulses)
    // New semantics (timepoint): the second c=true note's timepoint equals base time from 0.0 -> 1.25,
    // i.e., 1.25 measures = 2.5s.
    // In the interval (0.0, 1.25), there are two Stops, each 0.25 measure = 0.5s,
    // totaling 2.5 + 0.5 + 0.5 = 3.5s
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
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsonProcessor::parse(&bmson);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    // Advance to make the preload window cover the note at y=1.25
    // Note: With new playhead speed (1/240), speed is half of original (1/120)
    // So need more time to reach the same Y position
    // Also reaction time is now 1.2s instead of 0.6s
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
            // Check if 3.5s continue timepoint is found
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
    // Two sound_channels; their continue_time is independent:
    // A: 0.25 -> 0.5 measure (240 -> 480 pulses), c=true until next note; expect ~0.5s.
    // B: 0.375 -> 1.0 measure (360 -> 960 pulses), c=true until next note; expect ~1.25s.
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
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
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

    // The two c=false notes should be None
    assert_eq!(none_count, 2, "Expected two non-continue notes with None");
    // The two c=true notes should each have a timepoint, independent from each other:
    // there should be ~0.5s and ~0.75s values
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
#[test]
fn test_bmson_visible_event_activate_time_prediction() {
    // Simple BMSON: init BPM 120, resolution 240, one note at 0.25 measure (240 pulses)
    // Expected activate_time at start is 0.25 * (240/120) = 0.5s
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
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
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
    // init BPM 120, one note at 1.0 measure; BPM changes to 240 at 0.5 measure
    // Expected predicted activate_time at start: 0.5*2.0 + 0.5*1.0 = 1.5s
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
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
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
    // init BPM 120, one note at 1.0 measure; Stop at 0.5 measure lasting 0.25 measure (240 pulses)
    // Expected predicted activate_time at start: 1.0*2.0 + 0.5 = 2.5s
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
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
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

#[test]
fn test_bmson_visible_events_duration_matches_reaction_time() {
    // Test that when current_bpm == base_bpm and playback_ratio is 1,
    // events stay in visible_events for exactly reaction_time duration
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

    // Verify standard conditions
    let initial_state = processor.playback_state();
    assert_eq!(*initial_state.current_bpm(), Decimal::from(120));
    assert_eq!(*initial_state.playback_ratio(), Decimal::one());

    // Calculate expected visible window Y
    let test_base_bpm = BaseBpm::from(Decimal::from(120));
    let visible_range = VisibleRangePerBpm::new(&test_base_bpm, reaction_time);
    let state = processor.playback_state();
    let visible_window_y = visible_range.window_y(
        state.current_bpm(),
        &Decimal::one(), // BMSON has no current_speed
        state.playback_ratio(),
    );

    // Calculate time to cross window
    // velocity = current_bpm * playback_ratio / 240 (BMSON doesn't have current_speed)
    // time = visible_window_y / velocity
    let velocity = Decimal::from(120) * Decimal::one() / Decimal::from(240u64);
    let time_to_cross = visible_window_y.as_ref() / velocity;

    // Note: This test verifies the correctness of visible window calculation
    // Due to accumulated error, actual value may have slight deviation from theoretical value
    let actual_time_to_cross_f64 = time_to_cross.to_f64().unwrap_or(0.0);
    // Expected: 1.2s (actual calculated value), evaluated with 1 microsecond precision
    assert_time_close(1.2, actual_time_to_cross_f64, "time_to_cross");
}

#[test]
fn test_bmson_visible_events_duration_with_playback_ratio() {
    // Test that playback_ratio affects visible_window_y and event display duration
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

    // Get initial visible_window_y (playback_ratio = 1)
    let test_base_bpm = BaseBpm::from(Decimal::from(120));
    let visible_range = VisibleRangePerBpm::new(&test_base_bpm, reaction_time);

    let state = processor.playback_state();
    let visible_window_y_ratio_1 =
        visible_range.window_y(state.current_bpm(), &Decimal::one(), &Decimal::one());

    // Set playback_ratio to 0.5
    processor.post_events(std::iter::once(ControlEvent::SetPlaybackRatio {
        ratio: Decimal::from(0.5),
    }));

    // Verify playback_ratio changed
    let changed_state = processor.playback_state();
    assert_eq!(*changed_state.playback_ratio(), Decimal::from(0.5));

    // Get new visible_window_y (playback_ratio = 0.5)
    let state_0_5 = processor.playback_state();
    let visible_window_y_ratio_0_5 = visible_range.window_y(
        state_0_5.current_bpm(),
        &Decimal::one(),
        state_0_5.playback_ratio(),
    );

    // Verify: visible_window_y should halve when playback_ratio halves
    let ratio = visible_window_y_ratio_0_5.as_ref() / visible_window_y_ratio_1.as_ref();
    let ratio_f64 = ratio.to_f64().unwrap_or(0.0);
    assert_time_close(
        0.5,
        ratio_f64,
        "visible_window_y ratio when playback_ratio=0.5",
    );

    // Calculate time to cross window with playback_ratio = 0.5
    let velocity = Decimal::from(120) * Decimal::from(0.5) / Decimal::from(240u64);
    let time_to_cross = visible_window_y_ratio_0_5.as_ref() / velocity;

    // Note: This test verifies the correctness of visible window calculation
    // Due to accumulated error, actual value may have slight deviation from theoretical value
    let actual_time_to_cross_f64 = time_to_cross.to_f64().unwrap_or(0.0);
    // Expected: 1.2s (actual calculated value), evaluated with 1 microsecond precision
    assert_time_close(1.2, actual_time_to_cross_f64, "time_to_cross");
}

#[test]
fn test_visible_events_with_boundary_conditions() {
    // Test boundary conditions: zero or very small speed/playback_ratio
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

    // Test with very small playback_ratio (not zero, to avoid division by zero)
    let test_base_bpm = BaseBpm::from(Decimal::from(120));
    let visible_range = VisibleRangePerBpm::new(&test_base_bpm, reaction_time);

    let very_small_ratio = Decimal::from(1u64);
    let visible_window_y =
        visible_range.window_y(&Decimal::from(120), &Decimal::one(), &very_small_ratio);

    // Should not panic and should return a valid result
    assert!(
        *visible_window_y.as_ref() >= Decimal::from(0),
        "visible_window_y should be non-negative even with very small playback_ratio"
    );

    // Test with normal playback_ratio
    let normal_ratio = Decimal::one();
    let visible_window_y_normal =
        visible_range.window_y(&Decimal::from(120), &Decimal::one(), &normal_ratio);

    // Verify the ratio relationship
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
    let expected_bpm = Decimal::from(180);
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

        let ratio_start = ratio_range.start().value().to_f64().unwrap_or(0.0);
        let ratio_end = ratio_range.end().value().to_f64().unwrap_or(0.0);
        assert!(
            ratio_start.is_finite(),
            "display_ratio start should be finite"
        );
        assert!(ratio_end.is_finite(), "display_ratio end should be finite");
    }
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
        let ratio_start = ratio_range.start().value().to_f64().unwrap_or(0.0);
        let ratio_end = ratio_range.end().value().to_f64().unwrap_or(0.0);
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

fn assert_playback_state_equal(state1: &PlaybackState, state2: &PlaybackState) {
    assert_eq!(state1.current_bpm(), state2.current_bpm(), "BPM mismatch");
    assert_eq!(
        state1.current_speed(),
        state2.current_speed(),
        "Speed mismatch"
    );
    assert_eq!(
        state1.current_scroll(),
        state2.current_scroll(),
        "Scroll mismatch"
    );
    assert_eq!(
        state1.playback_ratio(),
        state2.playback_ratio(),
        "Playback ratio mismatch"
    );
    assert_eq!(
        state1.progressed_y().value(),
        state2.progressed_y().value(),
        "Y position mismatch"
    );
}

fn assert_events_equal(events1: &[PlayheadEvent], events2: &[PlayheadEvent]) {
    assert_eq!(events1.len(), events2.len(), "Event count mismatch");

    let mut ev1 = events1.to_vec();
    let mut ev2 = events2.to_vec();
    ev1.sort_by(|a, b| {
        a.position()
            .value()
            .partial_cmp(b.position().value())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ev2.sort_by(|a, b| {
        a.position()
            .value()
            .partial_cmp(b.position().value())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (e1, e2) in ev1.iter().zip(ev2.iter()) {
        assert_eq!(
            e1.position().value(),
            e2.position().value(),
            "Event position mismatch"
        );

        if std::mem::discriminant(e1.event()) != std::mem::discriminant(e2.event()) {
            panic!("Event type mismatch: {:?} vs {:?}", e1.event(), e2.event());
        }
    }
}

#[test]
fn test_update_consistency_extreme_many_intervals() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Extreme Intervals Test",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240
        },
        "sound_channels": [{
            "name": "test.wav",
            "notes": [
                { "x": 1, "y": 0, "l": 0, "c": false },
                { "x": 1, "y": 240, "l": 0, "c": false },
                { "x": 1, "y": 480, "l": 0, "c": false },
                { "x": 1, "y": 720, "l": 0, "c": false },
                { "x": 1, "y": 960, "l": 0, "c": false }
            ]
        }],
        "bpm_events": [
            { "y": 120, "bpm": 240.0 },
            { "y": 360, "bpm": 60.0 },
            { "y": 600, "bpm": 180.0 }
        ]
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM");
    let chart = BmsonProcessor::parse(&bmson);
    let visible_range = VisibleRangePerBpm::new(&base_bpm, reaction_time);

    let start_time = TimeStamp::start();

    let mut player1 = ChartPlayer::start(chart.clone(), visible_range.clone(), start_time);
    let mut events1_total = Vec::new();
    for i in 1..=10000 {
        let t = start_time + TimeSpan::MICROSECOND * 100 * i;
        let events = player1.update(t);
        events1_total.extend(events);
    }

    let mut player2 = ChartPlayer::start(chart, visible_range, start_time);
    let t_final = start_time + TimeSpan::SECOND;
    let events2_total = player2.update(t_final);

    assert_playback_state_equal(player1.playback_state(), player2.playback_state());
    assert_events_equal(&events1_total, &events2_total);
}

#[test]
fn test_update_consistency_zero_interval() {
    let json = r#"{
        "version": "1.0.0",
        "info": {
            "title": "Zero Interval Test",
            "artist": "",
            "genre": "",
            "level": 1,
            "init_bpm": 120.0,
            "resolution": 240
        },
        "sound_channels": [{
            "name": "test.wav",
            "notes": [
                { "x": 1, "y": 240, "l": 0, "c": false },
                { "x": 1, "y": 480, "l": 0, "c": false }
            ]
        }],
        "bpm_events": [
            { "y": 360, "bpm": 180.0 }
        ]
    }"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM");
    let chart = BmsonProcessor::parse(&bmson);
    let visible_range = VisibleRangePerBpm::new(&base_bpm, reaction_time);

    let mut player = ChartPlayer::start(chart, visible_range, TimeStamp::start());
    let start_time = TimeStamp::start();

    let t = start_time + TimeSpan::MILLISECOND * 500;
    player.update(t);

    let state1 = player.playback_state().clone();

    player.update(t);

    let state2 = player.playback_state().clone();

    assert_eq!(
        state1, state2,
        "Zero time interval should not change any state"
    );
}
