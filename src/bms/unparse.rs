//! Unparse Bms model into Vec<Token> without duplicate parsing logic.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};

use num::Integer;

use crate::bms::prelude::*;

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
        // Use iterator chains to efficiently collect all definition tokens
        let mut def_tokens: Vec<Token> = Vec::new();

        // Add basic definitions
        if let Some(bpm) = self.arrangers.bpm.as_ref() {
            def_tokens.push(Token::Bpm(bpm.clone()));
        }
        #[cfg(feature = "minor-command")]
        if let Some(base_bpm) = self.arrangers.base_bpm.as_ref() {
            def_tokens.push(Token::BaseBpm(base_bpm.clone()));
        }

        // Collect definition tokens using iterator chains
        def_tokens.extend(
            self.scope_defines
                .bpm_defs
                .iter()
                .map(|(id, v)| Token::BpmChange(*id, v.clone())),
        );

        def_tokens.extend(
            self.scope_defines
                .stop_defs
                .iter()
                .map(|(id, v)| Token::Stop(*id, v.clone())),
        );

        def_tokens.extend(
            self.scope_defines
                .scroll_defs
                .iter()
                .map(|(id, v)| Token::Scroll(*id, v.clone())),
        );

        def_tokens.extend(
            self.scope_defines
                .speed_defs
                .iter()
                .map(|(id, v)| Token::Speed(*id, v.clone())),
        );

        def_tokens.extend(
            self.others
                .texts
                .iter()
                .map(|(id, text)| Token::Text(*id, text.as_str())),
        );

        def_tokens.extend(
            self.scope_defines
                .exrank_defs
                .iter()
                .map(|(id, exrank)| Token::ExRank(*id, exrank.judge_level)),
        );

        #[cfg(feature = "minor-command")]
        {
            def_tokens.extend(
                self.scope_defines
                    .exwav_defs
                    .iter()
                    .map(|(id, def)| Token::ExWav {
                        id: *id,
                        pan: def.pan,
                        volume: def.volume,
                        frequency: def.frequency,
                        path: def.path.as_ref(),
                    }),
            );

            def_tokens.extend(
                self.scope_defines
                    .wavcmd_events
                    .values()
                    .map(|ev| Token::WavCmd(*ev)),
            );

            def_tokens.extend(
                self.scope_defines
                    .atbga_defs
                    .iter()
                    .map(|(id, def)| Token::AtBga {
                        id: *id,
                        source_bmp: def.source_bmp,
                        trim_top_left: def.trim_top_left.into(),
                        trim_size: def.trim_size.into(),
                        draw_point: def.draw_point.into(),
                    }),
            );

            def_tokens.extend(
                self.scope_defines
                    .bga_defs
                    .iter()
                    .map(|(id, def)| Token::Bga {
                        id: *id,
                        source_bmp: def.source_bmp,
                        trim_top_left: def.trim_top_left.into(),
                        trim_bottom_right: def.trim_bottom_right.into(),
                        draw_point: def.draw_point.into(),
                    }),
            );

            def_tokens.extend(
                self.scope_defines
                    .argb_defs
                    .iter()
                    .map(|(id, argb)| Token::Argb(*id, *argb)),
            );
        }

        tokens.extend(def_tokens);

        // Resources - Use iterator chains to efficiently collect resource tokens
        let mut resource_tokens: Vec<Token> = Vec::new();

        // Add basic resource tokens
        if let Some(path_root) = self.notes.wav_path_root.as_ref() {
            resource_tokens.push(Token::PathWav(path_root.as_ref()));
        }

        if let Some(poor_bmp) = self.graphics.poor_bmp.as_ref()
            && !poor_bmp.as_path().as_os_str().is_empty()
        {
            resource_tokens.push(Token::Bmp(None, poor_bmp.as_ref()));
        }

        if let Some(video_file) = self.graphics.video_file.as_ref()
            && !video_file.as_path().as_os_str().is_empty()
        {
            resource_tokens.push(Token::VideoFile(video_file.as_ref()));
        }

        #[cfg(feature = "minor-command")]
        {
            if let Some(colors) = self.graphics.video_colors {
                resource_tokens.push(Token::VideoColors(colors));
            }
            if let Some(delay) = self.graphics.video_dly.as_ref() {
                resource_tokens.push(Token::VideoDly(delay.clone()));
            }
            if let Some(fps) = self.graphics.video_fs.as_ref() {
                resource_tokens.push(Token::VideoFs(fps.clone()));
            }
        }

        // Collect WAV and BMP file tokens using iterator chains
        resource_tokens.extend(
            self.notes
                .wav_files
                .iter()
                .filter(|(_, path)| !path.as_path().as_os_str().is_empty())
                .map(|(id, path)| Token::Wav(*id, path.as_ref())),
        );

        resource_tokens.extend(
            self.graphics
                .bmp_files
                .iter()
                .filter(|(_, bmp)| !bmp.file.as_path().as_os_str().is_empty())
                .map(|(id, bmp)| {
                    if bmp.transparent_color == Argb::default() {
                        Token::Bmp(Some(*id), bmp.file.as_ref())
                    } else {
                        Token::ExBmp(*id, bmp.transparent_color, bmp.file.as_ref())
                    }
                }),
        );

        tokens.extend(resource_tokens);

        // Collect late definition tokens and message tokens
        let mut late_def_tokens: Vec<Token<'a>> = Vec::new();
        let mut message_tokens: Vec<Token<'a>> = Vec::new();

        // Messages: Section length - Use iterator chain to collect tokens
        message_tokens.extend(self.arrangers.section_len_changes.values().map(|obj| {
            Token::Message {
                track: obj.track,
                channel: Channel::SectionLen,
                message: Cow::Owned(obj.length.to_string()),
            }
        }));

        // Helper closures for mapping definitions
        let mut used_bpm_ids: HashSet<ObjId> =
            self.scope_defines.bpm_defs.keys().copied().collect();
        let used_stop_ids: HashSet<ObjId> = self.scope_defines.stop_defs.keys().copied().collect();
        let used_scroll_ids: HashSet<ObjId> =
            self.scope_defines.scroll_defs.keys().copied().collect();
        let used_speed_ids: HashSet<ObjId> =
            self.scope_defines.speed_defs.keys().copied().collect();
        let mut used_text_ids: HashSet<ObjId> = self.others.texts.keys().copied().collect();
        let used_exrank_ids: HashSet<ObjId> =
            self.scope_defines.exrank_defs.keys().copied().collect();

        let mut bpm_value_to_id: HashMap<Decimal, ObjId> = self
            .scope_defines
            .bpm_defs
            .iter()
            .map(|(k, v)| (v.clone(), *k))
            .collect();
        let stop_value_to_id: HashMap<Decimal, ObjId> = self
            .scope_defines
            .stop_defs
            .iter()
            .map(|(k, v)| (v.clone(), *k))
            .collect();
        let scroll_value_to_id: HashMap<Decimal, ObjId> = self
            .scope_defines
            .scroll_defs
            .iter()
            .map(|(k, v)| (v.clone(), *k))
            .collect();
        let speed_value_to_id: HashMap<Decimal, ObjId> = self
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
        let exrank_value_to_id: HashMap<JudgeLevel, ObjId> = self
            .scope_defines
            .exrank_defs
            .iter()
            .map(|(k, v)| (v.judge_level, *k))
            .collect();

        // Messages: BPM change (#xxx08 or #xxx03) - Use iterator chains for processing
        {
            // Process BPM changes and collect track-grouped data using iterator chains
            let (by_track_id, by_track_u8): (
                BTreeMap<Track, Vec<(ObjTime, ObjId)>>,
                BTreeMap<Track, Vec<(ObjTime, u8)>>,
            ) = self
                .arrangers
                .bpm_changes
                .iter()
                .map(|(&time, ev)| {
                    if let Some(&id) = bpm_value_to_id.get(&ev.bpm) {
                        return (Some((time.track(), (time, id))), None);
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
                        return (None, Some((time.track(), (time, v as u8))));
                    }

                    // otherwise, allocate new id definition
                    let new_id = alloc_id(&mut used_bpm_ids);
                    bpm_value_to_id.insert(ev.bpm.clone(), new_id);
                    late_def_tokens.push(Token::BpmChange(new_id, ev.bpm.clone()));

                    (Some((time.track(), (time, new_id))), None)
                })
                .fold(
                    (BTreeMap::new(), BTreeMap::new()),
                    |mut acc, (id_item, u8_item)| {
                        if let Some((track, item)) = id_item {
                            acc.0.entry(track).or_default().push(item);
                        }
                        if let Some((track, item)) = u8_item {
                            acc.1.entry(track).or_default().push(item);
                        }
                        acc
                    },
                );

            // Build message tokens using iterator chains
            message_tokens.extend(by_track_id.into_iter().filter_map(|(track, items)| {
                build_message_line_content(items.into_iter(), |id: &_| id.to_string()).map(
                    |message| Token::Message {
                        track,
                        channel: Channel::BpmChange,
                        message,
                    },
                )
            }));

            message_tokens.extend(by_track_u8.into_iter().filter_map(|(track, items)| {
                build_message_line_content(items.into_iter(), |value: &_| format!("{:02X}", value))
                    .map(|message| Token::Message {
                        track,
                        channel: Channel::BpmChangeU8,
                        message,
                    })
            }));
        }

        // Messages: STOP (#xxx09)
        let stop_result = build_messages_event(
            self.arrangers.stops.iter(),
            stop_value_to_id,
            used_stop_ids,
            Token::Stop,
            |ev| ev.duration.clone(),
            Channel::Stop,
        );
        late_def_tokens.extend(stop_result.late_def_tokens);
        message_tokens.extend(stop_result.message_tokens);

        // Messages: SCROLL (#xxxSC)
        let scroll_result = build_messages_event(
            self.arrangers.scrolling_factor_changes.iter(),
            scroll_value_to_id,
            used_scroll_ids,
            Token::Scroll,
            |ev| ev.factor.clone(),
            Channel::Scroll,
        );
        late_def_tokens.extend(scroll_result.late_def_tokens);
        message_tokens.extend(scroll_result.message_tokens);

        // Messages: SPEED (#xxxSP)
        let speed_result = build_messages_event(
            self.arrangers.speed_factor_changes.iter(),
            speed_value_to_id,
            used_speed_ids,
            Token::Speed,
            |ev| ev.factor.clone(),
            Channel::Speed,
        );
        late_def_tokens.extend(speed_result.late_def_tokens);
        message_tokens.extend(speed_result.message_tokens);

        // Messages: BGA changes (#xxx04/#xxx07/#xxx06/#xxx0A) - Use iterator chains
        {
            // Build track-grouped BGA data using iterator chains
            let by_track_layer: BTreeMap<(Track, (u16, u16)), Vec<(ObjTime, ObjId)>> = self
                .graphics
                .bga_changes
                .iter()
                .map(|(&time, bga)| {
                    let channel = bga.layer.to_channel();
                    let key = (time.track(), channel_sort_key(channel));
                    (key, (time, bga.id))
                })
                .fold(BTreeMap::new(), |mut acc, (key, time_id)| {
                    acc.entry(key).or_default().push(time_id);
                    acc
                });

            // Build message tokens using iterator chains
            message_tokens.extend(by_track_layer.into_iter().filter_map(
                |((track, (_r1, _r2)), items)| {
                    let channel = match _r1 {
                        0x0004 => Channel::BgaBase,
                        0x0006 => Channel::BgaPoor,
                        0x0007 => Channel::BgaLayer,
                        0x000A => Channel::BgaLayer2,
                        _ => Channel::BgaBase,
                    };
                    build_message_line_content(items.into_iter(), |id| id.to_string()).map(
                        |message| Token::Message {
                            track,
                            channel,
                            message,
                        },
                    )
                },
            ));
        }

        // Messages: BGM (#xxx01) and Notes (various #xx)
        process_bgm_note_events(&self.notes, &mut message_tokens);

        // Messages: BGM volume (#97) and KEY volume (#98) - Use iterator chains
        {
            // Build track-grouped volume data using iterator chains
            let by_track_bgm: BTreeMap<Track, Vec<(ObjTime, u8)>> = self
                .notes
                .bgm_volume_changes
                .iter()
                .map(|(&time, ev)| (time.track(), (time, ev.volume)))
                .fold(BTreeMap::new(), |mut acc, (track, time_vol)| {
                    acc.entry(track).or_default().push(time_vol);
                    acc
                });

            let by_track_key: BTreeMap<Track, Vec<(ObjTime, u8)>> = self
                .notes
                .key_volume_changes
                .iter()
                .map(|(&time, ev)| (time.track(), (time, ev.volume)))
                .fold(BTreeMap::new(), |mut acc, (track, time_vol)| {
                    acc.entry(track).or_default().push(time_vol);
                    acc
                });

            message_tokens.extend(build_messages_from_track(
                by_track_bgm
                    .into_iter()
                    .map(|(track, items)| (track, items.into_iter())),
                Channel::BgmVolume,
                |value| format!("{:02X}", value),
            ));

            message_tokens.extend(build_messages_from_track(
                by_track_key
                    .into_iter()
                    .map(|(track, items)| (track, items.into_iter())),
                Channel::KeyVolume,
                |value| format!("{:02X}", value),
            ));
        }

        // Messages: TEXT (#99) - Use iterator chains for processing
        {
            // Process text events and build track-grouped data using iterator chains
            let by_track_text: BTreeMap<Track, Vec<(ObjTime, ObjId)>> = self
                .notes
                .text_events
                .iter()
                .map(|(&time, ev)| {
                    let id = text_value_to_id
                        .get(ev.text.as_str())
                        .copied()
                        .unwrap_or_else(|| {
                            let new_id = alloc_id(&mut used_text_ids);
                            text_value_to_id.insert(ev.text.as_str(), new_id);
                            late_def_tokens.push(Token::Text(new_id, ev.text.as_str()));
                            new_id
                        });
                    (time.track(), (time, id))
                })
                .fold(BTreeMap::new(), |mut acc, (track, time_id)| {
                    acc.entry(track).or_default().push(time_id);
                    acc
                });

            message_tokens.extend(build_messages_from_track(
                by_track_text
                    .into_iter()
                    .map(|(track, items)| (track, items.into_iter())),
                Channel::Text,
                |id| id.to_string(),
            ));
        }

        let judge_result = build_messages_event(
            self.notes.judge_events.iter(),
            exrank_value_to_id,
            used_exrank_ids,
            Token::ExRank,
            |ev| ev.judge_level,
            Channel::Judge,
        );
        late_def_tokens.extend(judge_result.late_def_tokens);
        message_tokens.extend(judge_result.message_tokens);

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

