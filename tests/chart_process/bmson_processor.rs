#![cfg(feature = "bmson")]

use std::time::{Duration, SystemTime};

use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::prelude::*;

#[test]
fn test_bmson_continue_duration_references_bpm_and_stop() {
    // BMSON with init BPM 120, a single key note at y=0.5 measure (480 pulses), c=true,
    // and a stop starting at y=1.0 measure (960 pulses) lasting 240 pulses (0.25 measure).
    // Expected continue duration = time to stop start (0.5 measure) + stop time (0.25 measure)
    // With BPM 120: seconds per measure = 240/120 = 2.0s
    // => 0.5 * 2.0 + 0.25 * 2.0 = 1.5 seconds
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
    let mut processor = BmsonProcessor::new(bmson, base_bpm, Duration::from_millis(600));

    let start_time = SystemTime::now();
    processor.start_play(start_time);

    // Progress slightly so the note at y=0.5 is inside visible window (0.6 measure default)
    let t = start_time + Duration::from_millis(100);
    let _ = processor.update(t);

    // Find the note and assert continue_play duration
    let mut found = false;
    for ev in processor.visible_events(t) {
        if let ChartEvent::Note { continue_play, .. } = ev.event() {
            // Ensure it's our note
            if let Some(dur) = continue_play {
                let secs = dur.as_secs_f64();
                assert!(
                    (secs - 1.5).abs() < 0.02,
                    "continue duration should be ~1.5s, got {:.6}",
                    secs
                );
                found = true;
                break;
            }
        }
    }
    assert!(found, "Expected to find a note with continue duration");
}
