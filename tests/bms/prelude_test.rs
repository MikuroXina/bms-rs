use bms_rs::bms::prelude::*;

#[test]
fn test_prelude_imports() {
    let _obj_id = ObjId::try_from("A1", false).unwrap();
    let _player_mode = PlayerMode::Single;
    let _judge_level = JudgeLevel::Normal;
    let _argb = Argb::default();
    let _poor_mode = PoorMode::default();
    let _ln_type = LnType::default();
    let _ln_mode_type = LnMode::default();

    let _channel = Channel::Bgm;
    let _key = Key::Key(1);
    let _note_kind = NoteKind::Visible;
    let _player_side = PlayerSide::Player1;

    let _obj_time = ObjTime::start_of(1.into());

    let _obj = WavObj {
        offset: _obj_time,
        channel_id: NoteChannelId::bgm(),
        wav_id: _obj_id,
    };
    let _bga_obj = BgaObj {
        time: _obj_time,
        layer: BgaLayer::Base,
        id: _obj_id,
    };

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
    let _wav_cmd_param = WavCmdParam::Pitch;
    let _ex_wav_pan = ExWavPan::try_from(0).unwrap();
    let _ex_wav_volume = ExWavVolume::try_from(0).unwrap();
    let _ex_wav_frequency = ExWavFrequency::try_from(100).unwrap();
    let _stp_event = StpEvent {
        time: ObjTime::start_of(1.into()),
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

    let _ex_wav_def = ExWavDef {
        id: ObjId::try_from("A1", false).unwrap(),
        pan: _ex_wav_pan,
        volume: _ex_wav_volume,
        frequency: Some(_ex_wav_frequency),
        path: "test.wav".into(),
    };

    assert_eq!(_wav_cmd_param, WavCmdParam::Pitch);
    assert_eq!(*_ex_wav_pan.as_ref(), 0);
    assert_eq!(*_ex_wav_volume.as_ref(), 0);
    assert_eq!(u64::from(_ex_wav_frequency), 100);
}

#[test]
#[cfg(feature = "diagnostics")]
fn test_prelude_diagnostics_imports() {
    let _simple_source = SimpleSource::new("test.bms", "#TITLE Test");
    let _bms_warning = BmsWarning::PlayingWarning(PlayingWarning::TotalUndefined);

    let source_text = "#TITLE Test\n#ARTIST Composer\n";
    let source = SimpleSource::new("test.bms", source_text);
    let warnings = vec![BmsWarning::PlayingWarning(PlayingWarning::TotalUndefined)];

    let reports = bms_rs::diagnostics::collect_bms_reports("test.bms", source_text, &warnings);
    assert_eq!(reports.len(), warnings.len());

    let _report = _bms_warning.to_report(&source);
}
