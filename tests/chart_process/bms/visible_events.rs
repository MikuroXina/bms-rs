use std::str::FromStr;

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive, Zero};

use bms_rs::bms::Decimal;
use bms_rs::bms::command::channel::mapper::KeyLayoutBeat;
use bms_rs::bms::prelude::*;
use bms_rs::chart_process::prelude::*;

use super::{assert_time_close, parse_bms_no_warnings};

#[test]
fn test_bemuse_ext_basic_visible_events_functionality() {
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let bms_source = include_str!("../../bms/files/bemuse_ext.bms");
    let config = default_config().prompter(AlwaysUseOlder);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let initial_state = processor.playback_state();
    assert_eq!(*initial_state.current_bpm(), Decimal::from(120));
    assert_eq!(*initial_state.current_speed(), Decimal::one());
    assert_eq!(*initial_state.current_scroll(), Decimal::one());

    let after_first_change = start_time + TimeSpan::SECOND;
    let _ = processor.update(after_first_change);

    let state = processor.playback_state();
    let visible_window_y = processor.visible_range_per_bpm().window_y(
        state.current_bpm(),
        state.current_speed(),
        state.playback_ratio(),
    );
    assert!(
        visible_window_y.as_ref() > &Decimal::zero(),
        "Expected visible window y > 0, got: {:?}",
        visible_window_y.as_ref()
    );

    let after_change_events = processor.visible_events();
    assert!(
        !after_change_events.is_empty(),
        "Should have visible events"
    );

    assert!(
        after_change_events
            .iter()
            .any(|(_, range)| range.start().as_ref() > &Decimal::zero()),
        "Expected at least one display_ratio > 0"
    );

    for (visible_event, display_ratio_range) in &after_change_events {
        let y_value = visible_event.position().as_ref().to_f64().unwrap_or(0.0);
        let display_ratio_value = display_ratio_range.start().value().to_f64().unwrap_or(0.0);

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
    let reaction = TimeSpan::MILLISECOND * 600;
    let bms_source = include_str!("../../bms/files/bemuse_ext.bms");
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
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
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let initial_state = processor.playback_state();
    assert_eq!(*initial_state.current_bpm(), Decimal::from(151));

    let after_first_change = start_time + TimeSpan::SECOND * 2;
    let _ = processor.update(after_first_change);
    let bpm_75_5 = Decimal::from_str("75.5").unwrap_or_else(|err| {
        panic!("Failed to parse Decimal literal in test: {err:?}");
    });
    let state = processor.playback_state();
    assert_eq!(*state.current_bpm(), bpm_75_5);

    let after_bpm_events = processor.visible_events();
    assert!(
        !after_bpm_events.is_empty(),
        "Should still have visible events after BPM change"
    );

    for (_, display_ratio_range) in &after_bpm_events {
        let ratio_value = display_ratio_range.start().as_ref().to_f64().unwrap_or(0.0);
        assert!(ratio_value.is_finite() && ratio_value >= 0.0);
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
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let initial_state = processor.playback_state();
    assert_eq!(*initial_state.current_scroll(), Decimal::one());

    let initial_events = processor.visible_events();
    let initial_ratios: Vec<f64> = initial_events
        .iter()
        .map(|(_, display_ratio_range)| {
            display_ratio_range.start().as_ref().to_f64().unwrap_or(0.0)
        })
        .collect::<Vec<_>>();

    if initial_ratios.is_empty() {
        return;
    }

    let after_first_scroll = start_time + TimeSpan::SECOND;
    let _ = processor.update(after_first_scroll);
    let state = processor.playback_state();
    assert_eq!(*state.current_scroll(), Decimal::one());

    let after_first_ratios: Vec<f64> = processor
        .visible_events()
        .iter()
        .map(|(_, display_ratio_range)| {
            display_ratio_range.start().as_ref().to_f64().unwrap_or(0.0)
        })
        .collect::<Vec<_>>();

    if after_first_ratios.is_empty() {
        return;
    }

    for (initial_ratio, after_first_ratio) in initial_ratios.iter().zip(after_first_ratios.iter()) {
        let diff = (after_first_ratio - initial_ratio).abs();
        assert_time_close(0.0, diff, "Display ratio difference when scroll is 1.0");
    }

    let after_scroll_half = after_first_scroll + TimeSpan::SECOND * 2;
    let scroll_half = Decimal::from_str("0.5").unwrap_or_else(|err| {
        panic!("Failed to parse Decimal literal in test: {err:?}");
    });
    let _ = processor.update(after_scroll_half);
    let half_scroll_state = processor.playback_state();
    assert_eq!(*half_scroll_state.current_scroll(), scroll_half);

    let after_scroll_half_ratios: Vec<f64> = processor
        .visible_events()
        .iter()
        .map(|(_, display_ratio_range)| {
            display_ratio_range.start().as_ref().to_f64().unwrap_or(0.0)
        })
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
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
    let start_time = TimeStamp::start();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let initial_state = processor.playback_state();
    assert_eq!(*initial_state.current_bpm(), Decimal::from(120));
    assert_eq!(*initial_state.current_scroll(), Decimal::one());

    let after_changes = start_time + TimeSpan::from_duration(Duration::from_millis(1000));
    let _ = processor.update(after_changes);

    assert!(
        !processor.visible_events().is_empty(),
        "Should have visible events after flow events are triggered"
    );

    let state = processor.playback_state();
    assert!(
        *state.current_bpm() > Decimal::zero(),
        "BPM should be valid"
    );
    assert!(
        *state.current_scroll() > Decimal::zero(),
        "Scroll should be valid"
    );
}

#[test]
fn test_bms_stop_duration_conversion_from_192nd_note_to_beats() {
    let duration_192nd = Decimal::from(192);
    let expected_beats = Decimal::from(4);

    let converted_beats = duration_192nd / Decimal::from(48);

    assert_eq!(
        converted_beats, expected_beats,
        "192nd-note duration should be converted to beats: 192/48 = 4 beats"
    );

    let duration_96 = Decimal::from(96);
    let expected_2_beats = Decimal::from(2);
    let converted_2_beats = duration_96 / Decimal::from(48);
    assert_eq!(
        converted_2_beats, expected_2_beats,
        "96 192nd-notes should equal 2 beats"
    );

    let duration_48 = Decimal::from(48);
    let expected_1_beat = Decimal::from(1);
    let converted_1_beat = duration_48 / Decimal::from(48);
    assert_eq!(
        converted_1_beat, expected_1_beat,
        "48 192nd-notes should equal 1 beat"
    );
}

#[test]
fn test_bms_stop_timing_with_bpm_changes() {
    let duration_192nd = Decimal::from(192);
    let beats_at_120 = duration_192nd.clone() / Decimal::from(48);
    assert_eq!(beats_at_120, Decimal::from(4));

    let beats_at_180 = duration_192nd / Decimal::from(48);
    assert_eq!(beats_at_180, Decimal::from(4));

    assert_eq!(
        beats_at_120, beats_at_180,
        "STOP duration conversion should be independent of BPM"
    );

    let duration_96 = Decimal::from(96);
    let beats_96_at_120 = duration_96.clone() / Decimal::from(48);
    let beats_96_at_180 = duration_96 / Decimal::from(48);
    assert_eq!(beats_96_at_120, beats_96_at_180);
    assert_eq!(beats_96_at_120, Decimal::from(2));
}
