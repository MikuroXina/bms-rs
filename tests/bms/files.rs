use bms_rs::bms::prelude::*;

#[test]
fn test_lal() {
    let source = include_str!("files/lilith_mx.bms");
    let BmsOutput { bms, warnings, .. } = parse_bms(source, default_config());
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
    assert_eq!(bms.bpm.bpm, Some(Decimal::from(151)));
    assert_eq!(bms.metadata.play_level, Some(7));
    assert_eq!(bms.judge.rank, Some(JudgeLevel::Easy));
    assert_eq!(bms.metadata.difficulty, Some(2));
    assert_eq!(bms.judge.total, Some(Decimal::from(359.6)));

    eprintln!("{bms:?}");
}

#[test]
fn test_nc() {
    let source = include_str!("files/nc_mx.bme");
    let BmsOutput { bms, warnings, .. } = parse_bms(source, default_config());
    assert_eq!(warnings, vec![]);

    // Check header content
    assert_eq!(bms.music_info.title.as_deref(), Some("NULCTRL"));
    assert_eq!(
        bms.music_info.artist.as_deref(),
        Some("Silentroom obj: Mikuro Xina")
    );
    assert_eq!(bms.music_info.genre.as_deref(), Some("MOTION"));
    assert_eq!(bms.music_info.subtitle.as_deref(), Some("[STX]"));
    assert_eq!(bms.bpm.bpm, Some(Decimal::from(100)));
    assert_eq!(bms.metadata.play_level, Some(5));
    assert_eq!(bms.judge.rank, Some(JudgeLevel::Easy));
    assert_eq!(bms.metadata.difficulty, Some(2));
    assert_eq!(bms.judge.total, Some(Decimal::from(260)));
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
    let BmsOutput { bms, warnings, .. } = parse_bms(source, default_config());
    assert_eq!(warnings, vec![]);

    // Check header content
    assert_eq!(bms.music_info.title.as_deref(), Some("J219"));
    assert_eq!(
        bms.music_info.artist.as_deref(),
        Some("cranky (obj: Mikuro Xina)")
    );
    assert_eq!(bms.music_info.genre.as_deref(), Some("EURO BEAT"));
    assert_eq!(bms.bpm.bpm, Some(Decimal::from(147)));
    assert_eq!(bms.metadata.play_level, Some(6));
    assert_eq!(bms.judge.rank, Some(JudgeLevel::Easy));
    assert_eq!(bms.judge.total, Some(Decimal::from(218)));
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
        parse_errors,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, default_config().prompter(AlwaysUseNewer));
    let collected_parse_warnings = parse_warnings;
    assert_eq!(parse_errors, vec![]);
    assert_eq!(
        collected_parse_warnings
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
    let BmsOutput { bms, warnings, .. } = parse_bms(source, default_config());
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
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(1))
    );
    assert_eq!(
        bms.scroll
            .scroll_defs
            .get(&ObjId::try_from("02", false).unwrap()),
        Some(&Decimal::from(0.5))
    );
    assert_eq!(
        bms.speed
            .speed_defs
            .get(&ObjId::try_from("01", false).unwrap()),
        Some(&Decimal::from(1))
    );
    assert_eq!(
        bms.speed
            .speed_defs
            .get(&ObjId::try_from("02", false).unwrap()),
        Some(&Decimal::from(0.5))
    );

    eprintln!("{bms:?}");
}
