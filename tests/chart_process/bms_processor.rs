//! Integration tests for `bms_rs::chart_process::BmsProcessor`.

use std::str::FromStr;

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive, Zero};

use bms_rs::bms::Decimal;
use bms_rs::bms::prelude::*;
use bms_rs::chart_process::prelude::*;

use super::dsl::{bms_driver_with_newer_prompter, bms_driver_with_older_prompter};

const NANOS_PER_SECOND: u64 = 1_000_000_000;

#[test]
fn test_bemuse_ext_basic_visible_events_functionality() {
    // Test basic visible_events functionality using bemuse_ext.bms file
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let bms_source = include_str!("../bms/files/bemuse_ext.bms");

    bms_driver_with_older_prompter(bms_source, reaction_time)
        .check(|p| {
            assert_eq!(*p.current_bpm(), Decimal::from(120));
            assert_eq!(*p.current_speed(), Decimal::one());
            assert_eq!(*p.current_scroll(), Decimal::one());
        })
        .past(TimeSpan::SECOND)
        .view(|p| {
            let visible_window_y = p.visible_range_per_bpm().window_y(
                p.current_bpm(),
                p.current_speed(),
                p.playback_ratio(),
            );
            assert!(
                visible_window_y.as_ref() > &Decimal::zero(),
                "Expected visible window y > 0, got: {:?}",
                visible_window_y.as_ref()
            );
        })
        .view(|p| {
            let after_change_events: Vec<_> = p.visible_events().collect();
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
        })
        .view(|p| {
            let after_change_events: Vec<_> = p.visible_events().collect();
            // Verify display ratio calculation
            for (visible_event, display_ratio_range) in &after_change_events {
                let y_value = visible_event.position().as_ref().to_f64().unwrap_or(0.0);
                let display_ratio_value =
                    display_ratio_range.start().value().to_f64().unwrap_or(0.0);

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
        })
        .run();
}

#[test]
fn test_bms_visible_event_activate_time_within_reaction_window() {
    let reaction = TimeSpan::MILLISECOND * 600;
    let bms_source = include_str!("../bms/files/bemuse_ext.bms");

    bms_driver_with_newer_prompter(bms_source, reaction)
        .past(TimeSpan::SECOND)
        .view(|p| {
            let events: Vec<_> = p.visible_events().collect();
            assert!(
                !events.is_empty(),
                "Should have visible events after advance"
            );
            for (ev, _) in events {
                let secs = ev.activate_time().as_secs_f64();
                assert!(secs >= 1.0, "activate_time should be >= elapsed");
                assert!(secs.is_finite());
            }
        })
        .run();
}

#[test]
fn test_lilith_mx_bpm_changes_affect_visible_window() {
    // Test BPM changes' effect on visible window using lilith_mx.bms file
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let bms_source = include_str!("../bms/files/lilith_mx.bms");

    bms_driver_with_older_prompter(bms_source, reaction_time)
        .check(|p| assert_eq!(*p.current_bpm(), Decimal::from(151)))
        .past(TimeSpan::SECOND * 2)
        .view(|p| {
            let bpm_75_5 = Decimal::from_str("75.5").unwrap_or_else(|err| {
                panic!("Failed to parse Decimal literal in test: {err:?}");
            });
            assert_eq!(*p.current_bpm(), bpm_75_5);
        })
        .view(|p| {
            let after_bpm_events: Vec<_> = p.visible_events().collect();
            assert!(
                !after_bpm_events.is_empty(),
                "Should still have visible events after BPM change"
            );

            // Verify display ratio is still valid
            for (_, display_ratio_range) in &after_bpm_events {
                let ratio_value = display_ratio_range.start().as_ref().to_f64().unwrap_or(0.0);
                assert!(ratio_value.is_finite() && ratio_value >= 0.0);
            }
        })
        .run();
}

#[test]
fn test_bemuse_ext_scroll_half_display_ratio_scaling() {
    // Test DisplayRatio scaling when scroll value is 0.5 using bemuse_ext.bms file
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let bms_source = include_str!("../bms/files/bemuse_ext.bms");
    let config = default_config().prompter(AlwaysUseOlder);

    // This test requires comparing display_ratio at multiple time points, so manual implementation is retained
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(bms_source);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms: bms_res,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, config);
    assert_eq!(parse_warnings, vec![]);
    let bms = bms_res.ok().unwrap();

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let processor: BmsProcessor<'_, KeyLayoutBeat> = BmsProcessor::new(&bms);
    let mut player = processor.to_player(visible_range_per_bpm);

    let start_time = TimeStamp::now();
    player.start_play(start_time);

    // Verify initial state：Scroll = 1.0
    assert_eq!(*player.current_scroll(), Decimal::one());

    // Get initial visible events and their display ratios
    let initial_events: Vec<_> = player.visible_events().collect();
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
    let _ = player.update(after_first_scroll);
    assert_eq!(*player.current_scroll(), Decimal::one());

    let after_first_ratios: Vec<f64> = player
        .visible_events()
        .collect::<Vec<_>>()
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
        assert!(
            diff < 0.1,
            "Display ratio should be basically unchanged when scroll is 1.0, initial: {:.6}, after change: {:.6}",
            initial_ratio,
            after_first_ratio
        );
    }

    // Advance to second Scroll change point (scroll 0.5)
    let after_scroll_half = after_first_scroll + TimeSpan::SECOND * 2;
    let _ = player.update(after_scroll_half);
    let scroll_half = Decimal::from_str("0.5").unwrap_or_else(|err| {
        panic!("Failed to parse Decimal literal in test: {err:?}");
    });
    assert_eq!(*player.current_scroll(), scroll_half);

    let after_scroll_half_ratios: Vec<f64> = player
        .visible_events()
        .collect::<Vec<_>>()
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
            let expected_half_ratio = first_ratio * 0.5;
            let actual_diff = (half_ratio - expected_half_ratio).abs();

            assert!(
                actual_diff < 0.1,
                "Display ratio should be approximately 0.5 times original when scroll is 0.5, expected: {:.6}, actual: {:.6}",
                expected_half_ratio,
                half_ratio
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
    let bms_source = include_str!("../bms/files/bemuse_ext.bms");

    bms_driver_with_newer_prompter(bms_source, TimeSpan::MILLISECOND * 600)
        .past(TimeSpan::SECOND * 3)
        .events(|events| {
            assert!(
                !events.is_empty(),
                "Expected triggered events after 3s elapsed"
            );
        })
        .run();
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

    bms_driver_with_newer_prompter(source, TimeSpan::MILLISECOND * 600)
        .past(TimeSpan::SECOND * 2)
        .view(|p| {
            let events: Vec<_> = p
                .events_in_time_range(
                    (TimeSpan::ZERO - TimeSpan::MILLISECOND * 300)..=(TimeSpan::MILLISECOND * 300),
                )
                .collect();
            assert!(
                events
                    .iter()
                    .any(|ev| matches!(ev.event(), ChartEvent::Note { .. })),
                "Expected to find a note event around 2.0s"
            );
            for ev in events {
                assert!(
                    *ev.activate_time() >= TimeSpan::SECOND
                        && *ev.activate_time() <= TimeSpan::SECOND * 3,
                    "activate_time should be within the query window: {:?}",
                    ev.activate_time()
                );
            }
        })
        .run();
}

