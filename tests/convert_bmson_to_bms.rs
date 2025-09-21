#![cfg(feature = "bmson")]

use bms_rs::{
    bms::{command::LnMode, model::Bms},
    bmson::{Bmson, BmsonInfo, bmson_to_bms::BmsonToBmsOutput},
};
use std::borrow::Cow;
use std::num::NonZeroU64;

#[test]
fn test_bmson_to_bms_conversion() {
    // Create a simple Bmson
    let bmson = Bmson {
        version: Cow::Borrowed("1.0.0"),
        info: BmsonInfo {
            title: Cow::Borrowed("Test Song"),
            subtitle: Cow::Borrowed("Test Subtitle"),
            artist: Cow::Borrowed("Test Artist"),
            subartists: vec![Cow::Borrowed("Test Sub Artist")],
            genre: Cow::Borrowed("Test Genre"),
            mode_hint: Cow::Borrowed("beat-7k"),
            chart_name: Cow::Borrowed("NORMAL"),
            level: 5,
            init_bpm: bms_rs::bmson::fin_f64::FinF64::new(120.0).unwrap(),
            judge_rank: bms_rs::bmson::fin_f64::FinF64::new(100.0).unwrap(),
            total: bms_rs::bmson::fin_f64::FinF64::new(100.0).unwrap(),
            back_image: Some(Cow::Borrowed("back.png")),
            eyecatch_image: Some(Cow::Borrowed("eyecatch.png")),
            title_image: Some(Cow::Borrowed("title.png")),
            banner_image: Some(Cow::Borrowed("banner.png")),
            preview_music: Some(Cow::Borrowed("preview.wav")),
            resolution: NonZeroU64::new(240).unwrap(),
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
        version: Cow::Borrowed("1.0.0"),
        info: BmsonInfo {
            title: Cow::Borrowed("Test Song"),
            subtitle: Cow::Borrowed(""),
            artist: Cow::Borrowed("Test Artist"),
            subartists: vec![],
            genre: Cow::Borrowed("Test Genre"),
            mode_hint: Cow::Borrowed("beat-7k"),
            chart_name: Cow::Borrowed(""),
            level: 5,
            init_bpm: bms_rs::bmson::fin_f64::FinF64::new(120.0).unwrap(),
            judge_rank: bms_rs::bmson::fin_f64::FinF64::new(100.0).unwrap(),
            total: bms_rs::bmson::fin_f64::FinF64::new(100.0).unwrap(),
            back_image: None,
            eyecatch_image: None,
            title_image: None,
            banner_image: None,
            preview_music: None,
            resolution: NonZeroU64::new(240).unwrap(),
            ln_type: LnMode::Ln,
        },
        lines: None,
        bpm_events: vec![],
        stop_events: vec![],
        sound_channels: vec![SoundChannel {
            name: Cow::Borrowed("test.wav"),
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
