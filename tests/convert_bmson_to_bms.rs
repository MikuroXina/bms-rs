#![cfg(feature = "bmson")]

use bms_rs::{
    bms::{command::LnMode, model::Bms},
    bmson::{Bmson, BmsonInfo, bmson_to_bms::BmsonToBmsOutput, fin_f64::FinF64},
};

#[test]
fn test_bmson_to_bms_conversion() {
    // Create a simple Bmson
    let bmson = Bmson {
        version: "1.0.0".into(),
        info: BmsonInfo {
            title: "Test Song".into(),
            subtitle: "Test Subtitle".into(),
            artist: "Test Artist".into(),
            subartists: vec!["Test Sub Artist".into()],
            genre: "Test Genre".into(),
            mode_hint: "beat-7k".into(),
            chart_name: "NORMAL".into(),
            level: 5,
            init_bpm: FinF64::new(120.0).unwrap(),
            judge_rank: FinF64::new(100.0).unwrap(),
            total: FinF64::new(100.0).unwrap(),
            back_image: Some("back.png".into()),
            eyecatch_image: Some("eyecatch.png".into()),
            title_image: Some("title.png".into()),
            banner_image: Some("banner.png".into()),
            preview_music: Some("preview.wav".into()),
            resolution: 240,
            ln_type: LnMode::Ln,
        },
        lines: None,
        bpm_events: vec![],
        stop_events: vec![],
        sound_channels: vec![],
        bga: bms_rs::bmson::Bga::default(),
        scroll_events: vec![],
        mine_channels: vec![],
        key_channels: vec![],
    };

    // Convert to Bms
    let BmsonToBmsOutput { bms, .. } = Bms::from_bmson(bmson);

    // Verify conversion
    assert_eq!(bms.header.title, Some("Test Song".to_string()));
    assert_eq!(bms.header.subtitle, Some("Test Subtitle".to_string()));
    assert_eq!(bms.header.artist, Some("Test Artist".to_string()));
    assert_eq!(bms.header.sub_artist, Some("Test Sub Artist".to_string()));
    assert_eq!(bms.header.genre, Some("Test Genre".to_string()));
    assert_eq!(bms.header.play_level, Some(5));
    assert_eq!(
        bms.header.back_bmp,
        Some(std::path::PathBuf::from("back.png"))
    );
    assert_eq!(
        bms.header.stage_file,
        Some(std::path::PathBuf::from("eyecatch.png"))
    );
    assert_eq!(
        bms.header.banner,
        Some(std::path::PathBuf::from("banner.png"))
    );
    assert_eq!(
        bms.header.preview_music,
        Some(std::path::PathBuf::from("preview.wav"))
    );
}

#[test]
fn test_bmson_to_bms_with_notes() {
    use bms_rs::bmson::pulse::PulseNumber;
    use bms_rs::bmson::{Note, SoundChannel};
    use std::num::NonZeroU8;

    let bmson = Bmson {
        version: "1.0.0".into(),
        info: BmsonInfo {
            title: "Test Song".into(),
            subtitle: "".into(),
            artist: "Test Artist".into(),
            subartists: vec![],
            genre: "Test Genre".into(),
            mode_hint: "beat-7k".into(),
            chart_name: "".into(),
            level: 5,
            init_bpm: FinF64::new(120.0).unwrap(),
            judge_rank: FinF64::new(100.0).unwrap(),
            total: FinF64::new(100.0).unwrap(),
            back_image: None,
            eyecatch_image: None,
            title_image: None,
            banner_image: None,
            preview_music: None,
            resolution: 240,
            ln_type: LnMode::Ln,
        },
        lines: None,
        bpm_events: vec![],
        stop_events: vec![],
        sound_channels: vec![SoundChannel {
            name: "test.wav".into(),
            notes: vec![
                Note {
                    y: PulseNumber(240),                 // 1 quarter note
                    x: Some(NonZeroU8::new(1).unwrap()), // Key1
                    l: 0,                                // Normal note
                    c: false,
                    t: Some(LnMode::Ln),
                    up: Some(false),
                },
                Note {
                    y: PulseNumber(480),                 // 2 quarter notes
                    x: Some(NonZeroU8::new(2).unwrap()), // Key2
                    l: 240,                              // Long note
                    c: false,
                    t: Some(LnMode::Ln),
                    up: Some(false),
                },
            ],
        }],
        bga: bms_rs::bmson::Bga::default(),
        scroll_events: vec![],
        mine_channels: vec![],
        key_channels: vec![],
    };

    // Convert to Bms
    let BmsonToBmsOutput { bms, .. } = Bms::from_bmson(bmson);

    // Verify that notes were converted
    assert_eq!(bms.notes().wav_files.len(), 1);

    // Check that we have 2 notes
    let notes_count = bms.notes().all_notes().count();
    assert_eq!(notes_count, 2);
}