/// Generic function to build message strings from time-indexed values
///
/// This function processes time-indexed values from an iterator and converts them into a formatted message string.
/// It uses iterator chains to efficiently process and sort the items, calculate timing parameters,
/// and build the final message content.
///
/// Arguments:
///     items: An iterator yielding (time, value) pairs to be processed
///     formatter: Function to format values into strings
///
/// Execution flow:
/// 1. Collect all items from the iterator into a Vec for processing
/// 2. Early return if no items provided
/// 3. Sort items by time using iterator
/// 4. Calculate least common multiple of denominators using iterator
/// 5. Build slots map by iterating over items and calculating indices
/// 6. Find the maximum index to determine message length
/// 7. Build final string by iterating over all indices and formatting values
///
/// Returns:
///     Option<Cow<str>> - Formatted message string, or None if no items provided
///
/// The function leverages Rust's iterator chains for efficient processing. The Vec allocation is necessary
/// for sorting and multiple passes over the data, but this is more efficient than multiple allocations.
fn build_message_line_content<'a, T, I, F>(items: I, formatter: F) -> Option<Cow<'a, str>>
where
    I: Iterator<Item = (ObjTime, T)>,
    F: Fn(&T) -> String,
{
    // Collect items into a Vec for processing (necessary for sorting and multiple passes)
    let mut items: Vec<_> = items.collect();

    if items.is_empty() {
        return None;
    }

    // Sort items by time and calculate parameters using iterators
    items.sort_by_key(|(t, _)| *t);

    let denom: u64 = items
        .iter()
        .map(|(t, _)| t.denominator().get())
        .reduce(|acc, d| acc.lcm(&d))
        .unwrap_or(1);

    let (last_index, slots): (u64, HashMap<u64, T>) = items
        .into_iter()
        .map(|(t, value)| {
            let idx = t.numerator() * (denom / t.denominator().get());
            (idx, value)
        })
        .fold((0u64, HashMap::new()), |mut acc, (idx, value)| {
            acc.0 = acc.0.max(idx);
            acc.1.insert(idx, value);
            acc
        });

    // Build final string using iterator chain
    let message: String = (0..=last_index)
        .map(|i| {
            slots
                .get(&i)
                .map(|value| formatter(value))
                .unwrap_or_else(|| "00".to_string())
        })
        .collect();

    Some(Cow::Owned(message))
}

