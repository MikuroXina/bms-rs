use bms_rs::{
    lex::{BmsLexOutput, LexWarning, command::ObjId, parse},
    parse::{Bms, BmsParseOutput, prompt::AlwaysWarn, rng::RngMock},
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
    assert_eq!(bms.header.bpm, Some(151.0));
    assert_eq!(bms.header.play_level, Some(7));
    assert_eq!(
        bms.header.rank,
        Some(bms_rs::lex::command::JudgeLevel::Easy)
    );
    assert_eq!(bms.header.difficulty, Some(2));
    assert_eq!(bms.header.total, Some(359.6));

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
    assert_eq!(bms.header.bpm, Some(100.0));
    assert_eq!(bms.header.play_level, Some(5));
    assert_eq!(
        bms.header.rank,
        Some(bms_rs::lex::command::JudgeLevel::Easy)
    );
    assert_eq!(bms.header.difficulty, Some(2));
    assert_eq!(bms.header.total, Some(260.0));
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
    assert_eq!(bms.header.bpm, Some(147.0));
    assert_eq!(bms.header.play_level, Some(6));
    assert_eq!(
        bms.header.rank,
        Some(bms_rs::lex::command::JudgeLevel::Easy)
    );
    assert_eq!(bms.header.total, Some(218.0));
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
    assert_eq!(bms.header.scrolling_factor_changes.len(), 2);
    assert_eq!(bms.header.spacing_factor_changes.len(), 2);

    // Check specific values
    assert_eq!(
        bms.header
            .scrolling_factor_changes
            .get(&ObjId::try_from("01").unwrap()),
        Some(&1.0)
    );
    assert_eq!(
        bms.header
            .scrolling_factor_changes
            .get(&ObjId::try_from("02").unwrap()),
        Some(&0.5)
    );
    assert_eq!(
        bms.header
            .spacing_factor_changes
            .get(&ObjId::try_from("01").unwrap()),
        Some(&1.0)
    );
    assert_eq!(
        bms.header
            .spacing_factor_changes
            .get(&ObjId::try_from("02").unwrap()),
        Some(&0.5)
    );

    eprintln!("{bms:?}");
}
