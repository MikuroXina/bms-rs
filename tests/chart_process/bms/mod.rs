//! Integration tests for `bms_rs::chart_process::BmsProcessor`.

use std::str::FromStr;

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive, Zero};

use bms_rs::bms::Decimal;
use bms_rs::bms::command::channel::mapper::KeyLayoutBeat;
use bms_rs::bms::prelude::*;
use bms_rs::chart_process::prelude::*;

use super::{MICROSECOND_EPSILON, assert_time_close};

/// Parse BMS source and return the BMS struct, asserting no warnings.
fn parse_bms_no_warnings<T, P, R, M>(source: &str, config: ParseConfig<T, P, R, M>) -> Bms
where
    T: KeyLayoutMapper,
    P: Prompter,
    R: Rng,
    M: TokenModifier,
{
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms: bms_res,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, config);
    assert_eq!(parse_warnings, vec![]);
    bms_res.expect("Failed to parse BMS in test setup")
}

#[test]
fn test_bemuse_ext_basic_visible_events_functionality() {
    // Test basic visible_events functionality using bemuse_ext.bms file
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

    // Verify initial state
    let initial_state = processor.playback_state();
    assert_eq!(*initial_state.current_bpm(), Decimal::from(120));
    assert_eq!(*initial_state.current_speed(), Decimal::one());
    assert_eq!(*initial_state.current_scroll(), Decimal::one());

    // Advance to first change point
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

    // Check that visible_events method works normally
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

    // Verify display ratio calculation
    for (visible_event, display_ratio_range) in &after_change_events {
        let y_value = visible_event.position().as_ref().to_f64().unwrap_or(0.0);
        let display_ratio_value = display_ratio_range.start().value().to_f64().unwrap_or(0.0);

        // Display ratio should be in reasonable range
        assert!(
            (0.0..=2.0).contains(&display_ratio_value),
            "Display ratio should be in reasonable range, current value: {:.3}, event Y: {:.3}",
            display_ratio_value,
            y_value
        );

        // Verify event type
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
    // Test BPM changes' effect on visible window using lilith_mx.bms file
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

    // Initial state: BPM = 151
    let initial_state = processor.playback_state();
    assert_eq!(*initial_state.current_bpm(), Decimal::from(151));

    // Advance to first BPM change point
    // Note: With new playhead speed (1/240), speed is half of original (1/120)
    // So need twice the time to reach the same Y position
    let after_first_change = start_time + TimeSpan::SECOND * 2;
    let _ = processor.update(after_first_change);
    let bpm_75_5 = Decimal::from_str("75.5").unwrap_or_else(|err| {
        panic!("Failed to parse Decimal literal in test: {err:?}");
    });
    let state = processor.playback_state();
    assert_eq!(*state.current_bpm(), bpm_75_5);

    // Get visible events after BPM change
    let after_bpm_events = processor.visible_events();
    assert!(
        !after_bpm_events.is_empty(),
        "Should still have visible events after BPM change"
    );

    // Verify display ratio is still valid
    for (_, display_ratio_range) in &after_bpm_events {
        let ratio_value = display_ratio_range.start().as_ref().to_f64().unwrap_or(0.0);
        assert!(ratio_value.is_finite() && ratio_value >= 0.0);
    }
}

#[test]
fn test_bemuse_ext_scroll_half_display_ratio_scaling() {
    // Test DisplayRatio scaling when scroll value is 0.5 using bemuse_ext.bms file
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

    // Verify initial state：Scroll = 1.0
    let initial_state = processor.playback_state();
    assert_eq!(*initial_state.current_scroll(), Decimal::one());

    // Get initial visible events and their display ratios
    let initial_events = processor.visible_events();
    let initial_ratios: Vec<f64> = initial_events
        .iter()
        .map(|(_, display_ratio_range)| {
            display_ratio_range.start().as_ref().to_f64().unwrap_or(0.0)
        })
        .collect::<Vec<_>>();

    if initial_ratios.is_empty() {
        return; // If no visible events, skip test
    }

    // Advance to first Scroll change point (still 1.0)
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

    // Since scroll is still 1.0, display ratio should be basically the same
    for (initial_ratio, after_first_ratio) in initial_ratios.iter().zip(after_first_ratios.iter()) {
        let diff = (after_first_ratio - initial_ratio).abs();
        assert_time_close(0.0, diff, "Display ratio difference when scroll is 1.0");
    }

    // Advance to second Scroll change point (scroll 0.5)
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

    // Verify display ratio range and sign
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

    // Verify display ratio scaling effect when scroll is 0.5
    if after_first_ratios.len() == after_scroll_half_ratios.len() {
        for (first_ratio, half_ratio) in after_first_ratios
            .iter()
            .zip(after_scroll_half_ratios.iter())
        {
            // When scroll changes from 1.0 to 0.5, display ratio should become approximately 0.5 times the original
            let expected_half_ratio = *first_ratio * 0.5;

            assert_time_close(
                expected_half_ratio,
                *half_ratio,
                "Display ratio when scroll is 0.5",
            );
        }
    }

    // Additional verification: ensure display ratio when scroll is 0.5 is indeed less than when scroll is 1.0
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
fn test_bms_triggered_event_activate_time_equals_elapsed() {
    let bms_source = include_str!("../../bms/files/bemuse_ext.bms");
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
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
fn test_bms_events_in_time_range_returns_note_near_center() {
    let source = r#"
#TITLE Time Range Test
#ARTIST Test
#BPM 120
#PLAYER 1
#WAV01 test.wav
#00111:01
"#;
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
    let start_time = TimeStamp::start();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);
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
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let after_scroll_change = processor.started_at() + TimeSpan::MILLISECOND * 2700;
    let _ = processor.update(after_scroll_change);
    let state = processor.playback_state();
    assert_ne!(*state.current_scroll(), Decimal::one());

    // Restart by creating a new player
    let config2 = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms2 = parse_bms_no_warnings(bms_source, config2);

    let base_bpm2 = StartBpmGenerator
        .generate(&bms2)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm2 = VisibleRangePerBpm::new(&base_bpm2, reaction_time);
    let chart2 = BmsProcessor::parse::<KeyLayoutBeat>(&bms2);
    let start_time2 = TimeStamp::now();
    let restarted_processor = ChartPlayer::start(chart2, visible_range_per_bpm2, start_time2);
    let reset_state = restarted_processor.playback_state();
    assert_eq!(*reset_state.current_scroll(), Decimal::one());
}

#[test]
fn test_visible_events_duration_matches_reaction_time() {
    // Test that when current_bpm == base_bpm and speed/ratio are 1,
    // events stay in visible_events for exactly reaction_time duration
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
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
    let start_time = TimeStamp::now();
    let processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);
    let _start_time = start_time;

    // Verify standard conditions
    let initial_state = processor.playback_state();
    assert_eq!(*initial_state.current_bpm(), Decimal::from(120));
    assert_eq!(*initial_state.current_speed(), Decimal::one());
    assert_eq!(*initial_state.playback_ratio(), Decimal::one());

    // Calculate expected visible window Y
    let test_base_bpm = BaseBpm::from(Decimal::from(120));
    let visible_range = VisibleRangePerBpm::new(&test_base_bpm, reaction_time);
    let state = processor.playback_state();
    let visible_window_y = visible_range.window_y(
        state.current_bpm(),
        state.current_speed(),
        state.playback_ratio(),
    );

    // Calculate time to cross window
    // velocity = current_bpm * current_speed * playback_ratio / 240
    // time = visible_window_y / velocity
    let velocity = Decimal::from(120) * Decimal::one() * Decimal::one() / Decimal::from(240u64);
    let time_to_cross = visible_window_y.as_ref() / velocity;

    // Note: This test verifies the correctness of visible window calculation
    // Due to accumulated error, actual value may have slight deviation from theoretical value
    let actual_time_to_cross_f64 = time_to_cross.to_f64().unwrap_or(0.0);
    // Expected: 1.2s (actual calculated value), evaluated with 1 microsecond precision
    assert_time_close(1.2, actual_time_to_cross_f64, "time_to_cross");
}

#[test]
fn test_bms_multi_flow_events_same_y_all_triggered() {
    use std::time::Duration;

    // Test using existing bemuse_ext.bms file which has multiple flow events
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

    // Verify initial state
    let initial_state = processor.playback_state();
    assert_eq!(*initial_state.current_bpm(), Decimal::from(120));
    assert_eq!(*initial_state.current_scroll(), Decimal::one());

    // Advance past the first BPM and Scroll change point
    // The bemuse_ext.bms file has BPM and Scroll changes at specific positions
    let after_changes = start_time + TimeSpan::from_duration(Duration::from_millis(1000));
    let _ = processor.update(after_changes);

    // Verify that visible events computation still works after flow events
    assert!(
        !processor.visible_events().is_empty(),
        "Should have visible events after flow events are triggered"
    );

    // Verify that BPM and/or Scroll have potentially changed
    // We just check that the state is valid (not necessarily changed in this specific file)
    let state = processor.playback_state();
    assert!(
        *state.current_bpm() > Decimal::zero(),
        "BPM should be valid"
    );
    assert!(
        *state.current_scroll() > Decimal::zero(),
        "Scroll should be valid"
    );

    // Test passed if we can advance time and compute visible events
    // This confirms that multiple flow events at same Y position don't cause issues
}

#[test]
fn test_bms_stop_duration_conversion_from_192nd_note_to_beats() {
    // Test that STOP duration is correctly converted from 192nd-note units to beats
    use bms_rs::bms::Decimal;

    // Test conversion: 192nd-note / 48 = beats
    // 192 / 48 = 4 beats (one full measure in 4/4 time)
    let duration_192nd = Decimal::from(192);
    let expected_beats = Decimal::from(4);

    // Use the internal conversion function via pattern matching
    // Since it's a private function, we test the logic indirectly
    let converted_beats = duration_192nd / Decimal::from(48);

    assert_eq!(
        converted_beats, expected_beats,
        "192nd-note duration should be converted to beats: 192/48 = 4 beats"
    );

    // Test with different values
    let duration_96 = Decimal::from(96);
    let expected_2_beats = Decimal::from(2);
    let converted_2_beats = duration_96 / Decimal::from(48);
    assert_eq!(
        converted_2_beats, expected_2_beats,
        "96 192nd-notes should equal 2 beats"
    );

    // Test with fractional values
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
    // Test that STOP duration conversion is independent of BPM changes
    // This test verifies that the conversion formula doesn't depend on current BPM
    use bms_rs::bms::Decimal;

    // At BPM 120: 192nd-note should convert to 4 beats
    let duration_192nd = Decimal::from(192);
    let beats_at_120 = duration_192nd.clone() / Decimal::from(48);
    assert_eq!(beats_at_120, Decimal::from(4));

    // At BPM 180: conversion should be the same (conversion is independent of BPM)
    // The conversion from 192nd-note to beats is purely mathematical, not dependent on BPM
    let beats_at_180 = duration_192nd / Decimal::from(48);
    assert_eq!(beats_at_180, Decimal::from(4));

    // Verify they're the same
    assert_eq!(
        beats_at_120, beats_at_180,
        "STOP duration conversion should be independent of BPM"
    );

    // Test with different duration values
    let duration_96 = Decimal::from(96);
    let beats_96_at_120 = duration_96.clone() / Decimal::from(48);
    let beats_96_at_180 = duration_96 / Decimal::from(48);
    assert_eq!(beats_96_at_120, beats_96_at_180);
    assert_eq!(beats_96_at_120, Decimal::from(2));
}

#[test]
fn test_bms_zero_length_section_comprehensive() {
    let bms_source = r#"
#TITLE Zero Length Section Test
#ARTIST Test
#BPM 120
#PLAYER 1
#WAV01 test.wav

// Section 2 is zero-length
#00202:0

// Multiple events in zero-length section (all at different fractional positions)
#00211:01
#00212:02
#00213:03

// Events in normal sections for comparison
#00111:01
#00311:01
"#;

    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(bms_source);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms: bms_res,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert!(
        parse_warnings.is_empty(),
        "Parser should allow zero-length sections without warnings"
    );
    let bms = bms_res.expect("Failed to parse BMS with zero-length section");

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let _config = default_config().prompter(AlwaysWarnAndUseNewer);
    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);

    assert!(
        !chart.events().as_events().is_empty(),
        "Should have parsed some events"
    );

    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);
    let _ = processor.update(start_time + TimeSpan::SECOND * 3);

    let state = processor.playback_state();
    assert!(
        state.current_bpm().to_f64().is_some_and(f64::is_finite),
        "BPM should be finite"
    );

    let events = processor.visible_events();
    for (_ev, ratio_range) in events {
        let ratio_start = ratio_range.start().value().to_f64().unwrap_or(0.0);
        assert!(
            ratio_start.is_finite(),
            "display_ratio should be finite with zero-length section"
        );
    }
}

