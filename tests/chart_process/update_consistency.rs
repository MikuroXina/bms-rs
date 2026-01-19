//! Update call interval consistency tests
//!
//! Verifies that "multiple short-interval update calls" behave identically to
//! "a single long-interval update call".

#![cfg(feature = "bmson")]

use gametime::{TimeSpan, TimeStamp};

use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::PlayheadEvent;
use bms_rs::chart_process::prelude::*;

/// Parses BMSON JSON and returns `(ParsedChart, BaseBpm)`
fn parse_chart(json: &str) -> (ParsedChart, BaseBpm) {
    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM");
    let chart = BmsonProcessor::parse(&bmson);

    (chart, base_bpm)
}

/// Asserts that two `PlaybackState` values are equal
fn assert_playback_state_equal(state1: &PlaybackState, state2: &PlaybackState) {
    assert_eq!(
        state1.current_bpm(),
        state2.current_bpm(),
        "BPM mismatch: {} vs {}",
        state1.current_bpm(),
        state2.current_bpm()
    );
    assert_eq!(
        state1.current_speed(),
        state2.current_speed(),
        "Speed mismatch: {} vs {}",
        state1.current_speed(),
        state2.current_speed()
    );
    assert_eq!(
        state1.current_scroll(),
        state2.current_scroll(),
        "Scroll mismatch: {} vs {}",
        state1.current_scroll(),
        state2.current_scroll()
    );
    assert_eq!(
        state1.playback_ratio(),
        state2.playback_ratio(),
        "Playback ratio mismatch: {} vs {}",
        state1.playback_ratio(),
        state2.playback_ratio()
    );

    // Y coordinate: require exact match (no floating point tolerance)
    assert_eq!(
        state1.progressed_y().value(),
        state2.progressed_y().value(),
        "Y position mismatch: {} vs {}",
        state1.progressed_y().value(),
        state2.progressed_y().value()
    );
}

/// Asserts that two event lists are equal
fn assert_events_equal(events1: &[PlayheadEvent], events2: &[PlayheadEvent]) {
    assert_eq!(
        events1.len(),
        events2.len(),
        "Event count mismatch: {} vs {}",
        events1.len(),
        events2.len()
    );

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
        // Event position: require exact match
        assert_eq!(
            e1.position().value(),
            e2.position().value(),
            "Event position mismatch: {} vs {}",
            e1.position().value(),
            e2.position().value()
        );

        // Verify event types match
        if std::mem::discriminant(e1.event()) != std::mem::discriminant(e2.event()) {
            panic!("Event type mismatch: {:?} vs {:?}", e1.event(), e2.event());
        }
    }
}

/// Test 1: Extreme many short intervals vs single long interval
#[test]
fn test_update_consistency_extreme_many_intervals() {
    // Complex scenario with BPM changes
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
    let (chart, base_bpm) = parse_chart(json);
    let visible_range = VisibleRangePerBpm::new(&base_bpm, reaction_time);

    let start_time = TimeStamp::start();

    // Approach 1: 10000 updates with 0.1ms interval (extreme many short intervals)
    let mut player1 = ChartPlayer::start(chart.clone(), visible_range.clone(), start_time);
    let mut events1_total = Vec::new();
    for i in 1..=10000 {
        let t = start_time + TimeSpan::MICROSECOND * 100 * i;
        let events = player1.update(t);
        events1_total.extend(events);
    }

    // Approach 2: Single update with 1000ms interval
    let mut player2 = ChartPlayer::start(chart, visible_range, start_time);
    let t_final = start_time + TimeSpan::SECOND;
    let events2_total = player2.update(t_final);

    // Verify final states are completely identical
    assert_playback_state_equal(player1.playback_state(), player2.playback_state());
    assert_events_equal(&events1_total, &events2_total);
}

/// Test 2: Zero time interval boundary
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
    let (chart, base_bpm) = parse_chart(json);
    let visible_range = VisibleRangePerBpm::new(&base_bpm, reaction_time);

    let mut player = ChartPlayer::start(chart, visible_range, TimeStamp::start());
    let start_time = TimeStamp::start();

    // Advance to a time point
    let t = start_time + TimeSpan::MILLISECOND * 500;
    player.update(t);

    // Record state
    let state1 = player.playback_state().clone();

    // Call update again with the same time (zero time interval)
    player.update(t);

    let state2 = player.playback_state().clone();

    // Verify state remains unchanged
    assert_eq!(
        state1, state2,
        "Zero time interval should not change any state"
    );
}
