use bms_rs::bms::prelude::*;
use num::BigUint;

#[test]
fn test_prelude_imports() {
    // Test that we can use types from prelude
    let _obj_id = ObjId::try_from("A1", false).unwrap();
    let _player_mode = PlayerMode::Single;
    let _judge_level = JudgeLevel::Normal;
    let _volume = Volume::default();
    let _argb = Argb::default();
    let _rgb = Rgb { r: 255, g: 0, b: 0 };
    let _poor_mode = PoorMode::default();
    let _ln_type = LnType::default();
    let _ln_mode_type = LnMode::default();

    // Test channel types
    let _channel = Channel::Bgm;
    let _key = Key::Key(1);
    let _note_kind = NoteKind::Visible;
    let _player_side = PlayerSide::Player1;

    // Test graphics types
    let _pixel_point = PixelPoint { x: 0, y: 0 };
    let _pixel_size = PixelSize {
        width: 100,
        height: 100,
    };

    // Test time types
    let _track = Track(1);
    let _obj_time = ObjTime::new(1, 0, std::num::NonZeroU64::new(1).unwrap());

    // Test model types
    let _bms = Bms::default();

    // Test model::def types
    let _at_bga_def = AtBgaDef {
        id: _obj_id,
        source_bmp: _obj_id,
        trim_top_left: _pixel_point,
        trim_size: _pixel_size,
        draw_point: _pixel_point,
    };
    let _bga_def = BgaDef {
        id: _obj_id,
        source_bmp: _obj_id,
        trim_top_left: _pixel_point,
        trim_bottom_right: _pixel_point,
        draw_point: _pixel_point,
    };
    let _bmp = Bmp {
        file: "test.bmp".into(),
        transparent_color: _argb,
    };
    let _ex_rank_def = ExRankDef {
        id: _obj_id,
        judge_level: JudgeLevel::Normal,
    };

    // Test model::obj types
    let _obj = WavObj {
        offset: _obj_time,
        channel_id: NoteChannelId::bgm(),
        wav_id: _obj_id,
    };
    let _bpm_change_obj = BpmChangeObj {
        time: _obj_time,
        bpm: Decimal::from(120),
    };
    let _section_len_change_obj = SectionLenChangeObj {
        track: _track,
        length: Decimal::from(4),
    };
    let _stop_obj = StopObj {
        time: _obj_time,
        duration: Decimal::from(1),
    };
    let _bga_obj = BgaObj {
        time: _obj_time,
        layer: BgaLayer::Base,
        id: _obj_id,
    };
    let _scrolling_factor_obj = ScrollingFactorObj {
        time: _obj_time,
        factor: Decimal::from(1),
    };
    let _speed_obj = SpeedObj {
        time: _obj_time,
        factor: Decimal::from(1),
    };

    // Test prompt types
    let _duplication_workaround = DuplicationWorkaround::UseOlder;
    let _always_use_older = AlwaysUseOlder;
    let _always_use_newer = AlwaysUseNewer;
    let _always_warn_and_use_older = AlwaysWarnAndUseOlder;
    let _always_warn_and_use_newer = AlwaysWarnAndUseNewer;

    // Test rng types
    let _rng_mock = RngMock::<1>([BigUint::from(1u32)]);

    // Test that we can use the prelude types
    assert_eq!(_player_mode, PlayerMode::Single);
    assert_eq!(_judge_level, JudgeLevel::Normal);
    assert_eq!(_poor_mode, PoorMode::Interrupt);
    assert_eq!(_ln_type, LnType::Rdm);
    assert_eq!(_ln_mode_type, LnMode::Ln);
    assert_eq!(_channel, Channel::Bgm);
    assert_eq!(_key, Key::Key(1));
    assert_eq!(_note_kind, NoteKind::Visible);
    assert_eq!(_player_side, PlayerSide::Player1);
    assert_eq!(_bga_obj.layer, BgaLayer::Base);
}

#[test]

fn test_prelude_minor_command_imports() {
    // Test minor command types when feature is enabled
    let _wav_cmd_param = WavCmdParam::Pitch;
    let _ex_wav_pan = ExWavPan::try_from(0).unwrap();
    let _ex_wav_volume = ExWavVolume::try_from(0).unwrap();
    let _ex_wav_frequency = ExWavFrequency::try_from(100).unwrap();
    let _stp_event = StpEvent {
        time: ObjTime::new(1, 0, std::num::NonZeroU64::new(1).unwrap()),
        duration: std::time::Duration::from_secs(1),
    };
    let _wav_cmd_event = WavCmdEvent {
        param: WavCmdParam::Pitch,
        wav_index: ObjId::try_from("A1", false).unwrap(),
        value: 60,
    };
    let _sw_bga_event = SwBgaEvent {
        frame_rate: 17,
        total_time: 1000,
        line: 11,
        loop_mode: false,
        argb: Argb::default(),
        pattern: "01020304".to_string(),
    };

    // Test ExWavDef
    let _ex_wav_def = ExWavDef {
        id: ObjId::try_from("A1", false).unwrap(),
        pan: _ex_wav_pan,
        volume: _ex_wav_volume,
        frequency: Some(_ex_wav_frequency),
        path: "test.wav".into(),
    };

    // Test that we can use the minor command types
    assert_eq!(_wav_cmd_param, WavCmdParam::Pitch);
    assert_eq!(_ex_wav_pan.value(), 0);
    assert_eq!(_ex_wav_volume.value(), 0);
    assert_eq!(_ex_wav_frequency.value(), 100);
}

#[test]
#[cfg(feature = "diagnostics")]
fn test_prelude_diagnostics_imports() {
    // Test diagnostics types
    let _simple_source = SimpleSource::new("test.bms", "#TITLE Test");
    let _bms_warning = BmsWarning::PlayingWarning(PlayingWarning::TotalUndefined);

    // Test diagnostics functionality
    let source_text = "#TITLE Test\n#ARTIST Composer\n";
    let source = SimpleSource::new("test.bms", source_text);
    let warnings = vec![BmsWarning::PlayingWarning(PlayingWarning::TotalUndefined)];

    // Test that diagnostics reports can be generated without printing
    let reports = bms_rs::diagnostics::collect_bms_reports("test.bms", source_text, &warnings);
    assert_eq!(reports.len(), warnings.len());

    // Test ToAriadne trait
    let _report = _bms_warning.to_report(&source);
    // Report is created successfully
}