/// Generic message builder for track-based messages
///
/// This function processes an iterator of track-based events and converts them into message tokens.
/// It uses iterator chains to efficiently process each track's events and filter out empty messages.
///
/// Arguments:
///     track_events: An iterator yielding (track, items) pairs where items is an iterator of (time, value) pairs
///     channel: The channel type for all messages
///     formatter: Function to format values into strings
///
/// Execution flow:
/// 1. Iterate over each track and its associated time-value pairs from the provided iterator
/// 2. For each track, attempt to build message content using the provided formatter
/// 3. Filter out tracks that result in empty messages (None)
/// 4. Convert valid messages into Token::Message and return the complete vector
///
/// Returns:
///     Vec<Token<'a>> - A vector of message tokens for all valid tracks
///
/// The function leverages Rust's iterator chains for efficient processing. This design allows for maximum
/// flexibility - callers can pass BTreeMap, HashMap, Vec, or any other Iterator without needing to build Vecs.
fn build_messages_from_track<'a, T, I, J, F>(
    track_events: I,
    channel: Channel,
    formatter: F,
) -> Vec<Token<'a>>
where
    I: Iterator<Item = (Track, J)>,
    J: Iterator<Item = (ObjTime, T)>,
    F: Fn(&T) -> String + Copy,
{
    // Use iterator chain: iterate -> filter_map -> collect
    track_events
        .filter_map(|(track, items)| {
            build_message_line_content(items, formatter).map(|message| Token::Message {
                track,
                channel,
                message,
            })
        })
        .collect()
}

