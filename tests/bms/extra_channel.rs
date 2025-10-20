use bms_rs::bms::prelude::*;
use num::BigUint;
use std::num::NonZeroU64;

#[test]
fn test_channel_volume() {
    let src = r#"
    #00197:01020304
    #00198:22232425
    #00297:05060708
    "#;
    let BmsOutput { bms, warnings } = parse_bms_with_preset::<KeyLayoutBeat, _, _>(
        src,
        default_preset_with_rng(RngMock([BigUint::from(1u64)])),
    );
    assert!(
        warnings
            .into_iter()
            .filter(|w| matches!(w, BmsWarning::Lex(_) | BmsWarning::Parse(_)))
            .count()
            == 0
    );
    assert_eq!(bms.notes().bgm_volume_changes.len(), 8);
    assert_eq!(bms.notes().key_volume_changes.len(), 4);
    assert_eq!(
        bms.notes().bgm_volume_changes.get(&ObjTime::new(
            1,
            0,
            NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
        )),
        Some(&BgmVolumeObj {
            time: ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            volume: 1,
        })
    );
    assert_eq!(
        bms.notes().key_volume_changes.get(&ObjTime::new(
            1,
            0,
            NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
        )),
        Some(&KeyVolumeObj {
            time: ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            volume: 2 * 16 + 2,
        })
    );
    assert_eq!(
        bms.notes().bgm_volume_changes.get(&ObjTime::new(
            2,
            0,
            NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
        )),
        Some(&BgmVolumeObj {
            time: ObjTime::new(
                2,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            volume: 5,
        })
    );
    assert_eq!(
        bms.notes().key_volume_changes.get(&ObjTime::new(
            2,
            0,
            NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
        )),
        None
    );
}

#[test]
fn test_channel_text() {
    let src = r#"
    #TEXT01 Hello World
    #TEXT02  Test Message
    #00199:01000200
    #00299:02000100
    "#;
    let BmsOutput { bms, warnings } = parse_bms_with_preset::<KeyLayoutBeat, _, _>(
        src,
        default_preset_with_rng(RngMock([BigUint::from(1u64)])),
    );
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| matches!(w, BmsWarning::Lex(_) | BmsWarning::Parse(_)))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(bms.notes().text_events.len(), 4);
    assert_eq!(
        bms.notes().text_events.get(&ObjTime::new(
            1,
            0,
            NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
        )),
        Some(&TextObj {
            time: ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            text: "Hello World".to_string(),
        })
    );
    assert_eq!(
        bms.notes().text_events.get(&ObjTime::new(
            2,
            0,
            NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
        )),
        Some(&TextObj {
            time: ObjTime::new(
                2,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            text: "Test Message".to_string(),
        })
    );
}

#[test]
fn test_channel_judge() {
    // Test channel A0 (Judge)
    let Some(Channel::Judge) = read_channel("A0") else {
        panic!("Channel A0 should be Judge");
    };

    let src = r#"
    #EXRANK01 3
    #EXRANK02 2
    #001A0:01000200
    #002A0:02000100
    "#;
    let BmsOutput { bms, warnings } = parse_bms_with_preset::<KeyLayoutBeat, _, _>(
        src,
        default_preset_with_rng(RngMock([BigUint::from(1u64)])),
    );
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| matches!(w, BmsWarning::Lex(_) | BmsWarning::Parse(_)))
            .collect::<Vec<_>>(),
        vec![]
    );

    assert_eq!(bms.notes().judge_events.len(), 4);
    assert_eq!(
        bms.notes().judge_events.get(&ObjTime::new(
            1,
            0,
            NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
        )),
        Some(&JudgeObj {
            time: ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            judge_level: JudgeLevel::Easy,
        })
    );
    assert_eq!(
        bms.notes().judge_events.get(&ObjTime::new(
            2,
            0,
            NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
        )),
        Some(&JudgeObj {
            time: ObjTime::new(
                2,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            judge_level: JudgeLevel::Normal,
        })
    );
    assert_eq!(
        bms.notes().judge_events.get(&ObjTime::new(
            2,
            1,
            NonZeroU64::new(2).expect("2 should be a valid NonZeroU64")
        )),
        Some(&JudgeObj {
            time: ObjTime::new(
                2,
                1,
                NonZeroU64::new(2).expect("2 should be a valid NonZeroU64")
            ),
            judge_level: JudgeLevel::Easy,
        })
    );
}

