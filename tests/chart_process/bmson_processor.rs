#![cfg(feature = "bmson")]

use std::time::{Duration, SystemTime};

use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::prelude::*;

#[test]
fn test_bmson_continue_duration_references_bpm_and_stop() {
    // BMSON with init BPM 120, a single key note at y=0.5 measure (480 pulses), c=true,
    // and a stop starting at y=1.0 measure (960 pulses) lasting 240 pulses (0.25 measure).
    // 新语义（时间点）：该 note 的 continue 为从上次重启（无）到其 y=0.5 的时间点，Stop 在 1.0 小节，
    // 不在 (0.0, 0.5) 区间内，因此不计入。
    // BPM 120 下每小节 2.0s ⇒ 0.5 * 2.0 = 1.0s
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

    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM");
    // 默认反应时间 600ms 即可覆盖 y=0.5 的音符
    let mut processor = BmsonProcessor::new(bmson, base_bpm, Duration::from_millis(600));

    let start_time = SystemTime::now();
    processor.start_play(start_time);

    // Progress slightly so the note at y=0.5 is inside visible window (0.6 measure default)
    // 稍微推进以确保 y=0.5 进入可见窗口 (默认 0.6 小节)
    let t = start_time + Duration::from_millis(100);
    let _ = processor.update(t);

    // Find the note and assert continue_play duration
    let mut found = false;
    for ev in processor.visible_events(t) {
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
fn test_bmson_continue_duration_with_bpm_scroll_and_stop() {
    // BMSON with init BPM 120, a key note at y=0.25 measure (240 pulses), c=true.
    // BPM changes to 180 at y=0.75 measure (720 pulses).
    // A scroll event occurs at y=1.0 measure (960 pulses) but should not affect time.
    // Stop starts at y=1.25 measure (1200 pulses) with duration 240 pulses (0.25 measure).
    // 新语义（时间点）：note 在 y=0.25，小于 BPM/Stop/Scroll 发生点，因此只计算 0.25 小节时间点
    // BPM 120 下每小节 2.0s ⇒ 0.25 * 2.0 = 0.5s
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

    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM");
    let mut processor = BmsonProcessor::new(bmson, base_bpm, Duration::from_millis(600));

    let start_time = SystemTime::now();
    processor.start_play(start_time);

    // 稍微推进以确保 y=0.25 进入可见窗口 (默认 0.6 小节)
    let t = start_time + Duration::from_millis(100);
    let _ = processor.update(t);

    let mut found = false;
    for ev in processor.visible_events(t) {
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

    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM");
    // Use longer reaction time to include all notes in visible window
    let mut processor = BmsonProcessor::new(bmson, base_bpm, Duration::from_millis(5000));

    let start_time = SystemTime::now();
    processor.start_play(start_time);

    let t = start_time + Duration::from_millis(100);
    let _ = processor.update(t);

    let mut some_count = 0;
    let mut none_count = 0;
    let mut durations = Vec::new();
    for ev in processor.visible_events(t) {
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
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let a = durations[0];
    let b = durations[1];
    assert!(
        (a - 0.25).abs() < 0.02 && (b - 0.50).abs() < 0.02,
        "continue timepoints should be ~0.25s and ~0.50s, got {:.6} and {:.6}",
        a,
        b
    );
}

#[test]
fn test_bmson_continue_accumulates_multiple_stops_between_notes() {
    // 单个 sound_channel，中间存在多个 Stop，验证总时长累加：
    // init BPM 120, res 240；note1 在 0.25 小节 (240 pulses, c=true)，note2 在 1.25 小节 (1200 pulses)
    // Stop1: 0.5 小节处 (480 pulses, 240 pulses)，Stop2: 1.0 小节处 (960 pulses, 240 pulses)
    // 新语义（时间点）：第二个 c=true note 的时间点为从 0.0 -> 1.25 的基础时间 1.25 小节=2.5s，
    // 区间 (0.0, 1.25) 内 Stop 两次，各 0.25 小节=0.5s，总计 2.5 + 0.5 + 0.5 = 3.5s
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

    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM");
    let mut processor = BmsonProcessor::new(bmson, base_bpm, Duration::from_millis(600));

    let start_time = SystemTime::now();
    processor.start_play(start_time);

    // 推进到 800ms，使预加载窗口覆盖 y=1.25 的音符
    let t = start_time + Duration::from_millis(800);
    let _ = processor.update(t);

    let mut found = false;
    for ev in processor.visible_events(t) {
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
    // 两个 sound_channel，彼此的 continue_time 独立：
    // A: 0.25 -> 0.5 小节 (240 -> 480 pulses)，c=true 到下一音符；期望 ~0.5s。
    // B: 0.375 -> 1.0 小节 (360 -> 960 pulses)，c=true 到下一音符；期望 ~1.25s。
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

    let output = parse_bmson(json);
    let bmson = output.bmson.expect("Failed to parse BMSON");

    let base_bpm = StartBpmGenerator
        .generate(&bmson)
        .expect("Failed to generate base BPM");
    // 确保两个 channel 的所有音符都在可见窗口中
    let mut processor = BmsonProcessor::new(bmson, base_bpm, Duration::from_millis(5000));

    let start_time = SystemTime::now();
    processor.start_play(start_time);
    let t = start_time + Duration::from_millis(100);
    let _ = processor.update(t);

    let mut durations = Vec::new();
    let mut none_count = 0;
    for ev in processor.visible_events(t) {
        if let ChartEvent::Note { continue_play, .. } = ev.event() {
            match continue_play {
                Some(d) => durations.push(d.as_secs_f64()),
                None => none_count += 1,
            }
        }
    }

    // 两个 c=false 音符应为 None
    assert_eq!(none_count, 2, "Expected two non-continue notes with None");
    // 两个 c=true 音符应各自有时间点，且不相互影响：存在 ~0.5s 与 ~0.75s 两个值
    assert_eq!(durations.len(), 2, "Expected two continue durations");
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let a = durations[0];
    let b = durations[1];
    assert!(
        (a - 0.5).abs() < 0.02 && (b - 0.75).abs() < 0.02,
        "Expected ~0.5s and ~0.75s timepoints, got {:.6} and {:.6}",
        a,
        b
    );
}
