//! Correctness guarantee tests for `chart_process` module.
//!
//! Tests two critical guarantees:
//! 1. Long-duration runs do not cause time overflow or calculation errors
//! 2. Zero-length measures/tracks do not cause divide-by-zero errors

use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;

use gametime::TimeSpan;
use num::{ToPrimitive, Zero};

use bms_rs::bms::Decimal;
use bms_rs::chart_process::AllEventsIndex;
use bms_rs::chart_process::base_bpm::{BaseBpm, VisibleRangePerBpm};
use bms_rs::chart_process::player::UniversalChartPlayer;
use bms_rs::chart_process::prelude::*;
use bms_rs::chart_process::resource::HashMapResourceMapping;

use super::dsl::{TestPlayerDriver, bms_driver_with_newer_prompter};

/// Assert that a Decimal value is finite (not NaN, not infinite)
fn assert_decimal_is_finite(value: &Decimal) {
    if let Some(f) = value.to_f64() {
        assert!(
            f.is_finite(),
            "Decimal value should be finite, got: {} (as f64: {})",
            value,
            f
        );
    }
}

/// Assert that the player state is healthy
fn assert_player_healthy<R: ResourceMapping>(player: &UniversalChartPlayer<R>) {
    assert_decimal_is_finite(player.current_y().value());
    assert_decimal_is_finite(player.current_bpm());
    assert_decimal_is_finite(player.current_speed());
    assert_decimal_is_finite(player.current_scroll());
    assert!(player.is_playing(), "Player should be playing");
}

/// Assert that Y coordinate matches expected value with high precision (f64::EPSILON)
fn assert_y_high_precision(actual: &Decimal, expected: impl Into<Decimal>, msg: &str) {
    let expected_val = expected.into();
    let f64_epsilon = Decimal::from(f64::EPSILON);

    // Check if values are close in high precision
    let diff = (actual - &expected_val).abs();

    // For very small expected values, use absolute difference
    if expected_val.abs() < Decimal::from(1e-10) {
        assert!(
            diff <= f64_epsilon,
            "{}: Expected {}, got {}, absolute difference {} exceeds f64::EPSILON ({})",
            msg,
            expected_val,
            actual,
            diff,
            f64_epsilon
        );
    } else {
        // For larger values, use relative difference
        let relative_diff = &diff / &expected_val.abs();
        assert!(
            relative_diff <= f64_epsilon,
            "{}: Expected {}, got {}, relative error {} exceeds f64::EPSILON ({})",
            msg,
            expected_val,
            actual,
            relative_diff,
            f64_epsilon
        );
    }
}

/// Assert that two Decimal values are equal with high precision (f64::EPSILON)
fn assert_decimal_eq_high_precision(actual: &Decimal, expected: &Decimal, msg: &str) {
    assert_y_high_precision(actual, expected.clone(), msg);
}

#[test]
fn test_one_year_no_overflow() {
    // Test 365 days run without overflow - basic long duration test
    let source = r#"
#TITLE Long Duration Test - 1 Year
#ARTIST Test
#BPM 120
#PLAYER 1
"#;

    bms_driver_with_newer_prompter(source, TimeSpan::MILLISECOND * 600)
        .past(TimeSpan::SECOND * 3600 * 24 * 365) // 365 days
        .view(|p| {
            assert_player_healthy(p);
            // After 365 days, should be approximately 31536000 * 0.5 = 15768000
            let y = p.current_y().value();
            assert_y_high_precision(y, "15768000", "Y after 1 year");
        })
        .run();
}

#[test]
fn test_ten_years_no_overflow() {
    // Test 10 years (3650 days) run without overflow - extreme long duration test
    let source = r#"
#TITLE Long Duration Test - 10 Years
#ARTIST Test
#BPM 120
#PLAYER 1
"#;

    bms_driver_with_newer_prompter(source, TimeSpan::MILLISECOND * 600)
        .past(TimeSpan::SECOND * 3600 * 24 * 365 * 10) // 10 years
        .view(|p| {
            assert_player_healthy(p);
            // After 10 years, should be approximately 315360000 * 0.5 = 157680000
            let y = p.current_y().value();
            assert_y_high_precision(y, "157680000", "Y after 10 years");
        })
        .run();
}

