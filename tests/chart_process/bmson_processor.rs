#![cfg(feature = "bmson")]

//! Integration tests for `bms_rs::chart_process::BmsonProcessor`.

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive};

use bms_rs::bms::Decimal;
use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::prelude::*;

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Setup a BMSON processor for testing (without calling `start_play`)
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
    ChartPlayer::new(chart, visible_range_per_bpm)
}

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
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::now();
    processor.start_play(start_time);

    // Progress slightly so the note at y=0.5 is inside visible window (0.6 measure default)
    // Advance slightly to ensure y=0.5 enters the visible window (default 0.6 measure)
    let t = start_time + TimeSpan::MILLISECOND * 100;
    let _ = processor.update(t);

    // Find the note and assert continue_play duration
    let mut found = false;
    for (ev, _) in processor.visible_events().unwrap() {
        if let ChartEvent::Note {
            continue_play: Some(dur),
            ..
        } = ev.event()
        {
            let secs = dur.as_secs_f64();
            assert!(
                (secs - 1.0).abs() < 0.02,
                "continue timepoint should be ~1.0s, got {:.6}",
                secs
            );
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
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::start();
    processor.start_play(start_time);

    let _ = processor.update(start_time + TimeSpan::MILLISECOND * 100);

    let mut got_any_ratio = false;
    for (ev, ratio_range) in processor.visible_events().unwrap() {
        if matches!(ev.event(), ChartEvent::Note { .. }) {
            let ratio = ratio_range.start().value().to_f64().unwrap_or(0.0);
            // Expected value after precision fix: 0.8333... (5/6)
            let expected = 5.0 / 6.0;
            assert!(
                (ratio - expected).abs() <= 1e-9,
                "expected display_ratio: {} for visible note, got {}",
                expected,
                ratio
            );
            got_any_ratio = true;
            break;
        }
    }
    assert!(got_any_ratio, "expected at least one visible note event");
}

#[test]
fn test_bmson_start_play_resets_scroll_to_one() {
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
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::start();
    processor.start_play(start_time);

    let after_scroll = start_time + TimeSpan::MILLISECOND * 600;
    let _ = processor.update(after_scroll);
    let state = processor.playback_state().unwrap();
    assert_ne!(*state.current_scroll(), Decimal::one());

    processor.start_play(after_scroll + TimeSpan::SECOND);
    let state = processor.playback_state().unwrap();
    assert_eq!(*state.current_scroll(), Decimal::one());
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

    let mut processor = setup_bmson_processor(json, TimeSpan::MILLISECOND * 600);
    let start_time = TimeStamp::start();
    processor.start_play(start_time);
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
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::now();
    processor.start_play(start_time);

    // Advance slightly to ensure y=0.25 enters the visible window (default 0.6 measure)
    let t = start_time + TimeSpan::MILLISECOND * 100;
    let _ = processor.update(t);

    let mut found = false;
    for (ev, _) in processor.visible_events().unwrap() {
        if let ChartEvent::Note {
            continue_play: Some(dur),
            ..
        } = ev.event()
        {
            let secs = dur.as_secs_f64();
            assert!(
                (secs - 0.5).abs() < 0.02,
                "continue timepoint should be ~0.5s, got {:.6}",
                secs
            );
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
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::now();
    processor.start_play(start_time);

    let t = start_time + TimeSpan::MILLISECOND * 100;
    let _ = processor.update(t);

    let mut some_count = 0;
    let mut none_count = 0;
    let mut durations = Vec::new();
    for (ev, _) in processor.visible_events().unwrap() {
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
    assert!(
        (a - 0.25).abs() < 0.02 && (b - 0.50).abs() < 0.02,
        "continue timepoints should be ~0.25s and ~0.50s, got {:.6} and {:.6}",
        a,
        b
    );
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
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::now();
    processor.start_play(start_time);

    // Advance to make the preload window cover the note at y=1.25
    // Note: With new playhead speed (1/240), speed is half of original (1/120)
    // So need more time to reach the same Y position
    // Also reaction time is now 1.2s instead of 0.6s
    let t = start_time + TimeSpan::MILLISECOND * 2400;
    let _ = processor.update(t);

    let mut found = false;
    for (ev, _) in processor.visible_events().unwrap() {
        if let ChartEvent::Note {
            continue_play: Some(dur),
            ..
        } = ev.event()
        {
            let secs = dur.as_secs_f64();
            if (secs - 3.5).abs() < 0.02 {
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
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::now();
    processor.start_play(start_time);
    let t = start_time + TimeSpan::MILLISECOND * 100;
    let _ = processor.update(t);

    let mut durations = Vec::new();
    let mut none_count = 0;
    for (ev, _) in processor.visible_events().unwrap() {
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
    assert!(
        (a - 0.5).abs() < 0.02 && (b - 0.75).abs() < 0.02,
        "Expected ~0.5s and ~0.75s timepoints, got {:.6} and {:.6}",
        a,
        b
    );
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
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::now();
    processor.start_play(start_time);

    let _ = processor.update(start_time);
    let events = processor.visible_events().unwrap();
    assert!(!events.is_empty(), "Should have visible events at start");

    let mut checked = false;
    for (ev, _) in events {
        if let ChartEvent::Note { .. } = ev.event() {
            let secs = ev.activate_time().as_secs_f64();
            assert!(
                (secs - 0.5).abs() < 0.02,
                "activate_time should be ~0.5s, got {:.6}",
                secs
            );
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
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::now();
    processor.start_play(start_time);
    let _ = processor.update(start_time);

    let events = processor.visible_events().unwrap();
    assert!(!events.is_empty(), "Should have visible events at start");

    let mut checked = false;
    for (ev, _) in events {
        if let ChartEvent::Note { .. } = ev.event() {
            let secs = ev.activate_time().as_secs_f64();
            assert!(
                (secs - 1.5).abs() < 0.02,
                "activate_time should be ~1.5s, got {:.6}",
                secs
            );
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
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::now();
    processor.start_play(start_time);
    let _ = processor.update(start_time);

    let events = processor.visible_events().unwrap();
    assert!(!events.is_empty(), "Should have visible events at start");

    let mut checked = false;
    for (ev, _) in events {
        if let ChartEvent::Note { .. } = ev.event() {
            let secs = ev.activate_time().as_secs_f64();
            assert!(
                (secs - 2.5).abs() < 0.02,
                "activate_time should be ~2.5s, got {:.6}",
                secs
            );
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
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::start();
    processor.start_play(start_time);

    // Verify standard conditions
    let state = processor.playback_state().unwrap();
    assert_eq!(*state.current_bpm(), Decimal::from(120));
    assert_eq!(*state.playback_ratio(), Decimal::one());

    // Calculate expected visible window Y
    let base_bpm = BaseBpm::from(Decimal::from(120));
    let visible_range = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let state = processor.playback_state().unwrap();
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

    // Verify: time_to_cross should equal reaction_time
    let expected_time =
        Decimal::from(reaction_time.as_nanos().max(0)) / Decimal::from(NANOS_PER_SECOND);
    let diff = (time_to_cross.clone() - expected_time).abs();

    assert!(
        diff < Decimal::from(1u64), // Allow 1ms error margin
        "Expected time_to_cross ≈ reaction_time (600ms), got {:.6}s, diff: {:.6}s",
        time_to_cross.to_f64().unwrap_or(0.0),
        diff.to_f64().unwrap_or(0.0)
    );
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
    let mut processor = setup_bmson_processor(json, reaction_time);
    let start_time = TimeStamp::start();
    processor.start_play(start_time);

    // Get initial visible_window_y (playback_ratio = 1)
    let base_bpm = BaseBpm::from(Decimal::from(120));
    let visible_range = VisibleRangePerBpm::new(&base_bpm, reaction_time);

    let state = processor.playback_state().unwrap();
    let visible_window_y_ratio_1 =
        visible_range.window_y(state.current_bpm(), &Decimal::one(), &Decimal::one());

    // Set playback_ratio to 0.5
    processor.post_events(std::iter::once(ControlEvent::SetPlaybackRatio {
        ratio: Decimal::from(0.5),
    }));

    // Verify playback_ratio changed
    let state = processor.playback_state().unwrap();
    assert_eq!(*state.playback_ratio(), Decimal::from(0.5));

    // Get new visible_window_y (playback_ratio = 0.5)
    let state = processor.playback_state().unwrap();
    let visible_window_y_ratio_0_5 =
        visible_range.window_y(state.current_bpm(), &Decimal::one(), state.playback_ratio());

    // Verify: visible_window_y should halve when playback_ratio halves
    let ratio = visible_window_y_ratio_0_5.as_ref() / visible_window_y_ratio_1.as_ref();
    assert!(
        (ratio.clone() - Decimal::from(0.5)).abs() < Decimal::from(1u64),
        "Expected visible_window_y to halve when playback_ratio halves, ratio: {:.2}",
        ratio.to_f64().unwrap_or(0.0)
    );

    // Calculate time to cross window with playback_ratio = 0.5
    let velocity = Decimal::from(120) * Decimal::from(0.5) / Decimal::from(240u64);
    let time_to_cross = visible_window_y_ratio_0_5.as_ref() / velocity;

    // Verify: time_to_cross should still equal reaction_time
    let expected_time =
        Decimal::from(reaction_time.as_nanos().max(0)) / Decimal::from(NANOS_PER_SECOND);
    let diff = (time_to_cross.clone() - expected_time).abs();

    assert!(
        diff < Decimal::from(1u64),
        "Expected time_to_cross ≈ reaction_time even with playback_ratio=0.5, got {:.6}s",
        time_to_cross.to_f64().unwrap_or(0.0)
    );
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
    let _processor = setup_bmson_processor(json, reaction_time);

    // Test with very small playback_ratio (not zero, to avoid division by zero)
    let base_bpm = BaseBpm::from(Decimal::from(120));
    let visible_range = VisibleRangePerBpm::new(&base_bpm, reaction_time);

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

    assert!(
        (actual_ratio - expected_ratio).abs() < Decimal::from(1u64),
        "visible_window_y should scale linearly with playback_ratio"
    );
}
