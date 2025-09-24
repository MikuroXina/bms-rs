//! Unparse Bms model into Vec<Token> without duplicate parsing logic.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};

use num::Integer;

use crate::bms::prelude::*;

type BgaTrackLayerMap<'a> = BTreeMap<(Track, (u16, u16)), Vec<(ObjTime, ObjId)>>;

impl<T: KeyLayoutMapper> Bms<T> {
    /// Convert Bms to Vec<Token> (in conventional order: header -> definitions -> resources -> messages).
    /// - Avoid duplicate parsing: directly construct Tokens using model data;
    /// - For messages requiring ObjId, prioritize reusing existing definitions; if missing, allocate new ObjId and add definition Token (only reflected in returned Token list).
    #[must_use]
    pub fn unparse<'a>(&'a self) -> Vec<Token<'a>> {
        let mut tokens: Vec<Token<'a>> = Vec::new();

        // Others section lines FIRST to preserve order equality on roundtrip
        for line in &self.others.non_command_lines {
            tokens.push(Token::NotACommand(line.as_str()));
        }
        for line in &self.others.unknown_command_lines {
            tokens.push(Token::UnknownCommand(line.as_str()));
        }

        // Header
        if let Some(player) = self.header.player {
            tokens.push(Token::Player(player));
        }
        if let Some(maker) = self.header.maker.as_deref() {
            tokens.push(Token::Maker(maker));
        }
        if let Some(genre) = self.header.genre.as_deref() {
            tokens.push(Token::Genre(genre))
        }
        if let Some(title) = self.header.title.as_deref() {
            tokens.push(Token::Title(title))
        }
        if let Some(subtitle) = self.header.subtitle.as_deref() {
            tokens.push(Token::SubTitle(subtitle))
        }
        if let Some(artist) = self.header.artist.as_deref() {
            tokens.push(Token::Artist(artist))
        }
        if let Some(sub_artist) = self.header.sub_artist.as_deref() {
            tokens.push(Token::SubArtist(sub_artist))
        }
        if let Some(play_level) = self.header.play_level {
            tokens.push(Token::PlayLevel(play_level));
        }
        if let Some(rank) = self.header.rank {
            tokens.push(Token::Rank(rank));
        }
        if let Some(difficulty) = self.header.difficulty {
            tokens.push(Token::Difficulty(difficulty));
        }
        if let Some(total) = self.header.total.as_ref() {
            tokens.push(Token::Total(total.clone()));
        }
        if let Some(stage_file) = self.header.stage_file.as_ref()
            && !stage_file.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::StageFile(stage_file.as_ref()));
        }
        if let Some(back_bmp) = self.header.back_bmp.as_ref()
            && !back_bmp.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::BackBmp(back_bmp.as_ref()));
        }
        if let Some(banner) = self.header.banner.as_ref()
            && !banner.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::Banner(banner.as_ref()));
        }
        if let Some(preview) = self.header.preview_music.as_ref()
            && !preview.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::Preview(preview.as_ref()));
        }
        if let Some(movie) = self.header.movie.as_ref()
            && !movie.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::Movie(movie.as_ref()));
        }
        if let Some(comment_lines) = self.header.comment.as_ref() {
            for line in comment_lines {
                tokens.push(Token::Comment(line.as_str()));
            }
        }
        if let Some(email) = self.header.email.as_deref() {
            tokens.push(Token::Email(email));
        }
        if let Some(url) = self.header.url.as_deref() {
            tokens.push(Token::Url(url));
        }

        // LnType
        match self.header.ln_type {
            LnType::Rdm => tokens.push(Token::LnTypeRdm),
            LnType::Mgq => tokens.push(Token::LnTypeMgq),
        }
        // LnMode
        if self.header.ln_mode != LnMode::default() {
            tokens.push(Token::LnMode(self.header.ln_mode));
        }
        // VolWav
        if self.header.volume != Volume::default() {
            tokens.push(Token::VolWav(self.header.volume));
        }

        // Definitions in scope (existing ones first)
        if let Some(bpm) = self.arrangers.bpm.as_ref() {
            tokens.push(Token::Bpm(bpm.clone()));
        }
        #[cfg(feature = "minor-command")]
        if let Some(base_bpm) = self.arrangers.base_bpm.as_ref() {
            tokens.push(Token::BaseBpm(base_bpm.clone()));
        }
        for (id, v) in &self.scope_defines.bpm_defs {
            tokens.push(Token::BpmChange(*id, v.clone()));
        }
        for (id, v) in &self.scope_defines.stop_defs {
            tokens.push(Token::Stop(*id, v.clone()));
        }
        for (id, v) in &self.scope_defines.scroll_defs {
            tokens.push(Token::Scroll(*id, v.clone()));
        }
        for (id, v) in &self.scope_defines.speed_defs {
            tokens.push(Token::Speed(*id, v.clone()));
        }
        for (id, text) in &self.others.texts {
            tokens.push(Token::Text(*id, text.as_str()));
        }
        for (id, exrank) in &self.scope_defines.exrank_defs {
            tokens.push(Token::ExRank(*id, exrank.judge_level));
        }
        #[cfg(feature = "minor-command")]
        {
            for (id, def) in &self.scope_defines.exwav_defs {
                tokens.push(Token::ExWav {
                    id: *id,
                    pan: def.pan,
                    volume: def.volume,
                    frequency: def.frequency,
                    path: def.path.as_ref(),
                });
            }
            for (_id, ev) in &self.scope_defines.wavcmd_events {
                tokens.push(Token::WavCmd(*ev));
            }
            for (id, def) in &self.scope_defines.atbga_defs {
                tokens.push(Token::AtBga {
                    id: *id,
                    source_bmp: def.source_bmp,
                    trim_top_left: def.trim_top_left.into(),
                    trim_size: def.trim_size.into(),
                    draw_point: def.draw_point.into(),
                });
            }
            for (id, def) in &self.scope_defines.bga_defs {
                tokens.push(Token::Bga {
                    id: *id,
                    source_bmp: def.source_bmp,
                    trim_top_left: def.trim_top_left.into(),
                    trim_bottom_right: def.trim_bottom_right.into(),
                    draw_point: def.draw_point.into(),
                });
            }
            for (id, argb) in &self.scope_defines.argb_defs {
                tokens.push(Token::Argb(*id, *argb));
            }
        }

        // Resources
        if let Some(path_root) = self.notes.wav_path_root.as_ref() {
            tokens.push(Token::PathWav(path_root.as_ref()));
        }
        for (id, path) in &self.notes.wav_files {
            if path.as_path().as_os_str().is_empty() {
                continue;
            }
            tokens.push(Token::Wav(*id, path.as_ref()));
        }
        if let Some(poor_bmp) = self.graphics.poor_bmp.as_ref()
            && !poor_bmp.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::Bmp(None, poor_bmp.as_ref()));
        }
        for (id, bmp) in &self.graphics.bmp_files {
            if bmp.file.as_path().as_os_str().is_empty() {
                continue;
            }
            if bmp.transparent_color == Argb::default() {
                tokens.push(Token::Bmp(Some(*id), bmp.file.as_ref()));
            } else {
                tokens.push(Token::ExBmp(*id, bmp.transparent_color, bmp.file.as_ref()));
            }
        }
        if let Some(video_file) = self.graphics.video_file.as_ref()
            && !video_file.as_path().as_os_str().is_empty()
        {
            tokens.push(Token::VideoFile(video_file.as_ref()));
        }
        #[cfg(feature = "minor-command")]
        {
            if let Some(colors) = self.graphics.video_colors {
                tokens.push(Token::VideoColors(colors));
            }
            if let Some(delay) = self.graphics.video_dly.as_ref() {
                tokens.push(Token::VideoDly(delay.clone()));
            }
            if let Some(fps) = self.graphics.video_fs.as_ref() {
                tokens.push(Token::VideoFs(fps.clone()));
            }
        }

        // Collect late definition tokens and message tokens
        let mut late_def_tokens: Vec<Token<'a>> = Vec::new();
        let mut message_tokens: Vec<Token<'a>> = Vec::new();

        // Messages: Section length
        for obj in self.arrangers.section_len_changes.values() {
            let msg = obj.length.to_string();
            message_tokens.push(Token::Message {
                track: obj.track,
                channel: Channel::SectionLen,
                message: Cow::Owned(msg),
            });
        }

        // Helper closures for mapping definitions
        let mut used_bpm_ids: HashSet<ObjId> =
            self.scope_defines.bpm_defs.keys().copied().collect();
        let mut used_stop_ids: HashSet<ObjId> =
            self.scope_defines.stop_defs.keys().copied().collect();
        let mut used_scroll_ids: HashSet<ObjId> =
            self.scope_defines.scroll_defs.keys().copied().collect();
        let mut used_speed_ids: HashSet<ObjId> =
            self.scope_defines.speed_defs.keys().copied().collect();
        let mut used_text_ids: HashSet<ObjId> = self.others.texts.keys().copied().collect();
        let mut used_exrank_ids: HashSet<ObjId> =
            self.scope_defines.exrank_defs.keys().copied().collect();

        let mut bpm_value_to_id: HashMap<Decimal, ObjId> = self
            .scope_defines
            .bpm_defs
            .iter()
            .map(|(k, v)| (v.clone(), *k))
            .collect();
        let mut stop_value_to_id: HashMap<Decimal, ObjId> = self
            .scope_defines
            .stop_defs
            .iter()
            .map(|(k, v)| (v.clone(), *k))
            .collect();
        let mut scroll_value_to_id: HashMap<Decimal, ObjId> = self
            .scope_defines
            .scroll_defs
            .iter()
            .map(|(k, v)| (v.clone(), *k))
            .collect();
        let mut speed_value_to_id: HashMap<Decimal, ObjId> = self
            .scope_defines
            .speed_defs
            .iter()
            .map(|(k, v)| (v.clone(), *k))
            .collect();
        let mut text_value_to_id: HashMap<&str, ObjId> = self
            .others
            .texts
            .iter()
            .map(|(k, v)| (v.as_str(), *k))
            .collect();
        let mut exrank_value_to_id: HashMap<JudgeLevel, ObjId> = self
            .scope_defines
            .exrank_defs
            .iter()
            .map(|(k, v)| (v.judge_level, *k))
            .collect();

        // Messages: BPM change (#xxx08 or #xxx03)
        {
            let mut by_track_id: BTreeMap<Track, Vec<(ObjTime, ObjId)>> = BTreeMap::new();
            let mut by_track_u8: BTreeMap<Track, Vec<(ObjTime, u8)>> = BTreeMap::new();
            for (&time, ev) in &self.arrangers.bpm_changes {
                if let Some(&id) = bpm_value_to_id.get(&ev.bpm) {
                    by_track_id
                        .entry(time.track())
                        .or_default()
                        .push((time, id));
                    continue;
                }
                // try treat as u8 bpm (allow trailing .0/zeros)
                let mut s = ev.bpm.to_string();
                if let Some(_dot_pos) = s.find('.') {
                    // trim trailing zeros and possible dot
                    while s.ends_with('0') {
                        s.pop();
                    }
                    if s.ends_with('.') {
                        s.pop();
                    }
                }
                if s.chars().all(|c| c.is_ascii_digit())
                    && let Ok(v) = s.parse::<u64>()
                    && v <= u8::MAX as u64
                {
                    by_track_u8
                        .entry(time.track())
                        .or_default()
                        .push((time, v as u8));
                    continue;
                }
                // otherwise, allocate new id definition
                let new_id = alloc_id(&mut used_bpm_ids);
                bpm_value_to_id.insert(ev.bpm.clone(), new_id);
                late_def_tokens.push(Token::BpmChange(new_id, ev.bpm.clone()));
                by_track_id
                    .entry(time.track())
                    .or_default()
                    .push((time, new_id));
            }
            for (track, items) in by_track_id {
                let Some(message) = build_message_line_content(items, |id| id.to_string()) else {
                    continue;
                };
                message_tokens.push(Token::Message {
                    track,
                    channel: Channel::BpmChange,
                    message,
                });
            }
            for (track, items) in by_track_u8 {
                let Some(message) =
                    build_message_line_content(items, |value| format!("{:02X}", value))
                else {
                    continue;
                };
                message_tokens.push(Token::Message {
                    track,
                    channel: Channel::BpmChangeU8,
                    message,
                });
            }
        }

        // Messages: STOP (#xxx09)
        process_message_events(
            &self.arrangers.stops,
            &mut stop_value_to_id,
            &mut used_stop_ids,
            &mut late_def_tokens,
            Token::Stop,
            |ev| ev.duration.clone(),
            Channel::Stop,
            &mut message_tokens,
        );

        // Messages: SCROLL (#xxxSC)
        process_message_events(
            &self.arrangers.scrolling_factor_changes,
            &mut scroll_value_to_id,
            &mut used_scroll_ids,
            &mut late_def_tokens,
            Token::Scroll,
            |ev| ev.factor.clone(),
            Channel::Scroll,
            &mut message_tokens,
        );

        // Messages: SPEED (#xxxSP)
        process_message_events(
            &self.arrangers.speed_factor_changes,
            &mut speed_value_to_id,
            &mut used_speed_ids,
            &mut late_def_tokens,
            Token::Speed,
            |ev| ev.factor.clone(),
            Channel::Speed,
            &mut message_tokens,
        );

        // Messages: BGA changes (#xxx04/#xxx07/#xxx06/#xxx0A)
        {
            let mut by_track_layer: BgaTrackLayerMap = BTreeMap::new();
            for (&time, bga) in &self.graphics.bga_changes {
                let channel = bga.layer.to_channel();
                let key = (time.track(), channel_sort_key(channel));
                by_track_layer.entry(key).or_default().push((time, bga.id));
            }
            for ((track, (_r1, _r2)), items) in by_track_layer {
                let channel = match _r1 {
                    0x0004 => Channel::BgaBase,
                    0x0006 => Channel::BgaPoor,
                    0x0007 => Channel::BgaLayer,
                    0x000A => Channel::BgaLayer2,
                    _ => Channel::BgaBase,
                };
                let Some(message) = build_message_line_content(items, |id| id.to_string()) else {
                    continue;
                };
                message_tokens.push(Token::Message {
                    track,
                    channel,
                    message,
                });
            }
        }

        // Messages: BGM (#xxx01) and Notes (various #xx)
        process_bgm_note_events(&self.notes, &mut message_tokens);

        // Messages: BGM volume (#97) and KEY volume (#98)
        {
            let mut by_track_bgm: BTreeMap<Track, Vec<(ObjTime, u8)>> = BTreeMap::new();
            for (&time, ev) in &self.notes.bgm_volume_changes {
                by_track_bgm
                    .entry(time.track())
                    .or_default()
                    .push((time, ev.volume));
            }
            build_messages_from_track(
                by_track_bgm,
                Channel::BgmVolume,
                &mut message_tokens,
                |items| build_message_line_content(items, |value| format!("{:02X}", value)),
            );

            let mut by_track_key: BTreeMap<Track, Vec<(ObjTime, u8)>> = BTreeMap::new();
            for (&time, ev) in &self.notes.key_volume_changes {
                by_track_key
                    .entry(time.track())
                    .or_default()
                    .push((time, ev.volume));
            }
            build_messages_from_track(
                by_track_key,
                Channel::KeyVolume,
                &mut message_tokens,
                |items| build_message_line_content(items, |value| format!("{:02X}", value)),
            );
        }

        // Messages: TEXT (#99)
        {
            let mut by_track_text: BTreeMap<Track, Vec<(ObjTime, ObjId)>> = BTreeMap::new();
            for (&time, ev) in &self.notes.text_events {
                let id = if let Some(&id) = text_value_to_id.get(ev.text.as_str()) {
                    id
                } else {
                    let new_id = alloc_id(&mut used_text_ids);
                    text_value_to_id.insert(ev.text.as_str(), new_id);
                    late_def_tokens.push(Token::Text(new_id, ev.text.as_str()));
                    new_id
                };
                by_track_text
                    .entry(time.track())
                    .or_default()
                    .push((time, id));
            }
            build_messages_from_track(by_track_text, Channel::Text, &mut message_tokens, |items| {
                build_message_line_content(items, |id| id.to_string())
            });
        }

        process_message_events(
            &self.notes.judge_events,
            &mut exrank_value_to_id,
            &mut used_exrank_ids,
            &mut late_def_tokens,
            Token::ExRank,
            |ev| ev.judge_level,
            Channel::Judge,
            &mut message_tokens,
        );

        // Assembly: header/definitions/resources/others -> late definitions -> messages
        if !late_def_tokens.is_empty() {
            tokens.extend(late_def_tokens);
        }
        if !message_tokens.is_empty() {
            tokens.extend(message_tokens);
        }

        tokens
    }
}