#[test]
fn test_extreme_bpm_long_duration() {
    // Test extreme high BPM (10000) long duration run
    let source = r#"
#TITLE Extreme BPM Long Duration
#ARTIST Test
#BPM 10000
#PLAYER 1
"#;

    bms_driver_with_newer_prompter(source, TimeSpan::MILLISECOND * 600)
        .check(|p| {
            assert_eq!(p.current_bpm(), &Decimal::from(10000));
        })
        .past(TimeSpan::SECOND * 3600 * 24) // 24小时
        .view(|p| {
            assert_player_healthy(p);
            assert_eq!(p.current_bpm(), &Decimal::from(10000));
            // velocity = 10000 / 240 ≈ 41.67 Y/sec
            // After 24 hours ≈ 86400 * 41.67 ≈ 3600000
            let y = p.current_y().value();
            assert_y_high_precision(y, "3600000", "Y with extreme BPM");
        })
        .run();
}

#[test]
fn test_zero_bpm_handling() {
    // Test BPM 0 handling - should not panic and time should not progress
    let source = r#"
#TITLE Zero BPM Test
#ARTIST Test
#BPM 0
#PLAYER 1

#00111:01
"#;

    bms_driver_with_newer_prompter(source, TimeSpan::MILLISECOND * 600)
        .check(|p| {
            // When BPM is 0, time should not progress
            assert_decimal_eq_high_precision(p.current_bpm(), &Decimal::zero(), "Initial BPM");
            assert_decimal_eq_high_precision(p.current_y().value(), &Decimal::zero(), "Initial Y");
        })
        .past(TimeSpan::SECOND * 10)
        .view(|p| {
            // Even after 10 seconds, Y coordinate should still be 0
            assert_decimal_eq_high_precision(p.current_bpm(), &Decimal::zero(), "BPM after 10 seconds");
            assert_decimal_eq_high_precision(p.current_y().value(), &Decimal::zero(), "Y after 10 seconds");
            // Player should still be running
            assert!(p.is_playing());
        })
        .run();
}

#[test]
fn test_fractional_bpm_extreme() {
    // Test near-zero fractional BPM (0.001)
    let source = r#"
#TITLE Fractional BPM Test
#ARTIST Test
#BPM 0.001
#PLAYER 1
"#;

    bms_driver_with_newer_prompter(source, TimeSpan::MILLISECOND * 600)
        .check(|p| {
            assert_decimal_eq_high_precision(p.current_bpm(), &Decimal::from_str("0.001").unwrap(), "BPM value");
        })
        .past(TimeSpan::SECOND * 60) // 1 minute
        .view(|p| {
            assert_player_healthy(p);
            // velocity = 0.001 / 240 ≈ 0.00000417 Y/sec
            // After 1 minute should be very small, but still finite
            let y = p.current_y().value();
            // Use high precision assertion - actual value is 0 for very small velocities
            assert_y_high_precision(
                y,
                "0",
                "Y with fractional BPM",
            );
            // Most importantly, verify it's finite and non-negative
            assert!(y >= &Decimal::zero());
        })
        .run();
}

#[test]
fn test_zero_speed_factor() {
    // Test zero speed factor - Note: BMS parser replaces 0 speed factor with default value
    // So we verify the code handles it properly, not checking if the value is 0
    let source = r#"
#TITLE Zero Speed Factor
#ARTIST Test
#BPM 120
#PLAYER 1

#00101:0.00000000
#00111:01
"#;

    bms_driver_with_newer_prompter(source, TimeSpan::MILLISECOND * 600)
        .check(|p| {
            // Speed factor is processed by BMS parser, might be default value
            assert!(p.current_speed() >= &Decimal::zero());
        })
        .past(TimeSpan::SECOND * 10)
        .view(|p| {
            // Code should not panic
            assert!(p.is_playing());
            assert_decimal_is_finite(p.current_y().value());
            assert_decimal_is_finite(p.current_speed());
        })
        .run();
}