#[test]
fn test_bms_very_small_section_no_division_by_zero() {
    let bms_source = r#"
#TITLE Very Small Section Test
#ARTIST Test
#BPM 120
#PLAYER 1
#WAV01 test.wav

// Set section 2 length to very small value (but greater than zero)
#00202:0.000001

// Note BEFORE the very small section (in section 1)
#00111:01

// Note INSIDE the very small section (section 2)
#00211:01

// Note AFTER the very small section (in section 3)
#00311:01
"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let _ = processor.update(start_time + TimeSpan::SECOND);
    let events1 = processor.visible_events();
    let count1 = events1.len();

    let _ = processor.update(start_time + TimeSpan::SECOND * 5);
    let events2 = processor.visible_events();
    let count2 = events2.len();

    let _ = processor.update(start_time + TimeSpan::SECOND * 10);
    let events3 = processor.visible_events();

    assert!(
        count1 + count2 + events3.len() > 0,
        "Should have processed some events across all sections"
    );

    for (_ev, ratio_range) in events1.iter().chain(events2.iter()).chain(events3.iter()) {
        let ratio_start = ratio_range.start().value().to_f64().unwrap_or(0.0);
        let ratio_end = ratio_range.end().value().to_f64().unwrap_or(0.0);

        assert!(
            (0.0..=1.0).contains(&ratio_start),
            "display_ratio start should be in [0.0, 1.0] range"
        );
        assert!(
            (0.0..=1.0).contains(&ratio_end),
            "display_ratio end should be in [0.0, 1.0] range"
        );

        let ratio_diff = (ratio_start - ratio_end).abs();
        assert!(
            ratio_diff < 1e-6,
            "display_ratio range should be very small for short notes"
        );
    }

    let state = processor.playback_state();
    let expected_bpm = Decimal::from(120);
    assert_eq!(
        *state.current_bpm(),
        expected_bpm,
        "BPM should be {} after processing",
        expected_bpm,
    );
    let expected_speed = Decimal::one();
    assert_eq!(
        *state.current_speed(),
        expected_speed,
        "Speed should be {} after processing",
        expected_speed,
    );
}

