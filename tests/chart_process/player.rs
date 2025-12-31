//! Integration tests for `UniversalChartPlayer` convenience methods.
//!
//! Tests the `is_playing()`, `current_y()`, and `reset()` methods.

use std::collections::{BTreeMap, HashMap};

use bms_rs::bms::Decimal;
use bms_rs::bms::prelude::{Key, NoteKind, PlayerSide};
use bms_rs::chart_process::base_bpm::{BaseBpm, VisibleRangePerBpm};
use bms_rs::chart_process::core::{ChartEventId, PlayheadEvent};
use bms_rs::chart_process::player::UniversalChartPlayer;
use bms_rs::chart_process::resource::{BmpId, HashMapResourceMapping, WavId};
use bms_rs::chart_process::{AllEventsIndex, ChartEvent, ControlEvent};
use gametime::{TimeSpan, TimeStamp};
use num::{One, Zero};

fn create_test_player() -> UniversalChartPlayer<HashMapResourceMapping> {
    let wav_map = HashMap::new();
    let bmp_map = HashMap::new();
    let resources = HashMapResourceMapping::new(wav_map, bmp_map);

    let all_events = AllEventsIndex::new(BTreeMap::new());
    let flow_events_by_y = BTreeMap::new();
    let init_bpm = Decimal::from(120);
    let base_bpm = BaseBpm::new(Decimal::from(120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND);

    UniversalChartPlayer::new(
        all_events,
        flow_events_by_y,
        init_bpm,
        visible_range_per_bpm,
        resources,
    )
}

#[test]
fn test_is_playing() {
    let mut player = create_test_player();

    // Initially not playing
    assert!(!player.is_playing());

    // After starting playback
    player.start_play(TimeStamp::now());
    assert!(player.is_playing());
}

#[test]
fn test_current_y() {
    let mut player = create_test_player();

    // Y coordinate should be 0 before playback starts
    assert_eq!(player.current_y().value(), &Zero::zero());

    // Start playback
    let start_time = TimeStamp::now();
    player.start_play(start_time);

    // Update 1 second
    let after_1s = start_time + TimeSpan::SECOND;
    let _ = player.update(after_1s).count();

    // Y coordinate should be greater than 0
    assert!(player.current_y().value() > &Zero::zero());
}

#[test]
fn test_reset() {
    let mut player = create_test_player();

    // Play for some time
    let start_time = TimeStamp::now();
    player.start_play(start_time);
    let after_5s = start_time + TimeSpan::SECOND * 5;
    let _ = player.update(after_5s).count();

    // Verify playing state
    assert!(player.is_playing());
    let y_before_reset = player.current_y().value().clone();
    assert!(y_before_reset > Zero::zero());

    // Reset
    player.reset();

    // Verify reset state
    assert!(!player.is_playing());
    assert_eq!(player.started_at(), None);
    assert_eq!(player.current_y().value(), &Zero::zero());

    // Verify BPM and other parameters are NOT reset
    assert_eq!(player.current_bpm(), &Decimal::from(120));
    assert_eq!(player.current_speed(), &Decimal::one());
    assert_eq!(player.current_scroll(), &Decimal::one());
}

#[test]
fn test_update_with_time_rewind() {
    let mut player = create_test_player();

    let start_time = TimeStamp::now();
    player.start_play(start_time);

    let t1 = start_time + TimeSpan::SECOND;
    let _ = player.update(t1).count();

    // Attempt time rewind (now < last_poll_at)
    let t_rewound = t1 - TimeSpan::MILLISECOND * 500;
    let count = player.update(t_rewound).count();

    // Should return empty, no events should be triggered
    assert_eq!(count, 0);
}

#[test]
fn test_multiple_consecutive_updates() {
    let mut player = create_test_player();

    let start_time = TimeStamp::now();
    player.start_play(start_time);

    // Multiple consecutive updates
    for i in 1..=10 {
        let t = start_time + TimeSpan::MILLISECOND * 100 * i;
        let count = player.update(t).count();
        // No events, so should return 0
        assert_eq!(count, 0, "Update {} should return 0 events", i);
    }

    // Verify playing state is still active
    assert!(player.is_playing());
}

#[test]
fn test_resources_empty() {
    let player = create_test_player();

    // Verify for_each methods handle empty resources correctly
    let mut wav_count = 0;
    player.for_each_audio_file(|_id, _path| {
        wav_count += 1;
    });
    assert_eq!(wav_count, 0);

    let mut bmp_count = 0;
    player.for_each_bmp_file(|_id, _path| {
        bmp_count += 1;
    });
    assert_eq!(bmp_count, 0);
}

#[test]
fn test_update_before_start() {
    let mut player = create_test_player();

    // Call update before calling start_play
    let now = TimeStamp::now();
    let count = player.update(now).count();

    // Should return empty, no events should be triggered
    assert_eq!(count, 0);
    assert_eq!(player.started_at(), None);
}

#[test]
fn test_events_in_time_range_with_empty_player() {
    let player = create_test_player();

    // Query any time range, should return empty
    let count = player
        .events_in_time_range(TimeSpan::ZERO..=TimeSpan::SECOND * 10)
        .count();
    assert_eq!(count, 0);
}

#[test]
fn test_universal_chart_player_creation() {
    let mut wav_map = HashMap::new();
    wav_map.insert(WavId::new(0), std::path::PathBuf::from("test.wav"));

    let mut bmp_map = HashMap::new();
    bmp_map.insert(BmpId::new(0), std::path::PathBuf::from("test.bmp"));

    let resources = HashMapResourceMapping::new(wav_map, bmp_map);

    let all_events = AllEventsIndex::new(BTreeMap::new());
    let flow_events_by_y = BTreeMap::new();
    let init_bpm = Decimal::from(120);
    let base_bpm = BaseBpm::new(Decimal::from(120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND);

    let player = UniversalChartPlayer::new(
        all_events,
        flow_events_by_y,
        init_bpm,
        visible_range_per_bpm,
        resources,
    );

    assert_eq!(player.current_bpm(), &Decimal::from(120));
    assert_eq!(player.current_speed(), &Decimal::one());
    assert_eq!(player.current_scroll(), &Decimal::one());
}

#[test]
fn test_universal_chart_player_resource_access() {
    let mut wav_map = HashMap::new();
    wav_map.insert(WavId::new(0), std::path::PathBuf::from("audio1.wav"));
    wav_map.insert(WavId::new(1), std::path::PathBuf::from("audio2.wav"));

    let mut bmp_map = HashMap::new();
    bmp_map.insert(BmpId::new(0), std::path::PathBuf::from("img1.bmp"));
    bmp_map.insert(BmpId::new(1), std::path::PathBuf::from("img2.bmp"));

    let resources = HashMapResourceMapping::new(wav_map.clone(), bmp_map.clone());

    let all_events = AllEventsIndex::new(BTreeMap::new());
    let flow_events_by_y = BTreeMap::new();
    let init_bpm = Decimal::from(120);
    let base_bpm = BaseBpm::new(Decimal::from(120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND);

    let player = UniversalChartPlayer::new(
        all_events,
        flow_events_by_y,
        init_bpm,
        visible_range_per_bpm,
        resources,
    );

    // Test audio files access
    let mut audio_count = 0;
    let mut found_audio1 = false;
    player.for_each_audio_file(|id, path| {
        audio_count += 1;
        if id == WavId::new(0) && path == std::path::Path::new("audio1.wav") {
            found_audio1 = true;
        }
    });
    assert_eq!(audio_count, 2);
    assert!(found_audio1);

    // Test BMP files access
    let mut bmp_count = 0;
    let mut found_img1 = false;
    player.for_each_bmp_file(|id, path| {
        bmp_count += 1;
        if id == BmpId::new(0) && path == std::path::Path::new("img1.bmp") {
            found_img1 = true;
        }
    });
    assert_eq!(bmp_count, 2);
    assert!(found_img1);
}

#[test]
fn test_universal_chart_player_update() {
    let wav_map = HashMap::new();
    let bmp_map = HashMap::new();
    let resources = HashMapResourceMapping::new(wav_map, bmp_map);

    let all_events = AllEventsIndex::new(BTreeMap::new());
    let flow_events_by_y = BTreeMap::new();
    let init_bpm = Decimal::from(120);
    let base_bpm = BaseBpm::new(Decimal::from(120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND);

    let mut player = UniversalChartPlayer::new(
        all_events,
        flow_events_by_y,
        init_bpm,
        visible_range_per_bpm,
        resources,
    );

    let now = TimeStamp::now();

    // Test that update doesn't produce events when playback hasn't started
    assert_eq!(player.update(now).count(), 0);

    // Start playback
    player.start_play(now);
    assert_eq!(player.started_at(), Some(now));

    // Advance time
    let after_1s = now + TimeSpan::SECOND;
    // No events, so it should be empty
    assert_eq!(player.update(after_1s).count(), 0);
}

#[test]
fn test_universal_chart_player_events_in_time_range() {
    let wav_map = HashMap::new();
    let bmp_map = HashMap::new();
    let resources = HashMapResourceMapping::new(wav_map, bmp_map);

    let mut events_by_y = BTreeMap::new();
    events_by_y.insert(
        bms_rs::chart_process::YCoordinate::new(Decimal::from(100)),
        vec![PlayheadEvent::new(
            ChartEventId::new(0),
            bms_rs::chart_process::YCoordinate::new(Decimal::from(100)),
            ChartEvent::Note {
                side: PlayerSide::Player1,
                key: Key::Key(1),
                kind: NoteKind::Visible,
                wav_id: None,
                length: None,
                continue_play: None,
            },
            TimeSpan::ZERO,
        )],
    );
    events_by_y.insert(
        bms_rs::chart_process::YCoordinate::new(Decimal::from(200)),
        vec![PlayheadEvent::new(
            ChartEventId::new(1),
            bms_rs::chart_process::YCoordinate::new(Decimal::from(200)),
            ChartEvent::Note {
                side: PlayerSide::Player1,
                key: Key::Key(2),
                kind: NoteKind::Visible,
                wav_id: None,
                length: None,
                continue_play: None,
            },
            TimeSpan::SECOND,
        )],
    );
    events_by_y.insert(
        bms_rs::chart_process::YCoordinate::new(Decimal::from(300)),
        vec![PlayheadEvent::new(
            ChartEventId::new(2),
            bms_rs::chart_process::YCoordinate::new(Decimal::from(300)),
            ChartEvent::Note {
                side: PlayerSide::Player1,
                key: Key::Key(3),
                kind: NoteKind::Visible,
                wav_id: None,
                length: None,
                continue_play: None,
            },
            TimeSpan::SECOND * 2,
        )],
    );

    let all_events = AllEventsIndex::new(events_by_y);
    let flow_events_by_y = BTreeMap::new();
    let init_bpm = Decimal::from(120);
    let base_bpm = BaseBpm::new(Decimal::from(120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND);

    let mut player = UniversalChartPlayer::new(
        all_events,
        flow_events_by_y,
        init_bpm,
        visible_range_per_bpm,
        resources,
    );

    // Need to call start_play first
    player.start_play(TimeStamp::now());

    // Query events in range [0.5s, 1.5s]
    let events: Vec<_> = player
        .events_in_time_range(TimeSpan::MILLISECOND * 500..=TimeSpan::MILLISECOND * 1500)
        .collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().activate_time(), &TimeSpan::SECOND);

    // Query events in range [0s, 2.5s]
    let count = player
        .events_in_time_range(TimeSpan::ZERO..=TimeSpan::MILLISECOND * 2500)
        .count();
    assert_eq!(count, 3);
}

#[test]
fn test_universal_chart_player_post_events() {
    let wav_map = HashMap::new();
    let bmp_map = HashMap::new();
    let resources = HashMapResourceMapping::new(wav_map, bmp_map);

    let all_events = AllEventsIndex::new(BTreeMap::new());
    let flow_events_by_y = BTreeMap::new();
    let init_bpm = Decimal::from(120);
    let base_bpm = BaseBpm::new(Decimal::from(120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND);

    let mut player = UniversalChartPlayer::new(
        all_events,
        flow_events_by_y,
        init_bpm,
        visible_range_per_bpm,
        resources,
    );

    // Verify initial state
    assert_eq!(player.playback_ratio(), &Decimal::one());

    // Send playback ratio control event
    let new_ratio = Decimal::from(2);
    player.post_events(
        [ControlEvent::SetPlaybackRatio {
            ratio: new_ratio.clone(),
        }]
        .into_iter(),
    );
    assert_eq!(player.playback_ratio(), &new_ratio);

    // Send visible range control event
    let new_range = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND * 2);
    player.post_events(
        [ControlEvent::SetVisibleRangePerBpm {
            visible_range_per_bpm: new_range.clone(),
        }]
        .into_iter(),
    );
    assert_eq!(player.visible_range_per_bpm(), &new_range);
}

#[test]
fn test_universal_chart_player_visible_events() {
    let wav_map = HashMap::new();
    let bmp_map = HashMap::new();
    let resources = HashMapResourceMapping::new(wav_map, bmp_map);

    let all_events = AllEventsIndex::new(BTreeMap::new());
    let flow_events_by_y = BTreeMap::new();
    let init_bpm = Decimal::from(120);
    let base_bpm = BaseBpm::new(Decimal::from(120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND);

    let mut player = UniversalChartPlayer::new(
        all_events,
        flow_events_by_y,
        init_bpm,
        visible_range_per_bpm,
        resources,
    );

    let start_time = TimeStamp::now();
    player.start_play(start_time);

    // Advance time
    let after_1s = start_time + TimeSpan::SECOND;
    let _ = player.update(after_1s).count();

    // Get visible events (should be empty)
    // No events, so it should be empty
    assert_eq!(player.visible_events().count(), 0);
}

#[test]
fn test_universal_chart_player_start_play() {
    let wav_map = HashMap::new();
    let bmp_map = HashMap::new();
    let resources = HashMapResourceMapping::new(wav_map, bmp_map);

    let all_events = AllEventsIndex::new(BTreeMap::new());
    let flow_events_by_y = BTreeMap::new();
    let init_bpm = Decimal::from(120);
    let base_bpm = BaseBpm::new(Decimal::from(120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND);

    let mut player = UniversalChartPlayer::new(
        all_events,
        flow_events_by_y,
        init_bpm,
        visible_range_per_bpm,
        resources,
    );

    // Verify playback hasn't started
    assert_eq!(player.started_at(), None);

    // Start playback
    let start_time = TimeStamp::now();
    player.start_play(start_time);

    // Verify playback has started
    assert_eq!(player.started_at(), Some(start_time));

    // Calling start_play again should update the start time
    let new_start_time = start_time + TimeSpan::SECOND;
    player.start_play(new_start_time);
    assert_eq!(player.started_at(), Some(new_start_time));
}
