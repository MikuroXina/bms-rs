//! Integration tests for `UniversalChartPlayer` convenience methods.
//!
//! Tests the `is_playing()`, `current_y()`, and `reset()` methods.

use std::collections::{BTreeMap, HashMap};

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive, Zero};

use bms_rs::bms::Decimal;
use bms_rs::bms::prelude::{Key, NoteKind, PlayerSide};
use bms_rs::chart_process::base_bpm::{BaseBpm, VisibleRangePerBpm};
use bms_rs::chart_process::player::FlowEvent;
use bms_rs::chart_process::player::UniversalChartPlayer;
use bms_rs::chart_process::resource::{BmpId, HashMapResourceMapping, WavId};
use bms_rs::chart_process::{
    AllEventsIndex, ChartEvent, ChartEventId, ControlEvent, PlayheadEvent, YCoordinate,
};

use super::dsl::{TestPlayerDriver, test_player_driver};

#[test]
fn test_is_playing() {
    test_player_driver()
        .check(|p| assert!(p.is_playing()))
        .run();
}

#[test]
fn test_current_y() {
    test_player_driver()
        .past(TimeSpan::SECOND)
        .view(|p| assert!(p.current_y().value() > &Zero::zero()))
        .run();
}

#[test]
fn test_reset() {
    // This test requires manual reset call, so original implementation is retained
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
    // Since DSL time advancement is cumulative, this test requires special handling
    // We manually test time rewind behavior
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
    test_player_driver()
        .past_ms(100)
        .events(|evs| assert_eq!(evs.len(), 0, "Update 1 should return 0 events"))
        .past_ms(100)
        .events(|evs| assert_eq!(evs.len(), 0, "Update 2 should return 0 events"))
        .past_ms(100)
        .events(|evs| assert_eq!(evs.len(), 0, "Update 3 should return 0 events"))
        .past_ms(100)
        .events(|evs| assert_eq!(evs.len(), 0, "Update 4 should return 0 events"))
        .past_ms(100)
        .events(|evs| assert_eq!(evs.len(), 0, "Update 5 should return 0 events"))
        .past_ms(100)
        .events(|evs| assert_eq!(evs.len(), 0, "Update 6 should return 0 events"))
        .past_ms(100)
        .events(|evs| assert_eq!(evs.len(), 0, "Update 7 should return 0 events"))
        .past_ms(100)
        .events(|evs| assert_eq!(evs.len(), 0, "Update 8 should return 0 events"))
        .past_ms(100)
        .events(|evs| assert_eq!(evs.len(), 0, "Update 9 should return 0 events"))
        .past_ms(100)
        .events(|evs| assert_eq!(evs.len(), 0, "Update 10 should return 0 events"))
        .check(|p| assert!(p.is_playing()))
        .run();
}

#[test]
fn test_resources_empty() {
    test_player_driver()
        .check(|p| {
            let mut wav_count = 0;
            p.for_each_audio_file(|_id, _path| {
                wav_count += 1;
            });
            assert_eq!(wav_count, 0);

            let mut bmp_count = 0;
            p.for_each_bmp_file(|_id, _path| {
                bmp_count += 1;
            });
            assert_eq!(bmp_count, 0);
        })
        .run();
}

#[test]
fn test_update_before_start() {
    // Manual implementation needed to test behavior before start
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
    let count = player.update(now).count();
    assert_eq!(count, 0);
    assert_eq!(player.started_at(), None);
}

#[test]
fn test_events_in_time_range_with_empty_player() {
    test_player_driver()
        .check(|p| {
            let count = p
                .events_in_time_range(TimeSpan::ZERO..=TimeSpan::SECOND * 10)
                .count();
            assert_eq!(count, 0);
        })
        .run();
}

#[test]
fn test_universal_chart_player_creation() {
    // This test requires custom resources, so cannot use standard test_player_driver
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
    // This test requires custom resources
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
    // Manual implementation needed to test update behavior before and after start
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
    player.start_play(now);
    assert_eq!(player.started_at(), Some(now));

    // Advance time and check for events
    let after_1s = now + TimeSpan::SECOND;
    // No events in empty chart
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

    let player = UniversalChartPlayer::new(
        all_events,
        flow_events_by_y,
        init_bpm,
        visible_range_per_bpm,
        resources,
    );

    TestPlayerDriver::new(player)
        .check(|p| {
            // Query events in range [0.5s, 1.5s]
            let events: Vec<_> = p
                .events_in_time_range(TimeSpan::MILLISECOND * 500..=TimeSpan::MILLISECOND * 1500)
                .collect();
            assert_eq!(events.len(), 1);
            assert_eq!(events.first().unwrap().activate_time(), &TimeSpan::SECOND);

            // Query events in range [0s, 2.5s]
            let count = p
                .events_in_time_range(TimeSpan::ZERO..=TimeSpan::MILLISECOND * 2500)
                .count();
            assert_eq!(count, 3);
        })
        .run();
}

