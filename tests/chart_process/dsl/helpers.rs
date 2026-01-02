//! DSL helper functions
//!
//! Provides convenient factory functions for creating configured test drivers.

use std::collections::{BTreeMap, HashMap};

use gametime::TimeSpan;

use bms_rs::bms::prelude::*;
use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::AllEventsIndex;
use bms_rs::chart_process::base_bpm::{BaseBpm, VisibleRangePerBpm};
use bms_rs::chart_process::player::UniversalChartPlayer;
use bms_rs::chart_process::prelude::*;
use bms_rs::chart_process::resource::{HashMapResourceMapping, NameBasedResourceMapping};

use super::TestPlayerDriver;

/// Creates a BMS test driver (using `AlwaysUseOlder` prompter)
#[must_use]
pub fn bms_driver_with_older_prompter(
    source: &str,
    reaction_time: TimeSpan,
) -> TestPlayerDriver<HashMapResourceMapping> {
    let config = default_config().prompter(AlwaysUseOlder);
    let player = setup_bms_player_with_config(source, config, reaction_time);
    TestPlayerDriver::new(player)
}

/// Creates a BMS test driver (using `AlwaysWarnAndUseNewer` prompter)
#[must_use]
pub fn bms_driver_with_newer_prompter(
    source: &str,
    reaction_time: TimeSpan,
) -> TestPlayerDriver<HashMapResourceMapping> {
    let config = default_config().prompter(AlwaysWarnAndUseNewer);
    let player = setup_bms_player_with_config(source, config, reaction_time);
    TestPlayerDriver::new(player)
}

/// BMS player setup function
fn setup_bms_player_with_config<T, P, R, M>(
    source: &str,
    config: ParseConfig<T, P, R, M>,
    reaction_time: TimeSpan,
) -> UniversalChartPlayer<HashMapResourceMapping>
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
    let bms = match bms_res {
        Ok(bms) => bms,
        Err(err) => panic!("Failed to parse BMS in test setup: {err:?}"),
    };

    let base_bpm = StartBpmGenerator
        .generate(&bms)
        .unwrap_or_else(|| BaseBpm::new(bms_rs::bms::Decimal::from(120)));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let processor = BmsProcessor::<T>::new(&bms);
    processor.to_player(visible_range_per_bpm)
}

/// Creates a BMSON test driver
#[cfg(feature = "bmson")]
#[must_use]
pub fn bmson_driver(
    json: &str,
    reaction_time: TimeSpan,
) -> TestPlayerDriver<NameBasedResourceMapping> {
    let output = parse_bmson(json);
    let Some(bmson) = output.bmson else {
        panic!(
            "Failed to parse BMSON in test setup. Errors: {:?}",
            output.errors
        );
    };

    let Some(base_bpm) = StartBpmGenerator.generate(&bmson) else {
        panic!(
            "Failed to generate base BPM in test setup. Info: {:?}",
            bmson.info
        );
    };
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let processor = BmsonProcessor::new(&bmson);
    let player = processor.to_player(visible_range_per_bpm);
    TestPlayerDriver::new(player)
}

/// Creates a test player driver
#[must_use]
pub fn test_player_driver() -> TestPlayerDriver<HashMapResourceMapping> {
    use bms_rs::bms::Decimal;

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

    TestPlayerDriver::new(player)
}