#[cfg(feature = "minor-command")]
#[test]
fn test_bga_opacity_channels() {
    // Test BGA opacity channels as a group
    // Direct hexadecimal values for opacity (0x01-0xFF)
    let src = r#"
    #0010B:80
    #0010C:90
    #0010D:A0
    #0010E:B0
    "#;
    let BmsOutput { bms, warnings } = parse_bms_with_preset::<KeyLayoutBeat, _, _>(
        src,
        default_preset_with_rng(RngMock([BigUint::from(1u64)])),
    );
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| matches!(w, BmsWarning::Lex(_) | BmsWarning::Parse(_)))
            .collect::<Vec<_>>(),
        vec![]
    );

    // Verify BGA opacity objects are parsed correctly
    assert_eq!(bms.graphics.bga_opacity_changes.len(), 4);

    // Check BgaBaseOpacity (0B) - Base layer
    assert_eq!(
        bms.graphics
            .bga_opacity_changes
            .get(&BgaLayer::Base)
            .unwrap()
            .get(&ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            )),
        Some(&BgaOpacityObj {
            time: ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            layer: BgaLayer::Base,
            opacity: 0x80, // 128
        })
    );

    // Check BgaLayerOpacity (0C) - Overlay layer
    assert_eq!(
        bms.graphics
            .bga_opacity_changes
            .get(&BgaLayer::Overlay)
            .unwrap()
            .get(&ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            )),
        Some(&BgaOpacityObj {
            time: ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            layer: BgaLayer::Overlay,
            opacity: 0x90, // 144
        })
    );

    // Check BgaLayer2Opacity (0D) - Overlay2 layer
    assert_eq!(
        bms.graphics
            .bga_opacity_changes
            .get(&BgaLayer::Overlay2)
            .unwrap()
            .get(&ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            )),
        Some(&BgaOpacityObj {
            time: ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            layer: BgaLayer::Overlay2,
            opacity: 0xA0, // 160
        })
    );

    // Check BgaPoorOpacity (0E) - Poor layer
    assert_eq!(
        bms.graphics
            .bga_opacity_changes
            .get(&BgaLayer::Poor)
            .unwrap()
            .get(&ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            )),
        Some(&BgaOpacityObj {
            time: ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            layer: BgaLayer::Poor,
            opacity: 0xB0, // 176
        })
    );
}

#[cfg(feature = "minor-command")]
#[test]
fn test_bga_argb_channels() {
    // Test BGA ARGB channels as a group
    // Using #ARGB definitions and channel references
    let src = r#"
    #ARGB01 255,0,0,255
    #ARGB02 0,255,0,255
    #ARGB03 0,0,255,255
    #ARGB04 255,255,0,255
    #001A1:01020304
    #001A2:02010304
    #001A3:03010204
    #001A4:04010203
    "#;
    let BmsOutput { bms, warnings } = parse_bms_with_preset::<KeyLayoutBeat, _, _>(
        src,
        default_preset_with_rng(RngMock([BigUint::from(1u64)])),
    );
    assert_eq!(
        warnings
            .into_iter()
            .filter(|w| matches!(w, BmsWarning::Lex(_) | BmsWarning::Parse(_)))
            .collect::<Vec<_>>(),
        vec![]
    );

    // Verify BGA ARGB objects are parsed correctly
    assert_eq!(bms.graphics.bga_argb_changes.len(), 4);

    // Check BgaBaseArgb (A1) - Base layer with red color
    assert_eq!(
        bms.graphics
            .bga_argb_changes
            .get(&BgaLayer::Base)
            .unwrap()
            .get(&ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            )),
        Some(&BgaArgbObj {
            time: ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            layer: BgaLayer::Base,
            argb: Argb {
                alpha: 255,
                red: 0,
                green: 0,
                blue: 255,
            },
        })
    );

    // Check BgaLayerArgb (A2) - Overlay layer with green color
    assert_eq!(
        bms.graphics
            .bga_argb_changes
            .get(&BgaLayer::Overlay)
            .unwrap()
            .get(&ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            )),
        Some(&BgaArgbObj {
            time: ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            layer: BgaLayer::Overlay,
            argb: Argb {
                alpha: 0,
                red: 255,
                green: 0,
                blue: 255,
            },
        })
    );

    // Check BgaLayer2Argb (A3) - Overlay2 layer with blue color
    assert_eq!(
        bms.graphics
            .bga_argb_changes
            .get(&BgaLayer::Overlay2)
            .unwrap()
            .get(&ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            )),
        Some(&BgaArgbObj {
            time: ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            layer: BgaLayer::Overlay2,
            argb: Argb {
                alpha: 0,
                red: 0,
                green: 255,
                blue: 255,
            },
        })
    );

    // Check BgaPoorArgb (A4) - Poor layer with yellow color
    assert_eq!(
        bms.graphics
            .bga_argb_changes
            .get(&BgaLayer::Poor)
            .unwrap()
            .get(&ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            )),
        Some(&BgaArgbObj {
            time: ObjTime::new(
                1,
                0,
                NonZeroU64::new(1).expect("1 should be a valid NonZeroU64")
            ),
            layer: BgaLayer::Poor,
            argb: Argb {
                alpha: 255,
                red: 255,
                green: 0,
                blue: 255,
            },
        })
    );
}
