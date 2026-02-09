use gametime::{TimeSpan, TimeStamp};

use bms_rs::bms::command::channel::mapper::KeyLayoutBeat;
use bms_rs::bms::prelude::*;
use strict_num_extended::{FinF64, PositiveF64};

use bms_rs::chart_process::prelude::*;

use super::{MICROSECOND_EPSILON, assert_time_close, parse_bms_no_warnings};

#[test]
fn test_bms_triggered_event_activate_time_equals_elapsed() {
    let bms_source = include_str!("../../bms/files/bemuse_ext.bms");
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| PositiveF64::try_from(120.0).unwrap());
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms).expect("failed to parse chart");
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let elapsed = TimeSpan::SECOND * 3;
    let now = start_time + elapsed;
    let events = processor.update(now);
    assert!(
        !events.is_empty(),
        "Expected triggered events after {:?} elapsed",
        elapsed
    );

    for evp in events {
        let secs_actual = evp.activate_time().as_secs_f64();
        assert!(
            secs_actual <= elapsed.as_secs_f64() + MICROSECOND_EPSILON,
            "triggered event activate_time should be <= elapsed + 1μs, got {:.6} > {:.6}",
            secs_actual,
            elapsed.as_secs_f64()
        );
        assert!(secs_actual >= 0.0);
    }
}

#[test]
fn test_bms_restart_resets_scroll_to_one() {
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let bms_source = r#"
#TITLE Scroll Reset Test
#ARTIST Test
#BPM 120
#PLAYER 1

#SCROLL01 1.0
#SCROLL02 1.5

#001SC:00020000
#00111:00000000
"#;
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| PositiveF64::try_from(120.0).unwrap());
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms).expect("failed to parse chart");
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let after_scroll_change = processor.started_at() + TimeSpan::MILLISECOND * 2700;
    let _ = processor.update(after_scroll_change);
    let state = processor.playback_state();
    assert_ne!(*state.current_scroll(), FinF64::try_from(1.0).unwrap());

    let config2 = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms2 = parse_bms_no_warnings(bms_source, config2);

    let base_bpm2 = StartBpmGenerator
        .generate(&bms2)
        .unwrap_or_else(|| PositiveF64::try_from(120.0).unwrap());
    let visible_range_per_bpm2 = VisibleRangePerBpm::new(&base_bpm2, reaction_time);
    let chart2 = BmsProcessor::parse::<KeyLayoutBeat>(&bms2).expect("failed to parse chart");
    let start_time2 = TimeStamp::now();
    let restarted_processor = ChartPlayer::start(chart2, visible_range_per_bpm2, start_time2);
    let reset_state = restarted_processor.playback_state();
    assert_eq!(
        *reset_state.current_scroll(),
        FinF64::try_from(1.0).unwrap()
    );
}

#[test]
fn test_visible_events_duration_matches_reaction_time() {
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let bms_source = r#"
#TITLE Reaction Time Test
#ARTIST Test
#BPM 120
#PLAYER 1

#00111:00000001
"#;
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| PositiveF64::try_from(120.0).unwrap());
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms).expect("failed to parse chart");
    let start_time = TimeStamp::now();
    let processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);
    let _start_time = start_time;

    let initial_state = processor.playback_state();
    assert_eq!(
        *initial_state.current_bpm(),
        PositiveF64::try_from(120.0).unwrap()
    );
    assert_eq!(
        *initial_state.current_speed(),
        PositiveF64::try_from(1.0).unwrap()
    );
    assert_eq!(
        *initial_state.playback_ratio(),
        FinF64::try_from(1.0).unwrap()
    );

    let test_base_bpm = PositiveF64::try_from(120.0).unwrap();
    let visible_range = VisibleRangePerBpm::new(&test_base_bpm, reaction_time);
    let state = processor.playback_state();
    let visible_window_y = visible_range.window_y(
        state.current_bpm(),
        state.current_speed(),
        state.playback_ratio(),
    );

    let velocity = (FinF64::try_from(120.0).unwrap()
        * FinF64::try_from(1.0).unwrap()
        * FinF64::try_from(1.0).unwrap()
        / FinF64::try_from((240) as f64).unwrap())
    .unwrap();
    let time_to_cross = visible_window_y.as_f64() / velocity.as_f64();

    let actual_time_to_cross_f64 = time_to_cross;
    assert_time_close(0.6, actual_time_to_cross_f64, "time_to_cross");
}