/// Complete result from build_messages_event containing all processing outputs
struct EventProcessingResult<'a, K> {
    message_tokens: Vec<Token<'a>>,
    late_def_tokens: Vec<Token<'a>>,
    #[allow(unused)]
    updated_value_to_id: HashMap<K, ObjId>,
    #[allow(unused)]
    updated_used_ids: HashSet<ObjId>,
}

/// Unified generic function to process all message types with ID allocation
///
/// This function processes time-indexed events from an iterator and converts them into message tokens with automatic ID allocation.
/// It uses iterator chains to efficiently process events and handle ID allocation for new values.
///
/// Arguments:
///     events: An iterator yielding (&time, &event) pairs to process
///     value_to_id: Owned map of values to their assigned IDs (will be modified and returned)
///     used_ids: Owned set of already used IDs (will be modified and returned)
///     token_fn: Function to create definition tokens for new IDs
///     value_extractor: Function to extract key values from events
///     channel: The channel type for all messages
///
/// Execution flow:
/// 1. Iterate over each time-event pair from the provided iterator
/// 2. For each event, extract the key value using the value_extractor function
/// 3. Check if an ID already exists for this key value:
///    - If yes, use the existing ID
///    - If no, allocate a new ID, store the key->ID mapping, and create a late definition token
/// 4. Group events by track, collecting (time, id) pairs for each track
/// 5. Use build_messages_from_track to convert the track-grouped data into message tokens
/// 6. Each track's events are formatted as a single message line with proper timing
///
/// Returns:
///     EventProcessingResult - Contains:
///     - Message tokens for all processed events
///     - Late definition tokens for any new IDs created
///     - Updated value-to-ID mapping with any new allocations
///     - Updated set of used IDs with any new allocations
///
/// The function leverages Rust's iterator chains and HashMap for efficient lookups and ID allocation.
/// This design allows processing events from any source while maintaining full ownership semantics.
fn build_messages_event<'a, T, K, I, F1, F2>(
    events: I,
    mut value_to_id: HashMap<K, ObjId>,
    mut used_ids: HashSet<ObjId>,
    token_fn: F1,
    value_extractor: F2,
    channel: Channel,
) -> EventProcessingResult<'a, K>
where
    I: Iterator<Item = (&'a ObjTime, &'a T)>,
    T: Clone + 'a,
    K: std::hash::Hash + Eq + Clone,
    F1: Fn(ObjId, K) -> Token<'a>,
    F2: Fn(&T) -> K,
{
    let mut late_def_tokens: Vec<Token<'a>> = Vec::new();

    // Use iterator chain to process events and build track-grouped data
    let by_track: BTreeMap<Track, Vec<(ObjTime, ObjId)>> = events
        .map(|(&time, event)| {
            let key = value_extractor(event);
            let id = value_to_id.get(&key).copied().unwrap_or_else(|| {
                let new_id = alloc_id(&mut used_ids);
                value_to_id.insert(key.clone(), new_id);
                late_def_tokens.push(token_fn(new_id, key.clone()));
                new_id
            });
            (time.track(), (time, id))
        })
        .fold(BTreeMap::new(), |mut acc, (track, time_id)| {
            acc.entry(track).or_default().push(time_id);
            acc
        });

    let message_tokens = build_messages_from_track(
        by_track
            .into_iter()
            .map(|(track, items)| (track, items.into_iter())),
        channel,
        |id| id.to_string(),
    );

    EventProcessingResult {
        message_tokens,
        late_def_tokens,
        updated_value_to_id: value_to_id,
        updated_used_ids: used_ids,
    }
}

