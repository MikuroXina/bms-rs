use gametime::{TimeSpan, TimeStamp};

use bms_rs::bms::prelude::*;
use strict_num_extended::PositiveF64;

/// Default BPM value (120.0) for tests
const DEFAULT_BPM_120: PositiveF64 = PositiveF64::new_const(120.0);

use bms_rs::chart::prelude::*;
use std::time::Duration;

use super::super::assert_time_close;
use super::parse_bms_no_warnings;

#[test]
fn test_bms_events_in_time_range_returns_note_near_center() {
    let source = r"
#TITLE Time Range Test
#ARTIST Test
#BPM 120
#PLAYER 1
#WAV01 test.wav
#00111:01
";
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or(BaseBpm::new(DEFAULT_BPM_120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = Process::<KeyLayoutBeat>::process(&bms).expect("failed to parse chart");
    let start_time = TimeStamp::start();
    let mut processor = ChartPlayer::start(&chart, visible_range_per_bpm, start_time);
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
fn test_parsed_chart_tracks_have_correct_y_coordinates_and_wav_ids() {
    let bms_source = r"
#WAV01 test1.wav
#WAV02 test2.wav
#WAV03 test3.wav
#WAV04 test4.wav
#00202:0.0
#00211:01
#00212:02
#00213:0103
#00314:04
";

    let config = default_config().prompter(AlwaysUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let chart = Process::<KeyLayoutBeat>::process(&bms).expect("failed to parse chart");

    let note_events: Vec<_> = chart
        .events()
        .as_events()
        .iter()
        .filter_map(|ev| {
            if let ChartEvent::Note { key, wav_id, .. } = ev.event() {
                Some((*ev.position(), *key, *wav_id))
            } else {
                None
            }
        })
        .collect();

    // beatoraja-verified (jbms-parser): #00211/#00314 both at t=4000ms (y=2.0) —
    // track 2 has zero length (#00202:0.0) and takes no room, so track 3 still
    // starts at 2.0; fraction delta = fraction × length (0) → events pile up at y=2.0.
    let y2 = YCoordinate::new(NonNegativeF64::new(2.0).expect("2.0 is valid"));
    let expected_events = vec![
        (y2, Key::Key(1), Some(WavId::new(1))),
        (y2, Key::Key(3), Some(WavId::new(1))),
        (y2, Key::Key(2), Some(WavId::new(2))),
        (y2, Key::Key(3), Some(WavId::new(3))),
        (y2, Key::Key(4), Some(WavId::new(4))),
    ];

    assert_eq!(note_events, expected_events);
}

#[test]
fn test_stop_duration_is_milliseconds_and_freezes_playhead() {
    // `#STOP01 750` = 192nd-note units = 750/48 = 15.625 beats.
    // "0001" (4 chars = 2 fractions) → STOP at fraction 1 (y=0.5), note at y=1.0 (#00111:01).
    // BPM 120: y0→0.5 = 1.0s, STOP = 15.625 beats × 60/120 = 7.8125s, 0.5→1.0 = 1.0s
    // → Note activate = 9.8125s (matches beatoraja stop_ms = 1250×value/bpm: 1250×750/120 = 7812.5ms).
    let source = r"
#TITLE Stop Test
#BPM 120
#PLAYER 1
#WAV01 test.wav
#STOP01 750
#00009:0001
#00111:01
";
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(source, config);
    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or(BaseBpm::new(DEFAULT_BPM_120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = Process::<KeyLayoutBeat>::process(&bms).expect("failed to parse chart");
    let start_time = TimeStamp::start();
    let mut processor = ChartPlayer::start(&chart, visible_range_per_bpm.clone(), start_time);

    // Note activate_time: includes STOP 7.8125s → 9.8125s
    let events = processor.update(start_time + TimeSpan::SECOND * 10);
    let note = events
        .iter()
        .find(|e| matches!(e.event(), ChartEvent::Note { .. }))
        .expect("note event should trigger by 10.0s");
    assert_time_close(
        9.8125,
        note.activate_time().as_secs_f64(),
        "note activate_time with 15.625-beat stop at 120bpm",
    );

    // Playhead freezes: reaches y=0.5 at 1.0s and stays there at 1.1s
    let mut p2 = ChartPlayer::start(&chart, visible_range_per_bpm, start_time);
    p2.update(start_time + TimeSpan::SECOND);
    p2.update(start_time + TimeSpan::SECOND + TimeSpan::MILLISECOND * 100);
    let y_after_1_1s = p2.playback_state().progressed_y.as_f64();
    assert!(
        (y_after_1_1s - 0.5).abs() < 1e-9,
        "playhead should stay frozen at stop y=0.5 during 7.8s stop, got {y_after_1_1s}"
    );
    // 9.8125s: freeze ends, note event fires
    let events2 = p2.update(start_time + TimeSpan::SECOND * 10);
    assert!(
        events2
            .iter()
            .any(|e| matches!(e.event(), ChartEvent::Note { .. })),
        "note should trigger after stop freeze ends"
    );
}

#[test]
fn test_dense_stops_strobe_playhead_progression() {
    // [Clue]Random-style "note flash": back-to-back STOP01=750, one per 1/4 measure
    // ("01010101" = 8 chars = 4 fractions, all 01 → y=0, 0.25, 0.5, 0.75).
    // BPM 120: STOP = 15.625 beats × 60/120 = 7.8125s; 1/4-measure advance = 0.5s.
    //   0.000-7.8125s: frozen at y=0 (start STOP applies immediately)
    //   7.8125-8.3125s: advance 0.5s → y=0.25
    //   8.3125-16.125s: frozen at y=0.25
    //   16.125-16.625s: advance → y=0.5
    //   16.625-24.4375s: frozen at y=0.5
    //   24.4375-24.9375s: advance → y=0.75
    //   24.9375-32.75s: frozen at y=0.75
    //   32.75-33.25s: advance → y=1.0 (note fires)
    // Playhead pausing at STOPs and jumping between them = the "note flash" rhythm.
    let source = r"
#TITLE Strobe Stop Test
#BPM 120
#PLAYER 1
#WAV01 test.wav
#STOP01 750
#00009:01010101
#00111:01
";
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(source, config);
    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or(BaseBpm::new(DEFAULT_BPM_120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = Process::<KeyLayoutBeat>::process(&bms).expect("failed to parse chart");
    let start = TimeStamp::start();
    let mut p = ChartPlayer::start(&chart, visible_range_per_bpm, start);

    let check_y_at = |pl: &mut ChartPlayer<'_>, at_secs: f64, expected_y: f64, label: &str| {
        pl.update(start + TimeSpan::from_duration(Duration::from_secs_f64(at_secs)));
        let y = pl.playback_state().progressed_y.as_f64();
        assert!(
            (y - expected_y).abs() < 1e-6,
            "{label} (t={at_secs}s): expected y={expected_y}, got {y}"
        );
    };

    // 5s: frozen at y=0 (start STOP applies immediately)
    check_y_at(&mut p, 5.0, 0.0, "start stop freeze");
    // 8s: advancing (7.8125 ended) → y=(8.0-7.8125)/0.5*0.25=0.09375
    check_y_at(&mut p, 8.0, 0.09375, "advancing after first stop");
    // 10s: frozen at y=0.25 (8.3125-16.125)
    check_y_at(&mut p, 10.0, 0.25, "second stop freeze");
    // 17s: frozen at y=0.5 (16.625-24.4375)
    check_y_at(&mut p, 17.0, 0.5, "third stop freeze");
    // 25s: frozen at y=0.75 (24.9375-32.75)
    check_y_at(&mut p, 25.0, 0.75, "fourth stop freeze");
    // 33.5s: last freeze ended (32.75), note (y=1.0) already fired
    let events = p.update(start + TimeSpan::from_duration(Duration::from_secs_f64(33.5)));
    assert!(
        events
            .iter()
            .any(|e| matches!(e.event(), ChartEvent::Note { .. })),
        "note should trigger after the last stop freeze ends"
    );
}

#[test]
fn test_track_with_short_section_length_starts_at_measure_position() {
    // #00102:0.125 → track 1 (measure 1) has length 0.125, starting at index 1 (track 0 default 1.0).
    // track 1's STOP/note (#00109/#00111, fraction 1) should land at y = 1.0 + 1/2×0.125 = 1.0625,
    // not squashed to y≈0.06 (which would treat 0.125 as the start and add the fraction directly).
    let source = r"
#TITLE Short Section Test
#BPM 120
#PLAYER 1
#WAV01 test.wav
#STOP01 750
#00102:0.125
#00109:0101
#00111:01
#00011:01
";
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(source, config);
    let chart = Process::<KeyLayoutBeat>::process(&bms).expect("failed to parse chart");
    let events = chart.events().as_events();
    let mut note_y = Vec::new();
    let mut stop_y = None;
    for e in events {
        match e.event() {
            ChartEvent::Note { .. } => note_y.push(e.position().as_f64()),
            ChartEvent::Stop { .. } => stop_y = Some(e.position().as_f64()),
            _ => {}
        }
    }
    note_y.sort_by(f64::total_cmp);
    // #00011 (track 0 fraction 0) → y=0; #00111 (track 1 fraction 0) → y=1.0
    assert!(
        (note_y[0] - 0.0).abs() < 1e-6,
        "track0 note should be at y=0, got {:?}",
        note_y
    );
    assert!(
        (note_y[1] - 1.0).abs() < 1e-6,
        "track1 note should start at y=1.0 (measure position), got {:?}",
        note_y
    );
    // #00109 fraction 1 → y = 1.0 + (1/2)×0.125 = 1.0625
    assert!(
        (stop_y.unwrap() - 1.0625).abs() < 1e-6,
        "track1 stop at position 1/2 of 0.125-length section should be y=1.0625, got {:?}",
        stop_y
    );
}
