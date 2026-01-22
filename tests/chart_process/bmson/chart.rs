#![cfg(feature = "bmson")]

use gametime::{TimeSpan, TimeStamp};

use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::PlayheadEvent;
use bms_rs::chart_process::prelude::*;

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