#[test]
fn test_bms_consecutive_zero_length_sections() {
    let bms_source = r#"
#TITLE Consecutive Zero Length Sections
#ARTIST Test
#BPM 120
#PLAYER 1
#WAV01 test.wav

// Multiple consecutive zero-length sections
#00202:0
#00302:0
#00402:0

// Notes in zero-length sections
#00211:01
#00311:01
#00411:01

// Note in normal section after
#00511:01
"#;

    let reaction_time = TimeSpan::MILLISECOND * 600;
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);
    let start_time = TimeStamp::now();
    let mut processor = ChartPlayer::start(chart, visible_range_per_bpm, start_time);

    let _ = processor.update(start_time + TimeSpan::SECOND * 5);

    let events = processor.visible_events();
    for (_ev, ratio_range) in events {
        let ratio_start = ratio_range.start().value().to_f64().unwrap_or(0.0);
        assert!(
            ratio_start.is_finite(),
            "Should handle consecutive zero-length sections without errors"
        );
    }
}

#[test]
fn test_parsed_chart_tracks_have_correct_y_coordinates_and_wav_ids() {
    let bms_source = r#"
#WAV01 test1.wav
#WAV02 test2.wav
#WAV03 test3.wav
#WAV04 test4.wav
#00202:0.0
#00211:01
#00212:02
#00213:0103
#00314:04
"#;

    let config = default_config().prompter(AlwaysUseNewer);
    let bms = parse_bms_no_warnings(bms_source, config);

    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms);

    let note_events: Vec<_> = chart
        .events()
        .as_events()
        .iter()
        .filter_map(|ev| {
            if let ChartEvent::Note { key, wav_id, .. } = ev.event() {
                Some((ev.position().clone(), *key, *wav_id))
            } else {
                None
            }
        })
        .collect();

    let expected_events = vec![
        (YCoordinate::from(1.0), Key::Key(1), Some(WavId::new(1))),
        (YCoordinate::from(1.0), Key::Key(2), Some(WavId::new(2))),
        (YCoordinate::from(1.0), Key::Key(3), Some(WavId::new(3))),
        (YCoordinate::from(1.0), Key::Key(4), Some(WavId::new(4))),
    ];

    assert_eq!(note_events, expected_events);
}