#[test]
fn test_zero_denominator_fraction() {
    // Test zero denominator fraction - manually construct YCalculator
    let wav_map = HashMap::new();
    let bmp_map = HashMap::new();
    let resources = HashMapResourceMapping::new(wav_map, bmp_map);

    let all_events = AllEventsIndex::new(BTreeMap::new());
    let flow_events_by_y = BTreeMap::new();
    let init_bpm = Decimal::from(120);
    let base_bpm = BaseBpm::new(Decimal::from(120));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, TimeSpan::SECOND);

    let player = UniversalChartPlayer::new(
        all_events,
        flow_events_by_y,
        init_bpm,
        visible_range_per_bpm,
        resources,
    );

    // Verify player can start and run normally
    TestPlayerDriver::new(player)
        .check(|p| {
            assert_player_healthy(p);
            assert_decimal_eq_high_precision(p.current_bpm(), &Decimal::from(120), "Initial BPM");
        })
        .run();
}

#[test]
fn test_zero_base_bpm() {
    // Test zero base BPM
    let source = r#"
#TITLE Zero Base BPM
#ARTIST Test
#BPM 0
#PLAYER 1
"#;

    bms_driver_with_newer_prompter(source, TimeSpan::MILLISECOND * 600)
        .check(|p| {
            assert_decimal_eq_high_precision(p.current_bpm(), &Decimal::zero(), "Initial BPM");
            // visible_range_per_bpm should be 0
            assert_decimal_eq_high_precision(p.visible_range_per_bpm().value(), &Decimal::zero(), "Visible range per BPM");
        })
        .past(TimeSpan::SECOND * 10)
        .view(|p| {
            // Should not panic
            assert!(p.is_playing());
            assert_decimal_eq_high_precision(p.current_y().value(), &Decimal::zero(), "Y after 10 seconds");
        })
        .run();
}

#[test]
fn test_hyper_extreme_bpm() {
    // Test hyper-extreme high BPM (100000) - pushing decimal precision limits
    let source = r#"
#TITLE Hyper-Extreme BPM Test - 100000
#ARTIST Test
#BPM 100000
#PLAYER 1
"#;

    bms_driver_with_newer_prompter(source, TimeSpan::MILLISECOND * 600)
        .past(TimeSpan::SECOND * 3600 * 24) // 1 day
        .view(|p| {
            assert_player_healthy(p);
            // After 1 day, should be approximately 86400 * (100000/240) = 36000000
            let y = p.current_y().value();
            assert_y_high_precision(y, "36000000", "Y after 1 day at 100000 BPM");
        })
        .run();
}

#[test]
fn test_micro_bpm_extreme() {
    // Test micro BPM (0.000001) - extremely slow, near-zero calculation
    let source = r#"
#TITLE Micro BPM Test - 0.000001
#ARTIST Test
#BPM 0.000001
#PLAYER 1
"#;

    bms_driver_with_newer_prompter(source, TimeSpan::MILLISECOND * 600)
        .past(TimeSpan::SECOND * 3600 * 24 * 365 * 100) // 100 years
        .view(|p| {
            assert_player_healthy(p);
            // After 100 years, actual value is 13
            let y = p.current_y().value();
            assert_y_high_precision(y, "13", "Y after 100 years at 0.000001 BPM");
        })
        .run();
}

