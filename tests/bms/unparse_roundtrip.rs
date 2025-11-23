use bms_rs::bms::prelude::*;
use pretty_assertions::assert_eq;

fn assert_btree_maps_equal<K, V>(
    left: &std::collections::BTreeMap<K, V>,
    right: &std::collections::BTreeMap<K, V>,
    field_name: &str,
) where
    K: std::fmt::Debug + std::cmp::Eq + std::cmp::Ord,
    V: std::fmt::Debug + std::cmp::PartialEq,
{
    assert_eq!(left.len(), right.len(), "{} length mismatch", field_name);

    for (key, left_value) in left {
        match right.get(key) {
            Some(right_value) => assert_eq!(
                left_value, right_value,
                "{} key {:?} mismatch",
                field_name, key
            ),
            None => panic!("{} missing key {:?} in right map", field_name, key),
        }
    }

    for key in right.keys() {
        assert!(
            left.contains_key(key),
            "{} missing key {:?} in left map",
            field_name,
            key
        );
    }
}

fn assert_hash_maps_equal<K, V>(
    left: &std::collections::HashMap<K, V>,
    right: &std::collections::HashMap<K, V>,
    field_name: &str,
) where
    K: std::fmt::Debug + std::cmp::Eq + std::hash::Hash,
    V: std::fmt::Debug + std::cmp::PartialEq,
{
    assert_eq!(
        left.len(),
        right.len(),
        "{field_name} length mismatch: {left:?} vs {right:?}",
    );

    for (key, left_value) in left {
        match right.get(key) {
            Some(right_value) => assert_eq!(
                left_value, right_value,
                "{} key {:?} mismatch",
                field_name, key
            ),
            None => panic!("{} missing key {:?} in right map", field_name, key),
        }
    }

    for key in right.keys() {
        assert!(
            left.contains_key(key),
            "{} missing key {:?} in left map",
            field_name,
            key
        );
    }
}

