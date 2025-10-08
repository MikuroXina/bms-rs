use bms_rs::bms::prelude::*;

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
    } = TokenStream::parse_lex(source, default_parsers());
    // Allow warnings for files with empty resource definitions
    let _ = lex_warnings;

    // tokens -> Bms
    let ParseOutput {
        bms: bms1,
        parse_warnings,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens, AlwaysWarnAndUseOlder);
    // Allow warnings for files with empty resource definitions
    let _ = parse_warnings;

    // Bms -> tokens (unparse)
    let tokens2 = bms1.unparse::<KeyLayoutBeat>();
    let tokens2_wrapped: Vec<TokenWithRange<'_>> = tokens2
        .into_iter()
        .map(|t| SourceRangeMixin::new(t, 0..0))
        .collect();

    // tokens -> Bms
    let ParseOutput {
        bms: bms2,
        parse_warnings: parse_warnings2,
        ..
    } = Bms::from_token_stream::<'_, KeyLayoutBeat, _>(&tokens2_wrapped, AlwaysWarnAndUseOlder);
    // Allow warnings for files with empty resource definitions
    let _ = parse_warnings2;

    // Compare individual parts first to provide better debugging information
    assert_eq!(bms2.header, bms1.header, "Headers don't match");

    // Compare scope_defines sub-parts in detail
    assert_hash_maps_equal(
        &bms2.scope_defines.bpm_defs,
        &bms1.scope_defines.bpm_defs,
        "BPM definitions",
    );
    assert_hash_maps_equal(
        &bms2.scope_defines.stop_defs,
        &bms1.scope_defines.stop_defs,
        "Stop definitions",
    );
    assert_hash_maps_equal(
        &bms2.scope_defines.scroll_defs,
        &bms1.scope_defines.scroll_defs,
        "Scroll definitions",
    );
    assert_hash_maps_equal(
        &bms2.scope_defines.speed_defs,
        &bms1.scope_defines.speed_defs,
        "Speed definitions",
    );
    assert_hash_maps_equal(
        &bms2.scope_defines.exrank_defs,
        &bms1.scope_defines.exrank_defs,
        "EXRANK definitions",
    );

    // Compare minor-command scope_defines if enabled
    #[cfg(feature = "minor-command")]
    {
        assert_hash_maps_equal(
            &bms2.scope_defines.exwav_defs,
            &bms1.scope_defines.exwav_defs,
            "EXWAV definitions",
        );
        assert_hash_maps_equal(
            &bms2.scope_defines.wavcmd_events,
            &bms1.scope_defines.wavcmd_events,
            "WAVCMD events",
        );
        assert_hash_maps_equal(
            &bms2.scope_defines.atbga_defs,
            &bms1.scope_defines.atbga_defs,
            "@BGA definitions",
        );
        assert_hash_maps_equal(
            &bms2.scope_defines.bga_defs,
            &bms1.scope_defines.bga_defs,
            "BGA definitions",
        );
        assert_hash_maps_equal(
            &bms2.scope_defines.swbga_events,
            &bms1.scope_defines.swbga_events,
            "SWBGA events",
        );
        assert_hash_maps_equal(
            &bms2.scope_defines.argb_defs,
            &bms1.scope_defines.argb_defs,
            "ARGB definitions",
        );
    }

    assert_eq!(bms2.arrangers, bms1.arrangers, "Arrangers don't match");

    // Compare arrangers sub-parts in detail
    assert_btree_maps_equal(
        &bms2.arrangers.section_len_changes,
        &bms1.arrangers.section_len_changes,
        "Section length changes",
    );
    assert_eq!(bms2.arrangers.bpm, bms1.arrangers.bpm, "BPM");
    assert_btree_maps_equal(
        &bms2.arrangers.bpm_changes,
        &bms1.arrangers.bpm_changes,
        "BPM changes",
    );
    assert_eq!(
        bms2.arrangers.bpm_change_ids_used, bms1.arrangers.bpm_change_ids_used,
        "BPM change IDs used"
    );
    assert_btree_maps_equal(&bms2.arrangers.stops, &bms1.arrangers.stops, "Stops");
    assert_eq!(
        bms2.arrangers.stop_ids_used, bms1.arrangers.stop_ids_used,
        "Stop IDs used"
    );
    assert_btree_maps_equal(
        &bms2.arrangers.scrolling_factor_changes,
        &bms1.arrangers.scrolling_factor_changes,
        "Scrolling factor changes",
    );
    assert_btree_maps_equal(
        &bms2.arrangers.speed_factor_changes,
        &bms1.arrangers.speed_factor_changes,
        "Speed factor changes",
    );

    // Compare minor-command arrangers if enabled
    #[cfg(feature = "minor-command")]
    {
        assert_btree_maps_equal(
            &bms2.arrangers.stp_events,
            &bms1.arrangers.stp_events,
            "STP events",
        );
        assert_eq!(bms2.arrangers.base_bpm, bms1.arrangers.base_bpm, "Base BPM");
    }

    assert_eq!(bms2.notes, bms1.notes, "Notes don't match");

    // Compare notes sub-parts in detail
    assert_eq!(
        bms2.notes.wav_path_root, bms1.notes.wav_path_root,
        "WAV path root"
    );
    assert_hash_maps_equal(&bms2.notes.wav_files, &bms1.notes.wav_files, "WAV files");
    assert_btree_maps_equal(
        &bms2.notes.bgm_volume_changes,
        &bms1.notes.bgm_volume_changes,
        "BGM volume changes",
    );
    assert_btree_maps_equal(
        &bms2.notes.key_volume_changes,
        &bms1.notes.key_volume_changes,
        "Key volume changes",
    );
    assert_btree_maps_equal(
        &bms2.notes.text_events,
        &bms1.notes.text_events,
        "Text events",
    );
    assert_btree_maps_equal(
        &bms2.notes.judge_events,
        &bms1.notes.judge_events,
        "Judge events",
    );

    // Compare minor-command notes if enabled
    #[cfg(feature = "minor-command")]
    {
        assert_eq!(bms2.notes.midi_file, bms1.notes.midi_file, "MIDI file");
        assert_eq!(
            bms2.notes.materials_wav, bms1.notes.materials_wav,
            "Materials WAV"
        );
        assert_btree_maps_equal(
            &bms2.notes.seek_events,
            &bms1.notes.seek_events,
            "Seek events",
        );
        assert_btree_maps_equal(
            &bms2.notes.bga_keybound_events,
            &bms1.notes.bga_keybound_events,
            "BGA keybound events",
        );
        assert_btree_maps_equal(
            &bms2.notes.option_events,
            &bms1.notes.option_events,
            "Option events",
        );
    }

    // Compare graphics sub-parts in detail
    assert_eq!(
        bms2.graphics.video_file, bms1.graphics.video_file,
        "Video file"
    );
    assert_hash_maps_equal(
        &bms2.graphics.bmp_files,
        &bms1.graphics.bmp_files,
        "BMP files",
    );
    assert_btree_maps_equal(
        &bms2.graphics.bga_changes,
        &bms1.graphics.bga_changes,
        "BGA changes",
    );
    assert_eq!(bms2.graphics.poor_bmp, bms1.graphics.poor_bmp, "Poor BMP");
    assert_eq!(
        bms2.graphics.poor_bga_mode, bms1.graphics.poor_bga_mode,
        "Poor BGA mode"
    );

    // Compare minor-command graphics if enabled
    #[cfg(feature = "minor-command")]
    {
        assert_eq!(
            bms2.graphics.materials_bmp, bms1.graphics.materials_bmp,
            "Materials BMP"
        );
        assert_eq!(
            bms2.graphics.char_file, bms1.graphics.char_file,
            "Char file"
        );
        assert_eq!(
            bms2.graphics.video_colors, bms1.graphics.video_colors,
            "Video colors"
        );
        assert_eq!(
            bms2.graphics.video_dly, bms1.graphics.video_dly,
            "Video delay"
        );
        assert_eq!(
            bms2.graphics.video_fs, bms1.graphics.video_fs,
            "Video frame rate"
        );
        assert_hash_maps_equal(
            &bms2.graphics.bga_opacity_changes,
            &bms1.graphics.bga_opacity_changes,
            "BGA opacity changes",
        );
        assert_hash_maps_equal(
            &bms2.graphics.bga_argb_changes,
            &bms1.graphics.bga_argb_changes,
            "BGA ARGB changes",
        );
    }

    // Compare others sub-parts in detail
    assert_hash_maps_equal(&bms2.others.texts, &bms1.others.texts, "Texts");
    assert_eq!(
        bms2.others.non_command_lines, bms1.others.non_command_lines,
        "Non-command lines"
    );
    assert_eq!(
        bms2.others.raw_command_lines, bms1.others.raw_command_lines,
        "Raw command lines"
    );

    // Compare minor-command others if enabled
    #[cfg(feature = "minor-command")]
    {
        assert_eq!(bms2.others.options, bms1.others.options, "Options");
        assert_eq!(bms2.others.is_octave, bms1.others.is_octave, "Is octave");
        assert_eq!(bms2.others.cdda, bms1.others.cdda, "CDDA");
        assert_hash_maps_equal(
            &bms2.others.seek_events,
            &bms1.others.seek_events,
            "Seek events in others",
        );
        assert_eq!(
            bms2.others.extchr_events, bms1.others.extchr_events,
            "Extended character events"
        );
        assert_hash_maps_equal(
            &bms2.others.change_options,
            &bms1.others.change_options,
            "Change options",
        );
        assert_eq!(
            bms2.others.divide_prop, bms1.others.divide_prop,
            "Divide property"
        );
        assert_eq!(
            bms2.others.materials_path, bms1.others.materials_path,
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
