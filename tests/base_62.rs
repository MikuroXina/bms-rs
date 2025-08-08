use bms_rs::bms::prelude::*;

#[test]
fn test_not_base_62() {
    let BmsOutput { bms, warnings } = parse_bms(
        r"
        #WAVaa hoge.wav
        #WAVAA fuga.wav
    ",
    );
    assert!(warnings.iter().any(|w| matches!(
        w,
        BmsWarning::ParseWarning(ParseWarning {
            content: ParseWarningContent::HasDuplication,
            ..
        })
    )));
    eprintln!("{bms:?}");
    assert_eq!(bms.notes.wav_files.len(), 1);
    assert_eq!(
        bms.notes.wav_files.iter().next().unwrap().1,
        // It uses [`AlwaysWarnAndUseOlder`] by default.
        &std::path::Path::new("hoge.wav").to_path_buf()
    );
}

#[test]
fn test_base_62() {
    let BmsOutput { bms, warnings } = parse_bms(
        r"
        #WAVaa hoge.wav
        #WAVAA fuga.wav

        #BASE 62
    ",
    );
    assert!(warnings.iter().any(|w| !matches!(
        w,
        BmsWarning::ParseWarning(ParseWarning {
            content: ParseWarningContent::HasDuplication,
            ..
        })
    )));
    eprintln!("{bms:?}");
    assert_eq!(bms.notes.wav_files.len(), 2);
}
