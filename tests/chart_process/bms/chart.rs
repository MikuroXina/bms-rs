use gametime::{TimeSpan, TimeStamp};

use bms_rs::bms::command::channel::mapper::KeyLayoutBeat;
use bms_rs::bms::prelude::*;
use strict_num_extended::FinF64;

use bms_rs::chart_process::prelude::*;

use super::parse_bms_no_warnings;

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
        .unwrap_or_else(|| BaseBpm::new(FinF64::try_from(120.0).unwrap()));
    let visible_range_per_bpm = VisibleRangePerBpm::new(&base_bpm, reaction_time);
    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms).expect("failed to parse chart");
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

    let chart = BmsProcessor::parse::<KeyLayoutBeat>(&bms).expect("failed to parse chart");

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
