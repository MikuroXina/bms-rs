use bms_rs::bms::parse::model::Bms;
use bms_rs::bmson::{Bmson, BmsonInfo};

#[test]
fn test_bmson_to_bms_conversion() {
    // Create a simple Bmson
    let bmson = Bmson {
        version: "1.0.0".to_string(),
        info: BmsonInfo {
            title: "Test Song".to_string(),
            subtitle: "Test Subtitle".to_string(),
            artist: "Test Artist".to_string(),
            subartists: vec!["Test Sub Artist".to_string()],
            genre: "Test Genre".to_string(),
            mode_hint: "beat-7k".to_string(),
            chart_name: "NORMAL".to_string(),
            level: 5,
            init_bpm: bms_rs::bmson::fin_f64::FinF64::new(120.0).unwrap(),
            judge_rank: bms_rs::bmson::fin_f64::FinF64::new(100.0).unwrap(),
            total: bms_rs::bmson::fin_f64::FinF64::new(100.0).unwrap(),
            back_image: Some("back.png".to_string()),
            eyecatch_image: Some("eyecatch.png".to_string()),
            title_image: Some("title.png".to_string()),
            banner_image: Some("banner.png".to_string()),
            preview_music: Some("preview.wav".to_string()),
            resolution: 240,
            ln_type: bms_rs::bmson::LongNoteType::LN,
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
    let bms: Bms = bmson.into();

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
        version: "1.0.0".to_string(),
        info: BmsonInfo {
            title: "Test Song".to_string(),
            subtitle: "".to_string(),
            artist: "Test Artist".to_string(),
            subartists: vec![],
            genre: "Test Genre".to_string(),
            mode_hint: "beat-7k".to_string(),
            chart_name: "".to_string(),
            level: 5,
            init_bpm: bms_rs::bmson::fin_f64::FinF64::new(120.0).unwrap(),
            judge_rank: bms_rs::bmson::fin_f64::FinF64::new(100.0).unwrap(),
            total: bms_rs::bmson::fin_f64::FinF64::new(100.0).unwrap(),
            back_image: None,
            eyecatch_image: None,
            title_image: None,
            banner_image: None,
            preview_music: None,
            resolution: 240,
            ln_type: bms_rs::bmson::LongNoteType::LN,
        },
        lines: None,
        bpm_events: vec![],
        stop_events: vec![],
        sound_channels: vec![SoundChannel {
            name: "test.wav".to_string(),
            notes: vec![
                Note {
                    y: PulseNumber(240),                 // 1 quarter note
                    x: Some(NonZeroU8::new(1).unwrap()), // Key1
                    l: 0,                                // Normal note
                    c: false,
                    t: bms_rs::bmson::LongNoteType::LN,
                    up: false,
                },
                Note {
                    y: PulseNumber(480),                 // 2 quarter notes
                    x: Some(NonZeroU8::new(2).unwrap()), // Key2
                    l: 240,                              // Long note
                    c: false,
                    t: bms_rs::bmson::LongNoteType::LN,
                    up: false,
                },
            ],
        }],
        bga: bms_rs::bmson::Bga::default(),
        scroll_events: vec![],
        mine_channels: vec![],
        key_channels: vec![],
    };

    // Convert to Bms
    let bms: Bms = bmson.into();

    // Verify that notes were converted
    assert_eq!(bms.notes.wav_files.len(), 1);
    assert_eq!(bms.notes.objs.len(), 1);

    // Check that we have 2 notes
    let notes = bms.notes.objs.values().next().unwrap();
    assert_eq!(notes.len(), 2);
}