/// Generic function to build message strings from time-indexed values
fn build_message_line_content<'a, T, F>(
    mut items: Vec<(ObjTime, T)>,
    formatter: F,
) -> Option<Cow<'a, str>>
where
    F: Fn(&T) -> String,
{
    if items.is_empty() {
        return None;
    }
    items.sort_by_key(|(t, _)| *t);
    let mut denom: u64 = 1;
    for (t, _) in &items {
        denom = denom.lcm(&t.denominator().get());
    }
    let mut last_index = 0u64;
    let mut slots: HashMap<u64, T> = HashMap::new();
    for (t, value) in items {
        let idx = t.numerator() * (denom / t.denominator().get());
        last_index = last_index.max(idx);
        slots.insert(idx, value);
    }
    let mut s = String::with_capacity(((last_index + 1) * 2) as usize);
    for i in 0..=last_index {
        let Some(value) = slots.get(&i) else {
            s.push('0');
            s.push('0');
            continue;
        };
        s.push_str(&formatter(value));
    }
    Some(Cow::Owned(s))
}

fn channel_sort_key(channel: Channel) -> (u16, u16) {
    use Channel::*;
    match channel {
        Bgm => (0x0001, 0),
        SectionLen => (0x0002, 0),
        BpmChangeU8 => (0x0003, 0),
        BgaBase => (0x0004, 0),
        #[cfg(feature = "minor-command")]
        Seek => (0x0005, 0),
        BgaPoor => (0x0006, 0),
        BgaLayer => (0x0007, 0),
        BpmChange => (0x0008, 0),
        Stop => (0x0009, 0),
        BgaLayer2 => (0x000A, 0),
        #[cfg(feature = "minor-command")]
        BgaBaseOpacity => (0x000B, 0),
        #[cfg(feature = "minor-command")]
        BgaLayerOpacity => (0x000C, 0),
        #[cfg(feature = "minor-command")]
        BgaLayer2Opacity => (0x000D, 0),
        #[cfg(feature = "minor-command")]
        BgaPoorOpacity => (0x000E, 0),
        Scroll => (0x0100, 0),
        Speed => (0x0101, 0),
        BgmVolume => (0x0097, 0),
        KeyVolume => (0x0098, 0),
        Text => (0x0099, 0),
        Judge => (0x00A0, 0),
        #[cfg(feature = "minor-command")]
        BgaBaseArgb => (0x00A1, 0),
        #[cfg(feature = "minor-command")]
        BgaLayerArgb => (0x00A2, 0),
        #[cfg(feature = "minor-command")]
        BgaLayer2Argb => (0x00A3, 0),
        #[cfg(feature = "minor-command")]
        BgaPoorArgb => (0x00A4, 0),
        #[cfg(feature = "minor-command")]
        BgaKeybound => (0x00A5, 0),
        #[cfg(feature = "minor-command")]
        Option => (0x00A6, 0),
        #[cfg(feature = "minor-command")]
        ChangeOption => (0x0A60, 0),
        Note { channel_id } => (0xFFFF, channel_id.as_u16()),
    }
}

