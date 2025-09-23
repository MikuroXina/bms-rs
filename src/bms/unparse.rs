//! Unparse Bms model into Vec<Token> without重复解析逻辑。

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};

use num::Integer;

use crate::bms::prelude::*;

impl<T: KeyLayoutMapper> Bms<T> {
    /// 将 Bms 转换为 Vec<Token>（按常规顺序：头部 -> 定义 -> 资源 -> 消息）。
    /// - 避免重复解析：直接使用模型数据构造 Token；
    /// - 对需要 ObjId 的消息，优先复用现有定义；若缺失则分配新 ObjId 并补充定义 Token（仅体现在返回的 Token 列表中）。
    pub fn unparse(&self) -> Vec<Token<'static>> {
        let mut tokens: Vec<Token<'static>> = Vec::new();

        // Others section lines FIRST to preserve order equality on roundtrip
        for line in &self.others.non_command_lines {
            let s: &'static str = Box::<str>::leak(Box::<str>::from(line.as_str()));
            tokens.push(Token::NotACommand(s));
        }
        for line in &self.others.unknown_command_lines {
            let s: &'static str = Box::<str>::leak(Box::<str>::from(line.as_str()));
            tokens.push(Token::UnknownCommand(s));
        }

        // Header
        if let Some(player) = self.header.player {
            tokens.push(Token::Player(player));
        }
        if let Some(maker) = self.header.maker.as_deref() {
            let s: &'static str = Box::<str>::leak(Box::<str>::from(maker));
            tokens.push(Token::Maker(s));
        }
        if let Some(genre) = self.header.genre.as_deref() {
            let s: &'static str = Box::<str>::leak(Box::<str>::from(genre));
            tokens.push(Token::Genre(s))
        }
        if let Some(title) = self.header.title.as_deref() {
            let s: &'static str = Box::<str>::leak(Box::<str>::from(title));
            tokens.push(Token::Title(s))
        }
        if let Some(subtitle) = self.header.subtitle.as_deref() {
            let s: &'static str = Box::<str>::leak(Box::<str>::from(subtitle));
            tokens.push(Token::SubTitle(s))
        }
        if let Some(artist) = self.header.artist.as_deref() {
            let s: &'static str = Box::<str>::leak(Box::<str>::from(artist));
            tokens.push(Token::Artist(s))
        }
        if let Some(sub_artist) = self.header.sub_artist.as_deref() {
            let s: &'static str = Box::<str>::leak(Box::<str>::from(sub_artist));
            tokens.push(Token::SubArtist(s))
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
        if let Some(stage_file) = self.header.stage_file.as_ref() {
            let s: &'static str =
                Box::<str>::leak(Box::<str>::from(stage_file.to_string_lossy().into_owned()));
            tokens.push(Token::StageFile(std::path::Path::new(s)));
        }
        if let Some(back_bmp) = self.header.back_bmp.as_ref() {
            let s: &'static str =
                Box::<str>::leak(Box::<str>::from(back_bmp.to_string_lossy().into_owned()));
            tokens.push(Token::BackBmp(std::path::Path::new(s)));
        }
        if let Some(banner) = self.header.banner.as_ref() {
            let s: &'static str =
                Box::<str>::leak(Box::<str>::from(banner.to_string_lossy().into_owned()));
            tokens.push(Token::Banner(std::path::Path::new(s)));
        }
        if let Some(preview) = self.header.preview_music.as_ref() {
            let s: &'static str =
                Box::<str>::leak(Box::<str>::from(preview.to_string_lossy().into_owned()));
            tokens.push(Token::Preview(std::path::Path::new(s)));
        }
        if let Some(movie) = self.header.movie.as_ref() {
            let s: &'static str =
                Box::<str>::leak(Box::<str>::from(movie.to_string_lossy().into_owned()));
            tokens.push(Token::Movie(std::path::Path::new(s)));
        }
        if let Some(comment_lines) = self.header.comment.as_ref() {
            for line in comment_lines {
                let s: &'static str = Box::<str>::leak(Box::<str>::from(line.as_str()));
                tokens.push(Token::Comment(s));
            }
        }
        if let Some(email) = self.header.email.as_deref() {
            let s: &'static str = Box::<str>::leak(Box::<str>::from(email));
            tokens.push(Token::Email(s));
        }
        if let Some(url) = self.header.url.as_deref() {
            let s: &'static str = Box::<str>::leak(Box::<str>::from(url));
            tokens.push(Token::Url(s));
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
            let s: &'static str = Box::<str>::leak(Box::<str>::from(text.as_str()));
            tokens.push(Token::Text(*id, s));
        }
        for (id, exrank) in &self.scope_defines.exrank_defs {
            tokens.push(Token::ExRank(*id, exrank.judge_level));
        }
        #[cfg(feature = "minor-command")]
        {
            for (id, def) in &self.scope_defines.exwav_defs {
                let p: &'static std::path::Path = {
                    let s: &'static str =
                        Box::<str>::leak(Box::<str>::from(def.path.to_string_lossy().into_owned()));
                    std::path::Path::new(s)
                };
                tokens.push(Token::ExWav {
                    id: *id,
                    pan: def.pan,
                    volume: def.volume,
                    frequency: def.frequency,
                    path: p,
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
            let s: &'static str =
                Box::<str>::leak(Box::<str>::from(path_root.to_string_lossy().into_owned()));
            tokens.push(Token::PathWav(std::path::Path::new(s)));
        }
        for (id, path) in &self.notes.wav_files {
            let s: &'static str =
                Box::<str>::leak(Box::<str>::from(path.to_string_lossy().into_owned()));
            tokens.push(Token::Wav(*id, std::path::Path::new(s)));
        }
        if let Some(poor_bmp) = self.graphics.poor_bmp.as_ref() {
            let s: &'static str =
                Box::<str>::leak(Box::<str>::from(poor_bmp.to_string_lossy().into_owned()));
            tokens.push(Token::Bmp(None, std::path::Path::new(s)));
        }
        for (id, bmp) in &self.graphics.bmp_files {
            let s: &'static str =
                Box::<str>::leak(Box::<str>::from(bmp.file.to_string_lossy().into_owned()));
            if bmp.transparent_color == Argb::default() {
                tokens.push(Token::Bmp(Some(*id), std::path::Path::new(s)));
            } else {
                tokens.push(Token::ExBmp(
                    *id,
                    bmp.transparent_color,
                    std::path::Path::new(s),
                ));
            }
        }
        if let Some(video_file) = self.graphics.video_file.as_ref() {
            let s: &'static str =
                Box::<str>::leak(Box::<str>::from(video_file.to_string_lossy().into_owned()));
            tokens.push(Token::VideoFile(std::path::Path::new(s)));
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
        let mut late_def_tokens: Vec<Token<'static>> = Vec::new();
        let mut message_tokens: Vec<Token<'static>> = Vec::new();

        // Messages: Section length
        for (_track, obj) in &self.arrangers.section_len_changes {
            let msg = obj.length.to_string();
            let msg: &'static str = Box::<str>::leak(Box::<str>::from(msg));
            message_tokens.push(Token::Message {
                track: obj.track,
                channel: Channel::SectionLen,
                message: Cow::Borrowed(msg),
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

        let alloc_id = |used: &mut HashSet<ObjId>| -> ObjId {
            for i in 1..(62 * 62) {
                let id = create_obj_id_from_u16(i as u16);
                if !used.contains(&id) {
                    used.insert(id);
                    return id;
                }
            }
            ObjId::null()
        };

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
                if s.chars().all(|c| c.is_ascii_digit()) {
                    if let Ok(v) = s.parse::<u64>() {
                        if v <= u8::MAX as u64 {
                            by_track_u8
                                .entry(time.track())
                                .or_default()
                                .push((time, v as u8));
                            continue;
                        }
                    }
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
                if let Some(message) = build_id_message(items) {
                    message_tokens.push(Token::Message {
                        track,
                        channel: Channel::BpmChange,
                        message,
                    });
                }
            }
            for (track, items) in by_track_u8 {
                if let Some(message) = build_hex_message(items) {
                    message_tokens.push(Token::Message {
                        track,
                        channel: Channel::BpmChangeU8,
                        message,
                    });
                }
            }
        }

        // Messages: STOP (#xxx09)
        {
            let mut by_track: BTreeMap<Track, Vec<(ObjTime, ObjId)>> = BTreeMap::new();
            for (&time, ev) in &self.arrangers.stops {
                let id = if let Some(&id) = stop_value_to_id.get(&ev.duration) {
                    id
                } else {
                    let new_id = alloc_id(&mut used_stop_ids);
                    stop_value_to_id.insert(ev.duration.clone(), new_id);
                    late_def_tokens.push(Token::Stop(new_id, ev.duration.clone()));
                    new_id
                };
                by_track.entry(time.track()).or_default().push((time, id));
            }
            push_grouped_id_messages(&mut message_tokens, Channel::Stop, by_track);
        }

        // Messages: SCROLL (#xxxSC)
        {
            let mut by_track: BTreeMap<Track, Vec<(ObjTime, ObjId)>> = BTreeMap::new();
            for (&time, ev) in &self.arrangers.scrolling_factor_changes {
                let id = if let Some(&id) = scroll_value_to_id.get(&ev.factor) {
                    id
                } else {
                    let new_id = alloc_id(&mut used_scroll_ids);
                    scroll_value_to_id.insert(ev.factor.clone(), new_id);
                    late_def_tokens.push(Token::Scroll(new_id, ev.factor.clone()));
                    new_id
                };
                by_track.entry(time.track()).or_default().push((time, id));
            }
            push_grouped_id_messages(&mut message_tokens, Channel::Scroll, by_track);
        }

        // Messages: SPEED (#xxxSP)
        {
            let mut by_track: BTreeMap<Track, Vec<(ObjTime, ObjId)>> = BTreeMap::new();
            for (&time, ev) in &self.arrangers.speed_factor_changes {
                let id = if let Some(&id) = speed_value_to_id.get(&ev.factor) {
                    id
                } else {
                    let new_id = alloc_id(&mut used_speed_ids);
                    speed_value_to_id.insert(ev.factor.clone(), new_id);
                    late_def_tokens.push(Token::Speed(new_id, ev.factor.clone()));
                    new_id
                };
                by_track.entry(time.track()).or_default().push((time, id));
            }
            push_grouped_id_messages(&mut message_tokens, Channel::Speed, by_track);
        }

        // Messages: BGA changes (#xxx04/#xxx07/#xxx06/#xxx0A)
        {
            let mut by_track_layer: BTreeMap<(Track, (u16, u16)), Vec<(ObjTime, ObjId)>> =
                BTreeMap::new();
            for (&time, bga) in &self.graphics.bga_changes {
                let channel = bga.layer.to_channel();
                let key = (time.track(), channel_sort_key(channel));
                by_track_layer.entry(key).or_default().push((time, bga.id));
            }
            for ((track, (_r1, _r2)), items) in by_track_layer {
                let channel = {
                    // reconstruct channel from rank not stored; but we don't need for emission since we lost it.
                    // However we can infer from the first item's layer time reconstruct original channel again:
                    // We can't access layer here, so instead derive from key (_r1 identifies which channel group)
                    // Build back using match on r1
                    match _r1 {
                        0x0004 => Channel::BgaBase,
                        0x0006 => Channel::BgaPoor,
                        0x0007 => Channel::BgaLayer,
                        0x000A => Channel::BgaLayer2,
                        _ => Channel::BgaBase,
                    }
                };
                if let Some(message) = build_id_message(items) {
                    message_tokens.push(Token::Message {
                        track,
                        channel,
                        message,
                    });
                }
            }
        }

        // Messages: BGM (#xxx01) and Notes (various #xx)
        {
            for obj in self.notes.all_notes_insertion_order() {
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

        // Messages: BGM volume (#97) and KEY volume (#98)
        {
            let mut by_track_bgm: BTreeMap<Track, Vec<(ObjTime, u8)>> = BTreeMap::new();
            for (&time, ev) in &self.notes.bgm_volume_changes {
                by_track_bgm
                    .entry(time.track())
                    .or_default()
                    .push((time, ev.volume));
            }
            push_grouped_hex_messages(&mut message_tokens, Channel::BgmVolume, by_track_bgm);

            let mut by_track_key: BTreeMap<Track, Vec<(ObjTime, u8)>> = BTreeMap::new();
            for (&time, ev) in &self.notes.key_volume_changes {
                by_track_key
                    .entry(time.track())
                    .or_default()
                    .push((time, ev.volume));
            }
            push_grouped_hex_messages(&mut message_tokens, Channel::KeyVolume, by_track_key);
        }

        // Messages: TEXT (#99) and JUDGE (#A0)
        {
            let mut by_track_text: BTreeMap<Track, Vec<(ObjTime, ObjId)>> = BTreeMap::new();
            for (&time, ev) in &self.notes.text_events {
                let id = if let Some(&id) = text_value_to_id.get(ev.text.as_str()) {
                    id
                } else {
                    let new_id = alloc_id(&mut used_text_ids);
                    text_value_to_id.insert(ev.text.as_str(), new_id);
                    let s: &'static str = Box::<str>::leak(Box::<str>::from(ev.text.as_str()));
                    late_def_tokens.push(Token::Text(new_id, s));
                    new_id
                };
                by_track_text
                    .entry(time.track())
                    .or_default()
                    .push((time, id));
            }
            for (track, items) in by_track_text {
                if let Some(message) = build_id_message(items) {
                    message_tokens.push(Token::Message {
                        track,
                        channel: Channel::Text,
                        message,
                    });
                }
            }

            let mut by_track_judge: BTreeMap<Track, Vec<(ObjTime, ObjId)>> = BTreeMap::new();
            for (&time, ev) in &self.notes.judge_events {
                let id = if let Some(&id) = exrank_value_to_id.get(&ev.judge_level) {
                    id
                } else {
                    let new_id = alloc_id(&mut used_exrank_ids);
                    exrank_value_to_id.insert(ev.judge_level, new_id);
                    late_def_tokens.push(Token::ExRank(new_id, ev.judge_level));
                    new_id
                };
                by_track_judge
                    .entry(time.track())
                    .or_default()
                    .push((time, id));
            }
            for (track, items) in by_track_judge {
                if let Some(message) = build_id_message(items) {
                    message_tokens.push(Token::Message {
                        track,
                        channel: Channel::Judge,
                        message,
                    });
                }
            }
        }

        // 组装：先头部/定义/资源/其他 -> 延迟定义 -> 消息
        if !late_def_tokens.is_empty() {
            tokens.extend(late_def_tokens.into_iter());
        }
        if !message_tokens.is_empty() {
            tokens.extend(message_tokens.into_iter());
        }

        tokens
    }
}

fn build_id_message(mut items: Vec<(ObjTime, ObjId)>) -> Option<Cow<'static, str>> {
    if items.is_empty() {
        return None;
    }
    items.sort_by_key(|(t, _)| *t);
    let mut denom: u64 = 1;
    for (t, _) in &items {
        denom = denom.lcm(&t.denominator().get());
    }
    let mut last_index = 0u64;
    let mut slots: HashMap<u64, ObjId> = HashMap::new();
    for (t, id) in items {
        let idx = t.numerator() * (denom / t.denominator().get());
        last_index = last_index.max(idx);
        slots.insert(idx, id);
    }
    let mut s = String::with_capacity(((last_index + 1) * 2) as usize);
    for i in 0..=last_index {
        if let Some(id) = slots.get(&i) {
            s.push((id.to_string()).chars().nth(0).unwrap_or('0'));
            s.push((id.to_string()).chars().nth(1).unwrap_or('0'));
        } else {
            s.push('0');
            s.push('0');
        }
    }
    Some(Cow::Owned(s))
}

fn build_hex_message(mut items: Vec<(ObjTime, u8)>) -> Option<Cow<'static, str>> {
    if items.is_empty() {
        return None;
    }
    items.sort_by_key(|(t, _)| *t);
    let mut denom: u64 = 1;
    for (t, _) in &items {
        denom = denom.lcm(&t.denominator().get());
    }
    let mut last_index = 0u64;
    let mut slots: HashMap<u64, u8> = HashMap::new();
    for (t, v) in items {
        let idx = t.numerator() * (denom / t.denominator().get());
        last_index = last_index.max(idx);
        slots.insert(idx, v);
    }
    let mut s = String::with_capacity(((last_index + 1) * 2) as usize);
    for i in 0..=last_index {
        if let Some(v) = slots.get(&i) {
            use std::fmt::Write as _;
            let _ = write!(&mut s, "{:02X}", v);
        } else {
            s.push('0');
            s.push('0');
        }
    }
    Some(Cow::Owned(s))
}

fn push_grouped_id_messages(
    message_tokens: &mut Vec<Token<'static>>,
    channel: Channel,
    by_track: BTreeMap<Track, Vec<(ObjTime, ObjId)>>,
) {
    for (track, items) in by_track {
        if let Some(message) = build_id_message(items) {
            message_tokens.push(Token::Message {
                track,
                channel,
                message,
            });
        }
    }
}

fn push_grouped_hex_messages(
    message_tokens: &mut Vec<Token<'static>>,
    channel: Channel,
    by_track: BTreeMap<Track, Vec<(ObjTime, u8)>>,
) {
    for (track, items) in by_track {
        if let Some(message) = build_hex_message(items) {
            message_tokens.push(Token::Message {
                track,
                channel,
                message,
            });
        }
    }
}

fn create_obj_id_from_u16(value: u16) -> ObjId {
    let first = (value / 62) as u8;
    let second = (value % 62) as u8;
    let c1 = match first {
        0..=9 => (b'0' + first) as char,
        10..=35 => (b'A' + (first - 10)) as char,
        36..=61 => (b'a' + (first - 36)) as char,
        _ => '0',
    };
    let c2 = match second {
        0..=9 => (b'0' + second) as char,
        10..=35 => (b'A' + (second - 10)) as char,
        36..=61 => (b'a' + (second - 36)) as char,
        _ => '0',
    };
    ObjId::try_from([c1, c2]).unwrap_or_else(|_| ObjId::null())
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