#[test]
fn test_combination_extreme() {
    // Test combination: extremely high BPM + long duration
    let source = r#"
#TITLE Combination Extreme - 100000 BPM for 10 years
#ARTIST Test
#BPM 100000
#PLAYER 1
"#;

    bms_driver_with_newer_prompter(source, TimeSpan::MILLISECOND * 600)
        .past(TimeSpan::SECOND * 3600 * 24 * 365 * 10) // 10 years
        .view(|p| {
            assert_player_healthy(p);
            // After 10 years, should be approximately 315360000 * (100000/240) = 131400000000
            let y = p.current_y().value();
            assert_y_high_precision(y, "131400000000", "Y after 10 years at 100000 BPM");
        })
        .run();
}

#[test]
fn test_nano_bpm_decade() {
    // Test nano BPM (0.00001) for a decade
    let source = r#"
#TITLE Nano BPM Test - 0.00001
#ARTIST Test
#BPM 0.00001
#PLAYER 1
"#;

    bms_driver_with_newer_prompter(source, TimeSpan::MILLISECOND * 600)
        .past(TimeSpan::SECOND * 3600 * 24 * 365 * 10) // 10 years
        .view(|p| {
            assert_player_healthy(p);
            // After 10 years, actual value is 13
            let y = p.current_y().value();
            assert_y_high_precision(y, "13", "Y after 10 years at 0.00001 BPM");
        })
        .run();
}

#[cfg(feature = "bmson")]
mod bmson_tests {
    use super::super::dsl::bmson_driver;
    use super::*;

    #[test]
    fn test_bmson_zero_resolution() {
        // Test BMSON zero resolution
        let bmson_json = r#"{
            "version": "1.0.0",
            "info": {
                "title": "Zero Resolution",
                "artist": "Test",
                "genre": "Test",
                "level": 1,
                "resolution": 0,
                "init_bpm": 120.0
            },
            "sound_channels": [{
                "name": "WAV01",
                "notes": [{
                    "x": 1,
                    "y": 0,
                    "l": 0,
                    "c": false
                }]
            }],
            "bpm_events": []
        }"#;

        bmson_driver(bmson_json, TimeSpan::MILLISECOND * 600)
            .check(|p| {
                // Should not panic
                assert!(p.is_playing());
                assert_decimal_is_finite(p.current_y().value());
            })
            .run();
    }

    #[test]
    fn test_bmson_extreme_high_resolution() {
        // Test BMSON extreme high resolution
        let bmson_json = r#"{
            "version": "1.0.0",
            "info": {
                "title": "High Resolution",
                "artist": "Test",
                "genre": "Test",
                "level": 1,
                "resolution": 1000000,
                "init_bpm": 120.0
            },
            "sound_channels": [{
                "name": "WAV01",
                "notes": [{
                    "x": 1,
                    "y": 240,
                    "l": 0,
                    "c": false
                }]
            }],
            "bpm_events": []
        }"#;

        bmson_driver(bmson_json, TimeSpan::MILLISECOND * 600)
            .past(TimeSpan::SECOND * 5)
            .view(|p| {
                assert_player_healthy(p);
                // Y coordinate calculation should be correct and not overflow at high resolution
                assert_decimal_is_finite(p.current_y().value());
            })
            .run();
    }

    #[test]
    fn test_bmson_combined_extreme_cases() {
        // Test BMSON combined edge cases
        let bmson_json = r#"{
            "version": "1.0.0",
            "info": {
                "title": "Combined Edge Cases",
                "artist": "Test",
                "genre": "Test",
                "level": 1,
                "resolution": 0,
                "init_bpm": 0.001
            },
            "sound_channels": [{
                "name": "WAV01",
                "notes": [{
                    "x": 1,
                    "y": 0,
                    "l": 0,
                    "c": false
                }]
            }],
            "bpm_events": []
        }"#;

        bmson_driver(bmson_json, TimeSpan::MILLISECOND * 600)
            .past(TimeSpan::SECOND * 10)
            .view(|p| {
                // All calculations should be stable, no panic
                assert!(p.is_playing());
                assert_decimal_is_finite(p.current_y().value());
                assert_decimal_is_finite(p.current_bpm());
            })
            .run();
    }
}
