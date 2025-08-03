use bms_rs::bms::{
    Decimal,
    command::{JudgeLevel, ObjId},
    lex::{BmsLexOutput, LexWarning, parse},
    parse::{BmsParseOutput, model::Bms, prompt::AlwaysWarn, random::rng::RngMock},
};
use num::BigUint;

#[test]
fn test_lal() {
    let source = include_str!("files/lilith_mx.bms");
    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse(source);
    assert_eq!(warnings, vec![]);
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysWarn);
    assert_eq!(parse_warnings, vec![]);

    // Check header content
    assert_eq!(
        bms.header.title.as_deref(),
        Some("Lilith ambivalence lovers")
    );
    assert_eq!(
        bms.header.artist.as_deref(),
        Some("ikaruga_nex (obj:Mikuro Xina)")
    );
    assert_eq!(bms.header.genre.as_deref(), Some("Hi-Tech Rave"));
    assert_eq!(bms.arrangers.bpm, Some(Decimal::from(151)));
    assert_eq!(bms.header.play_level, Some(7));
    assert_eq!(bms.header.rank, Some(JudgeLevel::Easy));
    assert_eq!(bms.header.difficulty, Some(2));
    assert_eq!(bms.header.total, Some(Decimal::from(359.6)));

    eprintln!("{bms:?}");
}

#[test]
fn test_nc() {
    let source = include_str!("files/nc_mx.bme");
    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse(source);
    assert_eq!(warnings, vec![]);
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysWarn);
    assert_eq!(parse_warnings, vec![]);

    // Check header content
    assert_eq!(bms.header.title.as_deref(), Some("NULCTRL"));
    assert_eq!(
        bms.header.artist.as_deref(),
        Some("Silentroom obj: Mikuro Xina")
    );
    assert_eq!(bms.header.genre.as_deref(), Some("MOTION"));
    assert_eq!(bms.header.subtitle.as_deref(), Some("[STX]"));
    assert_eq!(bms.arrangers.bpm, Some(Decimal::from(100)));
    assert_eq!(bms.header.play_level, Some(5));
    assert_eq!(bms.header.rank, Some(JudgeLevel::Easy));
    assert_eq!(bms.header.difficulty, Some(2));
    assert_eq!(bms.header.total, Some(Decimal::from(260)));
    assert_eq!(
        bms.header.stage_file.as_ref().map(|p| p.to_string_lossy()),
        Some("stagefile.png".into())
    );
    assert_eq!(
        bms.header.banner.as_ref().map(|p| p.to_string_lossy()),
        Some("banner.png".into())
    );

    eprintln!("{bms:?}");
}

#[test]
fn test_j219() {
    let source = include_str!("files/J219_7key.bms");
    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse(source);
    assert_eq!(warnings, vec![]);
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysWarn);
    assert_eq!(parse_warnings, vec![]);

    // Check header content
    assert_eq!(bms.header.title.as_deref(), Some("J219"));
    assert_eq!(
        bms.header.artist.as_deref(),
        Some("cranky (obj: Mikuro Xina)")
    );
    assert_eq!(bms.header.genre.as_deref(), Some("EURO BEAT"));
    assert_eq!(bms.arrangers.bpm, Some(Decimal::from(147)));
    assert_eq!(bms.header.play_level, Some(6));
    assert_eq!(bms.header.rank, Some(JudgeLevel::Easy));
    assert_eq!(bms.header.total, Some(Decimal::from(218)));
    assert_eq!(
        bms.header.stage_file.as_ref().map(|p| p.to_string_lossy()),
        Some("J219title.bmp".into())
    );

    eprintln!("{bms:?}");
}

#[test]
fn test_blank() {
    let source = include_str!("files/dive_withblank.bme");
    let BmsLexOutput {
        tokens: _,
        lex_warnings: warnings,
    } = parse(source);
    assert_eq!(
        warnings,
        vec![
            LexWarning::ExpectedToken {
                line: 19,
                col: 8,
                message: "key audio filename".to_string()
            },
            LexWarning::ExpectedToken {
                line: 22,
                col: 7,
                message: "key audio filename".to_string()
            }
        ]
    );
}

#[test]
fn test_bemuse_ext() {
    let source = include_str!("files/bemuse_ext.bms");
    let BmsLexOutput {
        tokens,
        lex_warnings: warnings,
    } = parse(source);
    assert_eq!(warnings, vec![]);
    let BmsParseOutput {
        bms,
        parse_warnings,
        ..
    } = Bms::from_token_stream(&tokens, RngMock([BigUint::from(1u64)]), AlwaysWarn);
    assert_eq!(parse_warnings, vec![]);

    // Check header content - this file has minimal header info
    // but should have scrolling and spacing factor changes
    assert_eq!(bms.scope_defines.scroll_defs.len(), 2);
    assert_eq!(bms.scope_defines.speed_defs.len(), 2);

    assert_eq!(bms.arrangers.scrolling_factor_changes.len(), 4);
    assert_eq!(bms.arrangers.speed_factor_changes.len(), 4);

    // Check specific values
    assert_eq!(
        bms.scope_defines
            .scroll_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(1))
    );
    assert_eq!(
        bms.scope_defines
            .scroll_defs
            .get(&ObjId::try_from("02").unwrap()),
        Some(&Decimal::from(0.5))
    );
    assert_eq!(
        bms.scope_defines
            .speed_defs
            .get(&ObjId::try_from("01").unwrap()),
        Some(&Decimal::from(1))
    );
    assert_eq!(
        bms.scope_defines
            .speed_defs
            .get(&ObjId::try_from("02").unwrap()),
        Some(&Decimal::from(0.5))
    );

    eprintln!("{bms:?}");
}
