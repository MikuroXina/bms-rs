use strict_num_extended::{FinF64, PositiveF64};

use bms_rs::bms::prelude::*;

#[test]
fn test_lal() {
    let source = include_str!("files/lilith_mx.bms");
    let BmsOutput { bms, warnings } = parse_bms(source, default_config());
    let bms = bms.unwrap();
    assert_eq!(warnings, vec![]);

    // Check header content
    assert_eq!(
        bms.music_info.title.as_deref(),
        Some("Lilith ambivalence lovers")
    );
    assert_eq!(
        bms.music_info.artist.as_deref(),
        Some("ikaruga_nex (obj:Mikuro Xina)")
    );
    assert_eq!(bms.music_info.genre.as_deref(), Some("Hi-Tech Rave"));
    assert_eq!(
        bms.bpm.bpm.as_ref().map(|v| *v.value().as_ref().unwrap()),
        Some(PositiveF64::try_from(151.0).unwrap())
    );
    assert_eq!(bms.metadata.play_level, Some(7));
    assert_eq!(bms.judge.rank, Some(JudgeLevel::Easy));
    assert_eq!(bms.metadata.difficulty, Some(2));
    assert_eq!(
        bms.judge
            .total
            .as_ref()
            .map(|v| *v.value().as_ref().unwrap()),
        Some(FinF64::try_from(359.6).unwrap())
    );

    eprintln!("{bms:?}");
}

#[test]
fn test_nc() {
    let source = include_str!("files/nc_mx.bme");
    let BmsOutput { bms, warnings } = parse_bms(source, default_config());
    let bms = bms.unwrap();
    assert_eq!(warnings, vec![]);

    // Check header content
    assert_eq!(bms.music_info.title.as_deref(), Some("NULCTRL"));
    assert_eq!(
        bms.music_info.artist.as_deref(),
        Some("Silentroom obj: Mikuro Xina")
    );
    assert_eq!(bms.music_info.genre.as_deref(), Some("MOTION"));
    assert_eq!(bms.music_info.subtitle.as_deref(), Some("[STX]"));
    assert_eq!(
        bms.bpm.bpm.as_ref().map(|v| *v.value().as_ref().unwrap()),
        Some(PositiveF64::try_from(100.0).unwrap())
    );
    assert_eq!(bms.metadata.play_level, Some(5));
    assert_eq!(bms.judge.rank, Some(JudgeLevel::Easy));
    assert_eq!(bms.metadata.difficulty, Some(2));
    assert_eq!(
        bms.judge
            .total
            .as_ref()
            .map(|v| *v.value().as_ref().unwrap()),
        Some(FinF64::try_from(260.0).unwrap())
    );
    assert_eq!(
        bms.sprite.stage_file.as_ref().map(|p| p.to_string_lossy()),
        Some("stagefile.png".into())
    );
    assert_eq!(
        bms.sprite.banner.as_ref().map(|p| p.to_string_lossy()),
        Some("banner.png".into())
    );

    eprintln!("{bms:?}");
}

#[test]
fn test_j219() {
    let source = include_str!("files/J219_7key.bms");
    let BmsOutput { bms, warnings } = parse_bms(source, default_config());
    let bms = bms.unwrap();
    assert_eq!(warnings, vec![]);

    // Check header content
    assert_eq!(bms.music_info.title.as_deref(), Some("J219"));
    assert_eq!(
        bms.music_info.artist.as_deref(),
        Some("cranky (obj: Mikuro Xina)")
    );
    assert_eq!(bms.music_info.genre.as_deref(), Some("EURO BEAT"));
    assert_eq!(
        bms.bpm.bpm.as_ref().map(|v| *v.value().as_ref().unwrap()),
        Some(PositiveF64::try_from(147.0).unwrap())
    );
    assert_eq!(bms.metadata.play_level, Some(6));
    assert_eq!(bms.judge.rank, Some(JudgeLevel::Easy));
    assert_eq!(
        bms.judge
            .total
            .as_ref()
            .map(|v| *v.value().as_ref().unwrap()),
        Some(FinF64::try_from(218.0).unwrap())
    );
    assert_eq!(
        bms.sprite.stage_file.as_ref().map(|p| p.to_string_lossy()),
        Some("J219title.bmp".into())
    );

    eprintln!("{bms:?}");
}

#[test]
fn test_blank() {
    let source = include_str!("files/dive_withblank.bme");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(
        warnings
            .into_iter()
            .map(|w| w.content().clone())
            .collect::<Vec<_>>(),
        vec![]
    );

    let ParseOutput {
        bms: _,
        parse_warnings,
    } = Bms::from_token_stream(&tokens, default_config().prompter(AlwaysUseNewer));
    assert_eq!(
        parse_warnings
            .into_iter()
            .map(|w| w.content().clone())
            .collect::<Vec<_>>(),
        vec![
            ParseWarning::SyntaxError("expected image filename".into()),
            ParseWarning::SyntaxError("expected key audio filename".into()),
        ]
    );
}

#[test]
fn test_bemuse_ext() {
    let source = include_str!("files/bemuse_ext.bms");
    let BmsOutput { bms, warnings } = parse_bms(source, default_config());
    let bms = bms.unwrap();
    assert_eq!(
        warnings,
        vec![
            BmsWarning::PlayingWarning(PlayingWarning::TotalUndefined),
            BmsWarning::PlayingError(PlayingError::BpmUndefined),
            BmsWarning::PlayingError(PlayingError::NoNotes)
        ]
    );

    // Check header content - this file has minimal header info
    // but should have scrolling and spacing factor changes
    assert_eq!(bms.scroll.scroll_defs.len(), 2);
    assert_eq!(bms.speed.speed_defs.len(), 2);

    assert_eq!(bms.scroll.scrolling_factor_changes.len(), 4);
    assert_eq!(bms.speed.speed_factor_changes.len(), 4);

    // Check specific values
    assert_eq!(
        bms.scroll
            .scroll_defs
            .get(&ObjId::try_from("01", false).unwrap())
            .map(|v| *v.value().as_ref().unwrap()),
        Some(FinF64::try_from(1.0).unwrap())
    );
    assert_eq!(
        bms.scroll
            .scroll_defs
            .get(&ObjId::try_from("02", false).unwrap())
            .map(|v| *v.value().as_ref().unwrap()),
        Some(FinF64::try_from(0.5).unwrap())
    );
    assert_eq!(
        bms.speed
            .speed_defs
            .get(&ObjId::try_from("01", false).unwrap())
            .map(|v| *v.value().as_ref().unwrap()),
        Some(PositiveF64::try_from(1.0).unwrap())
    );
    assert_eq!(
        bms.speed
            .speed_defs
            .get(&ObjId::try_from("02", false).unwrap())
            .map(|v| *v.value().as_ref().unwrap()),
        Some(PositiveF64::try_from(0.5).unwrap())
    );

    eprintln!("{bms:?}");
}
