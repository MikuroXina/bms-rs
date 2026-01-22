use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive};

use bms_rs::bms::Decimal;
use bms_rs::bms::command::channel::mapper::KeyLayoutBeat;
use bms_rs::bms::prelude::*;
use bms_rs::chart_process::prelude::*;

use super::parse_bms_no_warnings;

#[test]
fn test_bms_zero_length_section_parser_allows_no_warnings() {
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
        "Parser should allow zero-length sections without warnings, got: {:?}",
        parse_warnings
    );
    let _bms = bms_res.expect("Failed to parse BMS with zero-length section");
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

    let reaction_time = TimeSpan::MILLISECOND * 1200;
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
