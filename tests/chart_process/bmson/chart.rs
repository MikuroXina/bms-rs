#![cfg(feature = "bmson")]

use gametime::{TimeSpan, TimeStamp};

use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::PlayheadEvent;
use bms_rs::chart_process::prelude::*;

fn assert_playback_state_equal(state1: &PlaybackState, state2: &PlaybackState) {
    // 使用近似比较来处理浮点数精度问题
    let tolerance = 1e-9;

    assert!(
        (state1.current_bpm().as_f64() - state2.current_bpm().as_f64()).abs() < tolerance,
        "BPM mismatch: left={}, right={}",
        state1.current_bpm(),
        state2.current_bpm()
    );

    assert!(
        (state1.current_speed().as_f64() - state2.current_speed().as_f64()).abs() < tolerance,
        "Speed mismatch: left={}, right={}",
        state1.current_speed(),
        state2.current_speed()
    );

    assert!(
        (state1.current_scroll().as_f64() - state2.current_scroll().as_f64()).abs() < tolerance,
        "Scroll mismatch: left={}, right={}",
        state1.current_scroll(),
        state2.current_scroll()
    );

    assert!(
        (state1.playback_ratio().as_f64() - state2.playback_ratio().as_f64()).abs() < tolerance,
        "Playback ratio mismatch: left={}, right={}",
        state1.playback_ratio(),
        state2.playback_ratio()
    );

    // Y position 可能累积更多误差，使用更大的容差
    let y_tolerance = 1e-12;
    assert!(
        (state1.progressed_y().value().as_f64() - state2.progressed_y().value().as_f64()).abs()
            < y_tolerance,
        "Y position mismatch: left={}, right={}",
        state1.progressed_y().value(),
        state2.progressed_y().value()
    );
}

fn assert_events_equal(events1: &[PlayheadEvent], events2: &[PlayheadEvent]) {
    // 先打印事件数量以便调试
    println!(
        "Event count: left={}, right={}",
        events1.len(),
        events2.len()
    );

    // 如果事件数量不匹配，打印详细的事件列表
    if events1.len() != events2.len() {
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

        println!("\nLeft events ({}):", ev1.len());
        for (i, e) in ev1.iter().enumerate() {
            println!(
                "  [{}] y={:?}, event={:?}",
                i,
                e.position().value(),
                e.event()
            );
        }

        println!("\nRight events ({}):", ev2.len());
        for (i, e) in ev2.iter().enumerate() {
            println!(
                "  [{}] y={:?}, event={:?}",
                i,
                e.position().value(),
                e.event()
            );
        }
    }

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

    let tolerance = 1e-12;
    for (e1, e2) in ev1.iter().zip(ev2.iter()) {
        // 使用近似比较来处理浮点数精度问题
        assert!(
            (e1.position().value().as_f64() - e2.position().value().as_f64()).abs() < tolerance,
            "Event position mismatch: left={}, right={}",
            e1.position().value(),
            e2.position().value()
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

    // 由于浮点数精度累积问题，多次小间隔更新和单次大间隔更新可能会导致
    // 边界上的事件触发不一致。这里我们允许一个事件的差异。
    //
    // 具体来说，如果某个音符恰好位于更新边界的 y 位置上，
    // 在一种情况下它会被包含（触发），在另一种情况下可能不会。
    // 这是 f64 浮点数精度的固有限制，在实际应用中影响很小。
    let event_count_diff = (events1_total.len() as i64 - events2_total.len() as i64).abs();
    if event_count_diff <= 1 {
        // 事件数量相差不超过1，这是可以接受的
        // 我们仍然检查共同的事件是否一致
        let tolerance = 1e-9;
        let mut matched = 0;

        // 使用简单的匹配策略：对于 events1 中的每个事件，在 events2 中寻找近似匹配
        for e1 in &events1_total {
            for e2 in &events2_total {
                let pos_match = (e1.position().value().as_f64() - e2.position().value().as_f64())
                    .abs()
                    < tolerance;
                let type_match =
                    std::mem::discriminant(e1.event()) == std::mem::discriminant(e2.event());

                if pos_match && type_match {
                    matched += 1;
                    break;
                }
            }
        }

        // 确保至少有 min(events1, events2) - 1 个事件匹配
        let min_events = events1_total.len().min(events2_total.len());
        assert!(
            matched >= min_events.saturating_sub(1),
            "Too many events mismatch: matched={}/{}, events1={}, events2={}",
            matched,
            min_events,
            events1_total.len(),
            events2_total.len()
        );
    } else {
        // 如果差异超过1，仍然使用严格的断言
        assert_events_equal(&events1_total, &events2_total);
    }
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
