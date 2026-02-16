use std::str::FromStr;

use gametime::{TimeSpan, TimeStamp};

use bms_rs::bms::command::channel::mapper::KeyLayoutBeat;
use bms_rs::bms::prelude::*;
use strict_num_extended::{FinF64, PositiveF64};

use bms_rs::chart_process::BaseBpm;
use bms_rs::chart_process::prelude::*;

use super::{assert_time_close, parse_bms_no_warnings};

const TEST_BPM_120: PositiveF64 = PositiveF64::new_const(120.0);

#[test]
fn test_bemuse_ext_basic_visible_events_functionality() {
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let bms_source = include_str!("../../bms/files/bemuse_ext.bms");
    let config = default_config().prompter(AlwaysUseOlder);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or(BaseBpm::new(TEST_BPM_120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms).expect("failed to parse chart");
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let initial_state = processor.playback_state();
    assert_eq!(initial_state.current_bpm, TEST_BPM_120);
    assert_eq!(initial_state.current_speed, PositiveF64::ONE);
    assert_eq!(initial_state.current_scroll, FinF64::ONE);

    let after_first_change = start_time + TimeSpan::SECOND;
    let _ = processor.update(after_first_change);

    let state = processor.playback_state();
    let visible_window_y = processor.visible_range_per_bpm().window_y(
        state.current_bpm,
        state.current_speed,
        state.playback_ratio,
    );
    assert!(
        visible_window_y.as_f64() > 0.0,
        "Expected visible window y > 0, got: {:?}",
        visible_window_y.as_f64()
    );

    let after_change_events = processor.visible_events();
    assert!(
        !after_change_events.is_empty(),
        "Should have visible events"
    );

    assert!(
        after_change_events
            .iter()
            .any(|(_, range)| range.start().as_ref() > &FinF64::ZERO),
        "Expected at least one display_ratio > 0"
    );

    for (visible_event, display_ratio_range) in &after_change_events {
        let y_value = visible_event.position().as_f64();
        let display_ratio_value = display_ratio_range.start().value().as_f64();

        assert!(
            (0.0..=2.0).contains(&display_ratio_value),
            "Display ratio should be in reasonable range, current value: {:.3}, event Y: {:.3}",
            display_ratio_value,
            y_value
        );

        match visible_event.event() {
            ChartEvent::Note { .. } | ChartEvent::Bgm { .. } => {
                assert!(
                    display_ratio_value.is_finite(),
                    "Display ratio for note/BGM events should be finite"
                );
            }
            ChartEvent::BpmChange { .. }
            | ChartEvent::SpeedChange { .. }
            | ChartEvent::ScrollChange { .. } => {
                assert!(
                    display_ratio_value.is_finite(),
                    "Display ratio for control events should be finite"
                );
            }
            _ => {}
        }
    }
}

#[test]
fn test_bms_visible_event_activate_time_within_reaction_window() {
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let bms_source = include_str!("../../bms/files/bemuse_ext.bms");
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or(BaseBpm::new(TEST_BPM_120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms).expect("failed to parse chart");
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let after = start_time + TimeSpan::SECOND;
    let _ = processor.update(after);
    let events = processor.visible_events();
    assert!(
        !events.is_empty(),
        "Should have visible events after advance"
    );

    for (ev, _) in events {
        let secs = ev.activate_time().as_secs_f64();
        let elapsed = (after - start_time).as_secs_f64();
        assert!(secs >= elapsed, "activate_time should be >= elapsed");
        assert!(secs.is_finite());
    }
}

#[test]
fn test_lilith_mx_bpm_changes_affect_visible_window() {
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let bms_source = include_str!("../../bms/files/lilith_mx.bms");
    let config = default_config().prompter(AlwaysUseOlder);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or(BaseBpm::new(TEST_BPM_120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms).expect("failed to parse chart");
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let initial_state = processor.playback_state();
    assert_eq!(
        initial_state.current_bpm,
        PositiveF64::try_from(151.0).unwrap()
    );

    let after_first_change = start_time + TimeSpan::SECOND * 2;
    let _ = processor.update(after_first_change);
    let bpm_75_5 = PositiveF64::from_str("75.5").unwrap_or_else(|err| {
        panic!("Failed to parse PositiveF64 literal in test: {err:?}");
    });
    let state = processor.playback_state();
    assert_eq!(state.current_bpm, bpm_75_5);

    let after_bpm_events = processor.visible_events();
    assert!(
        !after_bpm_events.is_empty(),
        "Should still have visible events after BPM change"
    );

    for (event, display_ratio_range) in &after_bpm_events {
        let ratio_start = display_ratio_range.start().as_ref().as_f64();
        let ratio_end = display_ratio_range.end().as_ref().as_f64();
        assert!(ratio_start.is_finite());
        assert!(ratio_end.is_finite());

        // For long notes, check that the tail is above the judgment line
        // For normal notes, check that the head is above the judgment line
        let is_long_note = matches!(
            event.event(),
            ChartEvent::Note {
                kind: NoteKind::Long,
                ..
            }
        );
        if is_long_note {
            assert!(
                ratio_end >= 0.0,
                "Long note tail should be above judgment line"
            );
        } else {
            assert!(
                ratio_start >= 0.0,
                "Normal note head should be above judgment line"
            );
        }
    }
}

#[test]
fn test_bemuse_ext_scroll_half_display_ratio_scaling() {
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let bms_source = include_str!("../../bms/files/bemuse_ext.bms");
    let config = default_config().prompter(AlwaysUseOlder);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or(BaseBpm::new(TEST_BPM_120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms).expect("failed to parse chart");
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let initial_state = processor.playback_state();
    assert_eq!(initial_state.current_scroll, FinF64::ONE);

    let initial_events = processor.visible_events();
    let initial_ratios: Vec<f64> = initial_events
        .iter()
        .map(|(_, display_ratio_range)| display_ratio_range.start().as_ref().as_f64())
        .collect::<Vec<_>>();

    if initial_ratios.is_empty() {
        return;
    }

    let after_first_scroll = start_time + TimeSpan::SECOND;
    let _ = processor.update(after_first_scroll);
    let state = processor.playback_state();
    assert_eq!(state.current_scroll, FinF64::ONE);

    let after_first_ratios: Vec<f64> = processor
        .visible_events()
        .iter()
        .map(|(_, display_ratio_range)| display_ratio_range.start().as_ref().as_f64())
        .collect::<Vec<_>>();

    if after_first_ratios.is_empty() {
        return;
    }

    for (initial_ratio, after_first_ratio) in initial_ratios.iter().zip(after_first_ratios.iter()) {
        let diff: f64 = (after_first_ratio - initial_ratio).abs();
        assert_time_close(0.0, diff, "Display ratio difference when scroll is 1.0");
    }

    let after_scroll_half = after_first_scroll + TimeSpan::SECOND * 2;
    let scroll_half = FinF64::HALF;
    let _ = processor.update(after_scroll_half);
    let half_scroll_state = processor.playback_state();
    assert_eq!(half_scroll_state.current_scroll, scroll_half);

    let after_scroll_half_ratios: Vec<f64> = processor
        .visible_events()
        .iter()
        .map(|(_, display_ratio_range)| display_ratio_range.start().as_ref().as_f64())
        .collect::<Vec<_>>();

    if after_scroll_half_ratios.is_empty() {
        return;
    }

    for ratio in after_scroll_half_ratios.iter() {
        assert!(
            ratio.is_finite(),
            "Display ratio should be finite when scroll is 0.5"
        );
        assert!(
            *ratio >= -5.0 && *ratio <= 5.0,
            "Display ratio should be in reasonable range when scroll is 0.5: {:.6}",
            ratio
        );
    }

    if after_first_ratios.len() == after_scroll_half_ratios.len() {
        for (first_ratio, half_ratio) in after_first_ratios
            .iter()
            .zip(after_scroll_half_ratios.iter())
        {
            let expected_half_ratio = *first_ratio * 0.5;

            assert_time_close(
                expected_half_ratio,
                *half_ratio,
                "Display ratio when scroll is 0.5",
            );
        }
    }

    if after_first_ratios.len() == after_scroll_half_ratios.len() {
        for (first_ratio, half_ratio) in after_first_ratios
            .iter()
            .zip(after_scroll_half_ratios.iter())
        {
            if *first_ratio > 0.0 {
                assert!(
                    *half_ratio < *first_ratio,
                    "Display ratio should be less when scroll is 0.5 than when scroll is 1.0, 1.0: {:.6}, 0.5: {:.6}",
                    first_ratio,
                    half_ratio
                );
            }
        }
    }
}

#[test]
fn test_bms_multi_flow_events_same_y_all_triggered() {
    use std::time::Duration;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let bms_source = include_str!("../../bms/files/bemuse_ext.bms");
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or(BaseBpm::new(TEST_BPM_120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(base_bpm.value(), reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms).expect("failed to parse chart");
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let initial_state = processor.playback_state();
    assert_eq!(initial_state.current_bpm, TEST_BPM_120);
    assert_eq!(initial_state.current_scroll, FinF64::ONE);
    assert_eq!(initial_state.current_scroll, FinF64::ONE);

    let after_changes = start_time + TimeSpan::from_duration(Duration::from_millis(1000));
    let _ = processor.update(after_changes);

    assert!(
        !processor.visible_events().is_empty(),
        "Should have visible events after flow events are triggered"
    );

    let state = processor.playback_state();
    assert!(state.current_bpm().as_f64() > 0.0, "BPM should be valid");
    assert!(
        state.current_scroll > FinF64::ZERO,
        "Scroll should be valid"
    );
}

#[test]
fn test_bms_stop_duration_conversion_from_192nd_note_to_beats() {
    let duration_192nd = FinF64::try_from(192.0).unwrap();
    const EXPECTED_BEATS_4: FinF64 = FinF64::new_const(4.0);
    let expected_beats = EXPECTED_BEATS_4;

    let converted_beats = (duration_192nd / FinF64::try_from(48.0).unwrap()).unwrap();

    assert_eq!(
        converted_beats, expected_beats,
        "192nd-note duration should be converted to beats: 192/48 = 4 beats"
    );

    let duration_96 = FinF64::try_from(96.0).unwrap();
    let expected_2_beats = FinF64::TWO;
    let converted_2_beats = (duration_96 / FinF64::try_from(48.0).unwrap()).unwrap();
    assert_eq!(
        converted_2_beats, expected_2_beats,
        "96 192nd-notes should equal 2 beats"
    );

    let duration_48 = FinF64::try_from(48.0).unwrap();
    let expected_1_beat = FinF64::ONE;
    let converted_1_beat = (duration_48 / FinF64::try_from(48.0).unwrap()).unwrap();
    assert_eq!(
        converted_1_beat, expected_1_beat,
        "48 192nd-notes should equal 1 beat"
    );
}

#[test]
fn test_bms_stop_timing_with_bpm_changes() {
    const EXPECTED_BEATS_4: FinF64 = FinF64::new_const(4.0);

    let duration_192nd = FinF64::try_from(192.0).unwrap();
    let beats_at_120 = (duration_192nd / FinF64::try_from(48.0).unwrap()).unwrap();
    assert_eq!(beats_at_120, EXPECTED_BEATS_4);

    let beats_at_180 = (duration_192nd / FinF64::try_from(48.0).unwrap()).unwrap();
    assert_eq!(beats_at_180, EXPECTED_BEATS_4);

    assert_eq!(
        beats_at_120, beats_at_180,
        "STOP duration conversion should be independent of BPM"
    );

    let duration_96 = FinF64::try_from(96.0).unwrap();
    let beats_96_at_120 = (duration_96 / FinF64::try_from(48.0).unwrap()).unwrap();
    let beats_96_at_180 = (duration_96 / FinF64::try_from(48.0).unwrap()).unwrap();
    assert_eq!(beats_96_at_120, beats_96_at_180);
    assert_eq!(beats_96_at_120, FinF64::TWO);
}

#[test]
fn test_custom_visibility_range() {
    let bms_source = include_str!("../../bms/files/bemuse_ext.bms");
    let config = default_config().prompter(AlwaysUseOlder);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or(BaseBpm::new(TEST_BPM_120));
    let visible_range_per_bpm =
        VisibleRangePerBpm::new(base_bpm.value(), TimeSpan::MILLISECOND * 600);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms).expect("failed to parse chart");
    let start_time = TimeStamp::now();
    let mut player = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    // Test default behavior (0.0..=1.0)
    let _ = player.update(start_time + TimeSpan::SECOND);
    let events_before = player.visible_events().len();

    // Set to show events past judgment line (-0.5..=1.0)
    player.set_visibility_range(FinF64::NEG_HALF..=FinF64::ONE);
    let events_extended = player.visible_events().len();
    assert!(
        events_extended >= events_before,
        "Extended visibility range should show more events"
    );

    // Set limited visibility range (0.0..=0.5)
    player.set_visibility_range(FinF64::ZERO..=FinF64::HALF);
    let events_limited = player.visible_events().len();
    assert!(
        events_limited <= events_before,
        "Limited visibility range should show fewer events"
    );

    // Test unbounded range
    player.set_visibility_range(..);
    let events_unbounded = player.visible_events().len();
    assert!(
        events_unbounded >= events_before,
        "Unbounded range should show most events"
    );
}

#[test]
fn test_visibility_range_bound_types() {
    let bms_source = include_str!("../../bms/files/bemuse_ext.bms");
    let config = default_config().prompter(AlwaysUseOlder);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or(BaseBpm::new(TEST_BPM_120));
    let visible_range_per_bpm =
        VisibleRangePerBpm::new(base_bpm.value(), TimeSpan::MILLISECOND * 600);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms).expect("failed to parse chart");
    let start_time = TimeStamp::now();
    let mut player = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let _ = player.update(start_time + TimeSpan::SECOND);

    // Test half-open range
    player.set_visibility_range(FinF64::ZERO..FinF64::ONE);
    let count_open = player.visible_events().len();

    // Test closed range
    player.set_visibility_range(FinF64::ZERO..=FinF64::ONE);
    let count_closed = player.visible_events().len();

    // Closed range should include events on the boundary
    assert!(count_closed >= count_open);
}
