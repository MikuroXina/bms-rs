#![cfg(feature = "bmson")]

use bms_rs::bmson::{
    BgaEvent, BgaHeader, BgaId, BpmEvent, fin_f64::FinF64, parse_bmson, pulse::PulseNumber,
};

#[test]
fn test_bmson100_lostokens() {
    let data = include_str!("files_bmson/lostokens.bmson");
    let bmson = parse_bmson(data).expect("failed to parse bmson json");
    // Basic fields assertion
    assert_eq!(bmson.info.title, "lostokens");
    assert_eq!(bmson.info.level, 5);
    assert!(!bmson.sound_channels.is_empty());
}

#[test]
fn test_bmson100_bemusic_story_48key() {
    let data = include_str!("files_bmson/bemusicstory_483_48K_ANOTHER.bmson");
    let bmson = parse_bmson(data).expect("failed to parse bmson json");
    // Basic fields assertion
    assert_eq!(bmson.info.title, "BE-MUSiC⇒STORY".to_string());
    // Bga
    assert_eq!(
        bmson.bga.bga_header,
        vec![BgaHeader {
            id: BgaId(1),
            name: "_BGA.mp4".to_string()
        }]
    );
    assert_eq!(
        bmson.bga.bga_events,
        vec![BgaEvent {
            y: PulseNumber(31680),
            id: BgaId(1)
        }]
    );
    // Bpm Events
    assert_eq!(
        bmson.bpm_events,
        vec![
            BpmEvent {
                y: PulseNumber(31680),
                bpm: FinF64::new(199.0).unwrap()
            },
            BpmEvent {
                y: PulseNumber(3500640),
                bpm: FinF64::new(200.0).unwrap()
            }
        ]
    );
}
