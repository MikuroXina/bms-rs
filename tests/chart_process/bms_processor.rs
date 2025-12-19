use std::str::FromStr;
use std::time::{Duration, Instant};

use num::{One, ToPrimitive};

use bms_rs::bms::Decimal;
use bms_rs::bms::prelude::*;
use bms_rs::chart_process::prelude::*;

#[test]
fn test_bemuse_ext_basic_visible_events_functionality() {
    // Test basic visible_events functionality using bemuse_ext.bms file
    let source = include_str!("../bms/files/bemuse_ext.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);

    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(&tokens, default_config().prompter(AlwaysUseOlder));
    assert_eq!(warnings, vec![]);
    let bms = bms.unwrap();

    // Calculate visible range per BPM from base BPM and reaction time
    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let reaction_time = Duration::from_millis(600);
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let mut processor = BmsProcessor::new::<KeyLayoutBeat>(&bms, visible_range_per_bpm);
    let start_time = Instant::now();
    processor.start_play(start_time);

    // Verify initial state
    assert_eq!(*processor.current_bpm(), Decimal::from(120));
    assert_eq!(*processor.current_speed(), Decimal::one());
    assert_eq!(*processor.current_scroll(), Decimal::one());

    // Advance to first change point
    let after_first_change = start_time + Duration::from_secs(1);
    let _ = processor.update(after_first_change);

    // Check that visible_events method works normally
    let after_change_events: Vec<_> = processor.visible_events(after_first_change).collect();
    assert!(
        !after_change_events.is_empty(),
        "Should have visible events"
    );

    // Verify display ratio calculation
    for visible_event in &after_change_events {
        let y_value = visible_event.position().value().to_f64().unwrap_or(0.0);
        let display_ratio_value = visible_event
            .display_ratio()
            .value()
            .to_f64()
            .unwrap_or(0.0);

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
    let source = include_str!("../bms/files/bemuse_ext.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);

    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(warnings, vec![]);
    let bms = bms.unwrap();

    // Calculate visible range per BPM from base BPM and reaction time
    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let reaction = Duration::from_millis(600);
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction);
    let mut processor = BmsProcessor::new::<KeyLayoutBeat>(&bms, visible_range_per_bpm);

    let start_time = Instant::now();
    processor.start_play(start_time);

    let after = start_time + Duration::from_secs(1);
    let _ = processor.update(after);
    let events: Vec<_> = processor.visible_events(after).collect();
    assert!(
        !events.is_empty(),
        "Should have visible events after advance"
    );

    for ev in events {
        let secs = ev.activate_time().as_secs_f64();
        let elapsed = after.duration_since(start_time).as_secs_f64();
        assert!(secs >= elapsed, "activate_time should be >= elapsed");
        assert!(secs.is_finite());
    }
}

#[test]
fn test_lilith_mx_bpm_changes_affect_visible_window() {
    // Test BPM changes' effect on visible window using lilith_mx.bms file
    let source = include_str!("../bms/files/lilith_mx.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);

    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(&tokens, default_config().prompter(AlwaysUseOlder));
    assert_eq!(warnings, vec![]);
    let bms = bms.unwrap();

    // Calculate visible range per BPM from base BPM and reaction time
    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let reaction_time = Duration::from_millis(600);
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let mut processor = BmsProcessor::new::<KeyLayoutBeat>(&bms, visible_range_per_bpm);
    let start_time = Instant::now();
    processor.start_play(start_time);

    // Initial state: BPM = 151
    assert_eq!(*processor.current_bpm(), Decimal::from(151));

    // Advance to first BPM change point
    // Note: With new playhead speed (1/240), speed is half of original (1/120)
    // So need twice the time to reach the same Y position
    let after_first_change = start_time + Duration::from_secs(2);
    let _ = processor.update(after_first_change);
    assert_eq!(*processor.current_bpm(), Decimal::from_str("75.5").unwrap());

    // Get visible events after BPM change
    let after_bpm_events: Vec<_> = processor.visible_events(after_first_change).collect();
    assert!(
        !after_bpm_events.is_empty(),
        "Should still have visible events after BPM change"
    );

    // Verify display ratio is still valid
    for visible_event in &after_bpm_events {
        let ratio_value = visible_event
            .display_ratio()
            .value()
            .to_f64()
            .unwrap_or(0.0);
        assert!(ratio_value.is_finite() && ratio_value >= 0.0);
    }
}

#[test]
fn test_bemuse_ext_scroll_half_display_ratio_scaling() {
    // Test DisplayRatio scaling when scroll value is 0.5 using bemuse_ext.bms file
    let source = include_str!("../bms/files/bemuse_ext.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);

    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(&tokens, default_config().prompter(AlwaysUseOlder));
    assert_eq!(warnings, vec![]);
    let bms = bms.unwrap();

    // Calculate visible range per BPM from base BPM and reaction time
    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let reaction_time = Duration::from_millis(600);
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let mut processor = BmsProcessor::new::<KeyLayoutBeat>(&bms, visible_range_per_bpm);
    let start_time = Instant::now();
    processor.start_play(start_time);

    // Verify initial state：Scroll = 1.0
    assert_eq!(*processor.current_scroll(), Decimal::one());

    // Get initial visible events and their display ratios
    let initial_events: Vec<_> = processor.visible_events(start_time).collect();
    let initial_ratios: Vec<f64> = initial_events
        .iter()
        .map(|visible_event| {
            visible_event
                .display_ratio()
                .value()
                .to_f64()
                .unwrap_or(0.0)
        })
        .collect::<Vec<_>>();

    if initial_ratios.is_empty() {
        return; // If no visible events, skip test
    }

    // Advance to first Scroll change point (still 1.0)
    let after_first_scroll = start_time + Duration::from_secs(1);
    let _ = processor.update(after_first_scroll);
    assert_eq!(*processor.current_scroll(), Decimal::one());

    let after_first_ratios: Vec<f64> = processor
        .visible_events(after_first_scroll)
        .collect::<Vec<_>>()
        .iter()
        .map(|visible_event| {
            visible_event
                .display_ratio()
                .value()
                .to_f64()
                .unwrap_or(0.0)
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
    let after_scroll_half = after_first_scroll + Duration::from_secs(2);
    let _ = processor.update(after_scroll_half);
    assert_eq!(
        *processor.current_scroll(),
        Decimal::from_str("0.5").unwrap()
    );

    let after_scroll_half_ratios: Vec<f64> = processor
        .visible_events(after_scroll_half)
        .collect::<Vec<_>>()
        .iter()
        .map(|visible_event| {
            visible_event
                .display_ratio()
                .value()
                .to_f64()
                .unwrap_or(0.0)
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
    let source = include_str!("../bms/files/bemuse_ext.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);

    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(warnings, vec![]);
    let bms = bms.unwrap();

    // Calculate visible range per BPM from base BPM and reaction time
    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let reaction_time = Duration::from_millis(600);
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let mut processor = BmsProcessor::new::<KeyLayoutBeat>(&bms, visible_range_per_bpm);

    let start_time = Instant::now();
    processor.start_play(start_time);

    let elapsed = Duration::from_secs(3);
    let now = start_time + elapsed;
    let events: Vec<_> = processor.update(now).collect();
    assert!(
        !events.is_empty(),
        "Expected triggered events after {:?} elapsed",
        elapsed
    );

    for evp in events {
        let secs_actual = evp.activate_time().as_secs_f64();
        assert!(
            secs_actual <= elapsed.as_secs_f64() + 1e-9,
            "triggered event activate_time should be <= elapsed, got {:.6} > {:.6}",
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
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);

    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(warnings, vec![]);
    let bms = bms.unwrap();

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, Duration::from_millis(600));
    let mut processor = BmsProcessor::new::<KeyLayoutBeat>(&bms, visible_range_per_bpm);

    let start_time = Instant::now() - Duration::from_secs(2);
    processor.start_play(start_time);

    let events: Vec<_> = processor
        .events_in_time_range(Duration::from_millis(300), Duration::from_millis(300))
        .collect();
    assert!(
        events
            .iter()
            .any(|ev| matches!(ev.event(), ChartEvent::Note { .. })),
        "Expected to find a note event around 2.0s"
    );
    for ev in events {
        assert!(
            *ev.activate_time() >= Duration::from_secs(1)
                && *ev.activate_time() <= Duration::from_secs(3),
            "activate_time should be within the query window: {:?}",
            ev.activate_time()
        );
    }
}

#[test]
fn test_bms_events_in_time_range_empty_before_start() {
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
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);

    let ParseOutput {
        bms,
        parse_warnings: warnings,
    } = Bms::from_token_stream(&tokens, default_config());
    assert_eq!(warnings, vec![]);
    let bms = bms.unwrap();

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, Duration::from_millis(600));
    let mut processor = BmsProcessor::new::<KeyLayoutBeat>(&bms, visible_range_per_bpm);

    let events: Vec<_> = processor
        .events_in_time_range(Duration::from_millis(100), Duration::from_millis(100))
        .collect();
    assert!(events.is_empty());
}