#[test]
fn test_universal_chart_player_post_events() {
    test_player_driver()
        .check(|p| assert_eq!(p.playback_ratio(), &Decimal::one()))
        .post_events([ControlEvent::SetPlaybackRatio {
            ratio: Decimal::from(2),
        }])
        .check(|p| assert_eq!(p.playback_ratio(), &Decimal::from(2)))
        .post_events([ControlEvent::SetVisibleRangePerBpm {
            visible_range_per_bpm: VisibleRangePerBpm::new(
                &BaseBpm::new(Decimal::from(120)),
                TimeSpan::SECOND * 2,
            ),
        }])
        .check(|p| {
            let base_bpm = BaseBpm::new(Decimal::from(120));
            let new_range = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND * 2);
            assert_eq!(p.visible_range_per_bpm(), &new_range);
        })
        .run();
}

#[test]
fn test_universal_chart_player_visible_events() {
    test_player_driver()
        .past(TimeSpan::SECOND)
        .events(|evs| {
            // Get visible events (should be empty)
            // No events, so it should be empty
            assert_eq!(evs.len(), 0);
        })
        .run();
}

#[test]
fn test_universal_chart_player_start_play() {
    // Manual implementation needed to test start_play method
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

    assert_eq!(player.started_at(), None);

    let start_time = TimeStamp::now();
    player.start_play(start_time);
    assert_eq!(player.started_at(), Some(start_time));

    // Calling start_play again should update the start time
    let new_start_time = start_time + TimeSpan::SECOND;
    player.start_play(new_start_time);
    assert_eq!(player.started_at(), Some(new_start_time));
}

#[test]
fn test_multiple_flow_events_at_same_position_applied_correctly() {
    // Test that multiple flow events (BPM and Speed) at the same position
    // are all applied correctly during playback

    let wav_map = HashMap::new();
    let bmp_map = HashMap::new();
    let resources = HashMapResourceMapping::new(wav_map, bmp_map);

    // Create all_events index (empty for this test)
    let all_events = AllEventsIndex::new(BTreeMap::new());

    // Create flow events at the same Y position
    // At Y=100, both BPM changes to 180 and Speed changes to 2.0
    let mut flow_events_by_y = BTreeMap::new();
    let event_y = YCoordinate::new(Decimal::from(100));

    flow_events_by_y.insert(
        event_y,
        vec![
            FlowEvent::Bpm(Decimal::from(180)),
            FlowEvent::Speed(Decimal::from(2)),
        ],
    );

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

    // Calculate time to reach Y=100 with initial BPM 120 and Speed 1.0
    // velocity = (120 / 240) * 1.0 = 0.5
    // Y = velocity * time => time = Y / velocity = 100 / 0.5 = 200 seconds
    let initial_velocity = Decimal::from(120) / Decimal::from(240) * Decimal::one();
    let time_to_reach_event_y = Decimal::from(100) / initial_velocity;
    let time_to_reach_event_secs = time_to_reach_event_y.to_u64().unwrap();

    TestPlayerDriver::new(player)
        .check(|p| {
            assert_eq!(p.current_bpm(), &Decimal::from(120));
            assert_eq!(p.current_speed(), &Decimal::one());
        })
        .past(TimeSpan::SECOND * ((time_to_reach_event_secs - 1) as i64))
        .view(|p| {
            // Should still have initial values
            assert_eq!(p.current_bpm(), &Decimal::from(120));
            assert_eq!(p.current_speed(), &Decimal::one());
        })
        .past(TimeSpan::SECOND * 11)
        .view(|p| {
            // Now both BPM and Speed should be updated
            assert_eq!(
                p.current_bpm(),
                &Decimal::from(180),
                "BPM should be updated to 180 after passing the event"
            );
            assert_eq!(
                p.current_speed(),
                &Decimal::from(2),
                "Speed should be updated to 2.0 after passing the event"
            );

            // Verify that the player continued to move after the event
            assert!(p.current_y().value() > &Decimal::from(100));
        })
        .run();
}

#[test]
fn test_flow_event_priority_ordering_during_playback() {
    // Test that when BPM and Speed change at the same position,
    // BPM is applied before Speed (correct priority order)

    let wav_map = HashMap::new();
    let bmp_map = HashMap::new();
    let resources = HashMapResourceMapping::new(wav_map, bmp_map);

    let all_events = AllEventsIndex::new(BTreeMap::new());

    // Create flow events at Y=50 with opposite insertion order
    // Speed first, then BPM (to test priority ordering)
    let mut flow_events_by_y = BTreeMap::new();
    let event_y = YCoordinate::new(Decimal::from(50));

    flow_events_by_y.insert(
        event_y,
        vec![
            // Insert in reverse order to test that priority is used, not insertion order
            FlowEvent::Speed(Decimal::from(3)),
            FlowEvent::Bpm(Decimal::from(240)),
        ],
    );

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

    // Calculate time to reach Y=50
    let initial_velocity = Decimal::from(120) / Decimal::from(240) * Decimal::one();
    let time_to_reach_event_y = Decimal::from(50) / initial_velocity;
    let time_to_reach_event_secs = time_to_reach_event_y.to_u64().unwrap();

    TestPlayerDriver::new(player)
        .past(TimeSpan::SECOND * ((time_to_reach_event_secs + 5) as i64))
        .view(|p| {
            // Both should be updated regardless of insertion order
            assert_eq!(p.current_bpm(), &Decimal::from(240));
            assert_eq!(p.current_speed(), &Decimal::from(3));

            // The player should have moved further due to higher velocity
            // Expected velocity after event: (240 / 240) * 3 = 3.0
            assert!(p.current_y().value() > &Decimal::from(50));
        })
        .run();
}