/// Process BGM and Note events (special case that doesn't use ID allocation)
///
/// This function processes all note events and converts them into message tokens.
/// It uses iterator chains to efficiently process each note and build the corresponding message strings.
///
/// Execution flow:
/// 1. Iterate over all notes in insertion order
/// 2. Determine channel type based on whether it's a note or BGM event
/// 3. Calculate timing parameters (denominator and numerator)
/// 4. Build message string by iterating over all time slots:
///    - Place '00' for empty slots
///    - Place formatted WAV ID for the note's actual position
/// 5. Create Token::Message and add to output vector
///
/// The function leverages Rust's iterator chains for efficient string building and memory allocation.
fn process_bgm_note_events<T: KeyLayoutMapper>(notes: &Notes<T>, message_tokens: &mut Vec<Token>) {
    let new_tokens: Vec<Token> = notes
        .all_notes_insertion_order()
        .map(|obj| {
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

            // Build message string using iterator chain
            let message: String = (0..denom)
                .map(|i| {
                    if i == num {
                        let id_str = obj.wav_id.to_string();
                        let mut chars = id_str.chars();
                        format!(
                            "{}{}",
                            chars.next().unwrap_or('0'),
                            chars.next().unwrap_or('0')
                        )
                    } else {
                        "00".to_string()
                    }
                })
                .collect();

            Token::Message {
                track,
                channel,
                message: Cow::Owned(message),
            }
        })
        .collect();

    message_tokens.extend(new_tokens);
}

/// Helper function to allocate a new ObjId
///
/// This function searches for an unused ObjId within the valid range and adds it to the used set.
/// It uses iterator chains to efficiently search through possible ID values.
///
/// Execution flow:
/// 1. Generate iterator over possible ID values (1 to 62*62-1)
/// 2. Convert each number to ObjId
/// 3. Find first ID that is not in the used set
/// 4. Insert the found ID into the used set and return it
/// 5. Return null ID if no available ID found
///
/// The function leverages Rust's iterator methods for efficient searching and early termination.
fn alloc_id(used: &mut HashSet<ObjId>) -> ObjId {
    (1..(62 * 62))
        .map(|i| ObjId::try_from(i as u16).unwrap_or_else(|_| ObjId::null()))
        .find(|id| !used.contains(id))
        .map(|id| {
            used.insert(id);
            id
        })
        .unwrap_or_else(ObjId::null)
}