/// Unified generic function to process all message types with ID allocation
fn process_message_events<'a, T, K, F1, F2>(
    events: &std::collections::BTreeMap<ObjTime, T>,
    value_to_id: &mut HashMap<K, ObjId>,
    used_ids: &mut HashSet<ObjId>,
    late_def_tokens: &mut Vec<Token<'a>>,
    token_fn: F1,
    value_extractor: F2,
    channel: Channel,
    message_tokens: &mut Vec<Token<'a>>,
) where
    T: Clone,
    K: std::hash::Hash + Eq + Clone,
    F1: Fn(ObjId, K) -> Token<'a>,
    F2: Fn(&T) -> K,
{
    let mut by_track: BTreeMap<Track, Vec<(ObjTime, ObjId)>> = BTreeMap::new();

    for (&time, event) in events {
        let key = value_extractor(event);
        let id = if let Some(&id) = value_to_id.get(&key) {
            id
        } else {
            let new_id = alloc_id(used_ids);
            value_to_id.insert(key.clone(), new_id);
            late_def_tokens.push(token_fn(new_id, key.clone()));
            new_id
        };
        by_track.entry(time.track()).or_default().push((time, id));
    }

    build_messages_from_track(by_track, channel, message_tokens, |items| {
        build_message_line_content(items, |id| id.to_string())
    });
}