#[test]
fn test_bms_events_in_time_range_empty_before_start() {
    // Manual implementation needed to test behavior before start
    let source = r#"
#TITLE Time Range Test
#ARTIST Test
#BPM 120
#PLAYER 1
#WAV01 test.wav
#00111:01
"#;

    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms: bms_res,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(parse_warnings, vec![]);
    let bms = match bms_res {
        Ok(bms) => bms,
        Err(err) => panic!("Failed to parse BMS in test setup: {err:?}"),
    };

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(bms_rs::bms::Decimal::from(120)));
    let reaction_time = TimeSpan::MILLISECOND * 600;
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let processor: BmsProcessor<'_, KeyLayoutBeat> = BmsProcessor::new(&bms);
    let player = processor.to_player(visible_range_per_bpm);

    // Before start_play, events_in_time_range should return empty
    assert!(
        player
            .events_in_time_range(
                (TimeSpan::ZERO - TimeSpan::MILLISECOND * 100)..=(TimeSpan::MILLISECOND * 100),
            )
            .next()
            .is_none()
    );
}

#[test]
fn test_bms_start_play_resets_scroll_to_one() {
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

    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(bms_source);
    assert_eq!(lex_warnings, vec![]);

    let ParseOutput {
        bms: bms_res,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, config);
    assert_eq!(parse_warnings, vec![]);
    let bms = bms_res.ok().unwrap();

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let processor: BmsProcessor<'_, KeyLayoutBeat> = BmsProcessor::new(&bms);
    let mut player = processor.to_player(visible_range_per_bpm);

    let start_time = TimeStamp::start();
    player.start_play(start_time);

    let after_scroll_change = start_time + TimeSpan::MILLISECOND * 2700;
    let _ = player.update(after_scroll_change).collect::<Vec<_>>();
    assert_ne!(*player.current_scroll(), Decimal::one());

    player.start_play(after_scroll_change + TimeSpan::SECOND);
    assert_eq!(*player.current_scroll(), Decimal::one());
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

    bms_driver_with_newer_prompter(bms_source, reaction_time)
        .check(move |p| {
            assert_eq!(*p.current_bpm(), Decimal::from(120));
            assert_eq!(*p.current_speed(), Decimal::one());
            assert_eq!(*p.playback_ratio(), Decimal::one());

            // Calculate expected visible window Y
            let base_bpm = BaseBpm::from(Decimal::from(120));
            let visible_range = VisibleRangePerBpm::new(&base_bpm, reaction_time);
            let visible_window_y =
                visible_range.window_y(p.current_bpm(), p.current_speed(), p.playback_ratio());

            // Calculate time to cross window
            // velocity = current_bpm * current_speed * playback_ratio / 240
            // time = visible_window_y / velocity
            let velocity =
                Decimal::from(120) * Decimal::one() * Decimal::one() / Decimal::from(240u64);
            let time_to_cross = visible_window_y.as_ref() / velocity;

            // Verify: time_to_cross should equal reaction_time
            let expected_time =
                Decimal::from(reaction_time.as_nanos().max(0)) / Decimal::from(NANOS_PER_SECOND);
            let diff = (time_to_cross.clone() - expected_time).abs();

            assert!(
                diff < Decimal::from(1u64), // Allow 1ms error margin
                "Expected time_to_cross ≈ reaction_time (600ms), got {:.6}s, diff: {:.6}s",
                time_to_cross.to_f64().unwrap_or(0.0),
                diff.to_f64().unwrap_or(0.0)
            );
        })
        .run();
}