fn roundtrip_source_bms_tokens_bms(source: &str) {
    // file -> tokens
    let LexOutput {
        tokens,
        lex_warnings,
    } = TokenStream::parse_lex(source);
    // 始终检查词法警告
    assert_eq!(lex_warnings, vec![]);

    // tokens -> Bms
    let ParseOutput {
        bms: bms1,
        parse_warnings: warnings1,
        control_flow_errors: parse_errors1,
    } = Bms::from_token_stream(&tokens, default_config().prompter(AlwaysWarnAndUseOlder));
    assert_eq!(warnings1, vec![]);
    assert_eq!(parse_errors1, vec![]);

    // Bms -> tokens (unparse)
    let tokens2 = bms1.unparse::<KeyLayoutBeat>();
    let tokens2_wrapped: Vec<TokenWithRange<'_>> = tokens2
        .into_iter()
        .map(|t| SourceRangeMixin::new(t, 0..0))
        .collect();

    // tokens -> Bms
    let ParseOutput {
        bms: bms2,
        parse_warnings: warnings2,
        control_flow_errors: parse_errors2,
    } = Bms::from_token_stream(
        &tokens2_wrapped,
        default_config().prompter(AlwaysWarnAndUseOlder),
    );
    assert_eq!(warnings2, vec![]);
    assert_eq!(parse_errors2, vec![]);

    // Compare individual parts first to provide better debugging information
    assert_eq!(bms2.repr, bms1.repr, "Representations don't match");
    assert_eq!(
        bms2.music_info, bms1.music_info,
        "Music information don't match"
    );
    assert_eq!(bms2.metadata, bms1.metadata, "Metadata don't match");

    // Compare definitions in detail
    assert_hash_maps_equal(&bms2.bpm.bpm_defs, &bms1.bpm.bpm_defs, "BPM definitions");
    assert_hash_maps_equal(
        &bms2.stop.stop_defs,
        &bms1.stop.stop_defs,
        "Stop definitions",
    );
    assert_hash_maps_equal(
        &bms2.scroll.scroll_defs,
        &bms1.scroll.scroll_defs,
        "Scroll definitions",
    );
    assert_hash_maps_equal(
        &bms2.speed.speed_defs,
        &bms1.speed.speed_defs,
        "Speed definitions",
    );
    assert_hash_maps_equal(
        &bms2.judge.exrank_defs,
        &bms1.judge.exrank_defs,
        "EXRANK definitions",
    );

    // Compare minor-command scope_defines if enabled

    {
        assert_hash_maps_equal(
            &bms2.wav.exwav_defs,
            &bms1.wav.exwav_defs,
            "EXWAV definitions",
        );
        assert_hash_maps_equal(
            &bms2.wav.wavcmd_events,
            &bms1.wav.wavcmd_events,
            "WAVCMD events",
        );
        assert_hash_maps_equal(
            &bms2.bmp.atbga_defs,
            &bms1.bmp.atbga_defs,
            "@BGA definitions",
        );
        assert_hash_maps_equal(&bms2.bmp.bga_defs, &bms1.bmp.bga_defs, "BGA definitions");
        assert_hash_maps_equal(
            &bms2.bmp.swbga_events,
            &bms1.bmp.swbga_events,
            "SWBGA events",
        );
        assert_hash_maps_equal(&bms2.bmp.argb_defs, &bms1.bmp.argb_defs, "ARGB definitions");
    }

    // Compare events in detail
    assert_btree_maps_equal(
        &bms2.section_len.section_len_changes,
        &bms1.section_len.section_len_changes,
        "Section length changes",
    );
    assert_eq!(bms2.bpm.bpm, bms1.bpm.bpm, "BPM");
    assert_btree_maps_equal(&bms2.bpm.bpm_changes, &bms1.bpm.bpm_changes, "BPM changes");
    assert_eq!(
        bms2.bpm.bpm_change_ids_used, bms1.bpm.bpm_change_ids_used,
        "BPM change IDs used"
    );
    assert_btree_maps_equal(&bms2.stop.stops, &bms1.stop.stops, "Stops");
    assert_eq!(
        bms2.stop.stop_ids_used, bms1.stop.stop_ids_used,
        "Stop IDs used"
    );
    assert_btree_maps_equal(
        &bms2.scroll.scrolling_factor_changes,
        &bms1.scroll.scrolling_factor_changes,
        "Scrolling factor changes",
    );
    assert_btree_maps_equal(
        &bms2.speed.speed_factor_changes,
        &bms1.speed.speed_factor_changes,
        "Speed factor changes",
    );

    // Compare minor-command arrangers if enabled

    {
        assert_btree_maps_equal(&bms2.stop.stp_events, &bms1.stop.stp_events, "STP events");
        assert_eq!(bms2.bpm.base_bpm, bms1.bpm.base_bpm, "Base BPM");
    }

    // Compare notes in detail
    assert_eq!(
        bms2.metadata.wav_path_root, bms1.metadata.wav_path_root,
        "WAV path root"
    );
    assert_hash_maps_equal(&bms2.wav.wav_files, &bms1.wav.wav_files, "WAV files");
    assert_btree_maps_equal(
        &bms2.volume.bgm_volume_changes,
        &bms1.volume.bgm_volume_changes,
        "BGM volume changes",
    );
    assert_btree_maps_equal(
        &bms2.volume.key_volume_changes,
        &bms1.volume.key_volume_changes,
        "Key volume changes",
    );
    assert_btree_maps_equal(
        &bms2.text.text_events,
        &bms1.text.text_events,
        "Text events",
    );
    assert_btree_maps_equal(
        &bms2.judge.judge_events,
        &bms1.judge.judge_events,
        "Judge events",
    );

    // Compare minor-command notes if enabled

    {
        assert_eq!(
            bms2.resources.midi_file, bms1.resources.midi_file,
            "MIDI file"
        );
        assert_eq!(
            bms2.resources.materials_wav, bms1.resources.materials_wav,
            "Materials WAV"
        );
        assert_btree_maps_equal(
            &bms2.video.seek_events,
            &bms1.video.seek_events,
            "Seek events",
        );
        assert_btree_maps_equal(
            &bms2.bmp.bga_keybound_events,
            &bms1.bmp.bga_keybound_events,
            "BGA keybound events",
        );
        assert_btree_maps_equal(
            &bms2.option.option_events,
            &bms1.option.option_events,
            "Option events",
        );
    }

    // Compare graphics sub-parts in detail
    assert_eq!(bms2.video.video_file, bms1.video.video_file, "Video file");
    assert_hash_maps_equal(&bms2.bmp.bmp_files, &bms1.bmp.bmp_files, "BMP files");
    assert_btree_maps_equal(&bms2.bmp.bga_changes, &bms1.bmp.bga_changes, "BGA changes");
    assert_eq!(bms2.bmp.poor_bmp, bms1.bmp.poor_bmp, "Poor BMP");
    assert_eq!(
        bms2.bmp.poor_bga_mode, bms1.bmp.poor_bga_mode,
        "Poor BGA mode"
    );

    // Compare minor-command graphics if enabled

    {
        assert_eq!(
            bms2.resources.materials_bmp, bms1.resources.materials_bmp,
            "Materials BMP"
        );
        assert_eq!(bms2.sprite.char_file, bms1.sprite.char_file, "Char file");
        assert_eq!(
            bms2.video.video_colors, bms1.video.video_colors,
            "Video colors"
        );
        assert_eq!(bms2.video.video_dly, bms1.video.video_dly, "Video delay");
        assert_eq!(bms2.video.video_fs, bms1.video.video_fs, "Video frame rate");
        assert_hash_maps_equal(
            &bms2.bmp.bga_opacity_changes,
            &bms1.bmp.bga_opacity_changes,
            "BGA opacity changes",
        );
        assert_hash_maps_equal(
            &bms2.bmp.bga_argb_changes,
            &bms1.bmp.bga_argb_changes,
            "BGA ARGB changes",
        );
    }

    // Compare others sub-parts in detail
    assert_hash_maps_equal(&bms2.text.texts, &bms1.text.texts, "Texts");
    assert_eq!(
        bms2.repr.non_command_lines, bms1.repr.non_command_lines,
        "Non-command lines"
    );
    assert_eq!(
        bms2.repr.raw_command_lines, bms1.repr.raw_command_lines,
        "Raw command lines"
    );

    // Compare minor-command others if enabled

    {
        assert_eq!(bms2.option.options, bms1.option.options, "Options");
        assert_eq!(
            bms2.metadata.is_octave, bms1.metadata.is_octave,
            "Is octave"
        );
        assert_eq!(bms2.resources.cdda, bms1.resources.cdda, "CDDA");
        assert_hash_maps_equal(
            &bms2.video.seek_defs,
            &bms1.video.seek_defs,
            "Seek events in others",
        );
        assert_eq!(
            bms2.sprite.extchr_events, bms1.sprite.extchr_events,
            "Extended character events"
        );
        assert_hash_maps_equal(
            &bms2.option.change_options,
            &bms1.option.change_options,
            "Change options",
        );
        assert_eq!(
            bms2.metadata.divide_prop, bms1.metadata.divide_prop,
            "Divide property"
        );
        assert_eq!(
            bms2.resources.materials_path, bms1.resources.materials_path,
            "Materials path"
        );
    }

    // If all parts match, the objects should be equal
    assert_eq!(bms2, bms1);
}

#[test]
fn roundtrip_lilith_mx_file_bms_tokens_bms() {
    let source = include_str!("files/lilith_mx.bms");
    roundtrip_source_bms_tokens_bms(source);
}

#[test]
fn roundtrip_bemuse_ext_file_bms_tokens_bms() {
    let source = include_str!("files/bemuse_ext.bms");
    roundtrip_source_bms_tokens_bms(source);
}

#[test]
fn roundtrip_j219_7key_file_bms_tokens_bms() {
    let source = include_str!("files/J219_7key.bms");
    roundtrip_source_bms_tokens_bms(source);
}

#[test]
fn roundtrip_nc_mx_file_bms_tokens_bms() {
    let source = include_str!("files/nc_mx.bme");
    roundtrip_source_bms_tokens_bms(source);
}