/// Process BGM and Note events (special case that doesn't use ID allocation)
fn process_bgm_note_events<T: KeyLayoutMapper>(notes: &Notes<T>, message_tokens: &mut Vec<Token>) {
    for obj in notes.all_notes_insertion_order() {
        let channel = if let Some(_map) = obj.channel_id.try_into_map::<T>() {
            Channel::Note {
                channel_id: obj.channel_id,
            }
        } else {
            Channel::Bgm
        };
        let track = obj.offset.track();
        let denom = obj.offset.denominator().get();
        let num = obj.offset.numerator();
        let mut s = String::with_capacity((denom as usize) * 2);
        for i in 0..denom {
            if i == num {
                let id_str = obj.wav_id.to_string();
                let mut chars = id_str.chars();
                s.push(chars.next().unwrap_or('0'));
                s.push(chars.next().unwrap_or('0'));
            } else {
                s.push('0');
                s.push('0');
            }
        }
        message_tokens.push(Token::Message {
            track,
            channel,
            message: Cow::Owned(s),
        });
    }
}

/// Generic message builder for track-based messages
fn build_messages_from_track<T, F>(
    by_track: BTreeMap<Track, Vec<(ObjTime, T)>>,
    channel: Channel,
    message_tokens: &mut Vec<Token>,
    message_builder: F,
) where
    F: Fn(Vec<(ObjTime, T)>) -> Option<Cow<'static, str>>,
{
    for (track, items) in by_track {
        let Some(message) = message_builder(items) else {
            continue;
        };
        message_tokens.push(Token::Message {
            track,
            channel,
            message,
        });
    }
}

/// Helper function to allocate a new ObjId
fn alloc_id(used: &mut HashSet<ObjId>) -> ObjId {
    for i in 1..(62 * 62) {
        let id = ObjId::try_from(i as u16).unwrap_or_else(|_| ObjId::null());
        if !used.contains(&id) {
            used.insert(id);
            return id;
        }
    }
    ObjId::null()
}
