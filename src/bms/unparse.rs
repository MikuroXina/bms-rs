//! Unparse Bms model into Vec<Token> without duplicate parsing logic.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};

use fraction::{One, ToPrimitive, Zero};
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
        let mut text_value_to_id: HashMap<&'a str, ObjId> = self
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

        // Messages: BPM change (#xxx08 or #xxx03)
        message_tokens.extend(build_bpm_change_messages(
            self,
            &mut bpm_value_to_id,
            &mut used_bpm_ids,
            &mut late_def_tokens,
        ));

        // Messages: STOP (#xxx09)
        let stop_result = build_messages_event(
            self.arrangers.stops.iter(),
            IdManager::new(stop_value_to_id, used_stop_ids),
            Token::Stop,
            |ev| ev.duration.clone(),
            |_ev| Channel::Stop,
            |id| MessageValue::ObjId(*id),
        );
        late_def_tokens.extend(stop_result.late_def_tokens);
        message_tokens.extend(stop_result.message_tokens);

        // Messages: SCROLL (#xxxSC)
        let scroll_result = build_messages_event(
            self.arrangers.scrolling_factor_changes.iter(),
            IdManager::new(scroll_value_to_id, used_scroll_ids),
            Token::Scroll,
            |ev| ev.factor.clone(),
            |_ev| Channel::Scroll,
            |id| MessageValue::ObjId(*id),
        );
        late_def_tokens.extend(scroll_result.late_def_tokens);
        message_tokens.extend(scroll_result.message_tokens);

        // Messages: SPEED (#xxxSP)
        let speed_result = build_messages_event(
            self.arrangers.speed_factor_changes.iter(),
            IdManager::new(speed_value_to_id, used_speed_ids),
            Token::Speed,
            |ev| ev.factor.clone(),
            |_ev| Channel::Speed,
            |id| MessageValue::ObjId(*id),
        );
        late_def_tokens.extend(speed_result.late_def_tokens);
        message_tokens.extend(speed_result.message_tokens);

        // Messages: BGA changes (#xxx04/#xxx07/#xxx06/#xxx0A)
        message_tokens.extend(build_bga_change_messages(self));

        // Messages: BGM (#xxx01) and Notes (various #xx)
        message_tokens.extend(build_note_messages(self));

        // Messages: BGM volume (#97)
        message_tokens.extend(build_bgm_volume_messages(self));

        // Messages: KEY volume (#98)
        message_tokens.extend(build_key_volume_messages(self));

        // Messages: TEXT (#99)
        message_tokens.extend(build_text_messages(
            self,
            &mut text_value_to_id,
            &mut used_text_ids,
            &mut late_def_tokens,
        ));

        let judge_result = build_messages_event(
            self.notes.judge_events.iter(),
            IdManager::new(exrank_value_to_id, used_exrank_ids),
            Token::ExRank,
            |ev| ev.judge_level,
            |_ev| Channel::Judge,
            |id| MessageValue::ObjId(*id),
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

#[allow(dead_code)]
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

/// Generic message builder for track-based messages
///
/// This function processes an iterator of track-based events and converts them into message tokens.
/// It uses iterator chains to efficiently process each track's events and filter out empty messages.
///
/// Arguments:
///     track_events: An iterator yielding (track, items) pairs where items is an iterator of (time, event) pairs
///     channel_mapper: Function to map events to channels (allows same event type to map to different channels)
///     message_formatter: Function to format events into strings
///
/// Returns:
///     Vec<Token<'a>> - A vector of message tokens for all valid tracks
///
/// The function leverages Rust's iterator chains for efficient processing. This design allows for maximum
/// flexibility - callers can pass BTreeMap, HashMap, Vec, or any other Iterator without needing to build Vecs.
/// The channel_mapper function allows the same value type to be converted to different channels.
fn build_messages_from_track<
    'a,
    Event,
    EventIterator,
    TrackEventIterator,
    ChannelMapper,
    MessageFormatter,
>(
    track_events: EventIterator,
    channel_mapper: ChannelMapper,
    message_formatter: MessageFormatter,
) -> Vec<Token<'a>>
where
    EventIterator: Iterator<Item = (Track, TrackEventIterator)>,
    TrackEventIterator: Iterator<Item = (ObjTime, Event)>,
    ChannelMapper: Fn(&Event) -> Channel + Copy,
    MessageFormatter: Fn(&Event) -> MessageValue + Copy,
{
    track_events
        .flat_map(|(track, items)| {
            // Collect items into a Vec for processing
            let mut items_vec: Vec<_> = items.collect();

            if items_vec.is_empty() {
                return Vec::new();
            }

            // Step 1: Sort items by time
            items_vec.sort_by_key(|(t, _)| *t);

            // Step 2: Calculate least common multiple of denominators
            let denom: u64 = items_vec
                .iter()
                .map(|(t, _)| t.denominator().get())
                .reduce(|acc, d| acc.lcm(&d))
                .unwrap_or(1);

            // Step 3: Group items by time slot and channel to handle multiple values at same time
            let mut time_channel_groups: BTreeMap<(u64, Channel), Vec<Event>> = BTreeMap::new();
            for (t, value) in items_vec {
                let idx = t.numerator() * (denom / t.denominator().get());
                let channel = channel_mapper(&value);
                time_channel_groups
                    .entry((idx, channel))
                    .or_default()
                    .push(value);
            }

            // Step 4: Generate tokens for each time-channel group
            time_channel_groups
                .into_iter()
                .flat_map(|((time_idx, channel), values)| {
                    // Create a separate token for each value at this time slot and channel
                    values.into_iter().map(move |value| {
                        // Create message string with value at position and 00s elsewhere
                        let mut message_parts = Vec::new();
                        for i in 0..denom {
                            if i == time_idx {
                                let msg_value = message_formatter(&value);
                                let chars = msg_value.to_chars();
                                message_parts.push(chars.iter().collect::<String>());
                            } else {
                                message_parts.push("00".to_string());
                            }
                        }
                        Token::Message {
                            track,
                            channel,
                            message: Cow::Owned(message_parts.join("")),
                        }
                    })
                })
                .collect::<Vec<_>>()
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

/// Configuration for ID management in build_messages_event
struct IdManager<K> {
    value_to_id: HashMap<K, ObjId>,
    used_ids: HashSet<ObjId>,
}

impl<K> IdManager<K>
where
    K: std::hash::Hash + Eq + Clone,
{
    fn new(value_to_id: HashMap<K, ObjId>, used_ids: HashSet<ObjId>) -> Self {
        Self {
            value_to_id,
            used_ids,
        }
    }

    fn get_or_allocate_id<'a>(
        &mut self,
        key: K,
        create_token: impl Fn(ObjId, K) -> Token<'a>,
    ) -> (ObjId, Option<Token<'a>>) {
        if let Some(&id) = self.value_to_id.get(&key) {
            (id, None)
        } else {
            let new_id = alloc_id(&mut self.used_ids);
            self.value_to_id.insert(key.clone(), new_id);
            let token = create_token(new_id, key);
            (new_id, Some(token))
        }
    }
}

/// Unified generic function to process all message types with ID allocation
///
/// This function processes time-indexed events from an iterator and converts them into message tokens with automatic ID allocation.
/// It uses iterator chains to efficiently process events and handle ID allocation for new values.
///
/// Arguments:
///     events: An iterator yielding (&time, &event) pairs to process
///     id_manager: Manager for ID allocation and tracking (will be modified and returned)
///     def_token_creator: Function to create definition tokens for new IDs
///     key_extractor: Function to extract key values from events
///     channel_mapper: Function to map events to channels (allows same event type to map to different channels)
///     message_formatter: Function to format IDs into message strings
///
/// Execution flow:
/// 1. Iterate over each time-event pair from the provided iterator
/// 2. For each event, extract the key value using the key_extractor function
/// 3. Check if an ID already exists for this key value:
///    - If yes, use the existing ID
///    - If no, allocate a new ID, store the key->ID mapping, and create a late definition token
/// 4. Group events by track and channel, collecting (time, id) pairs for each track-channel combination
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
/// The channel_mapper function allows the same event type to be converted to different channels.
fn build_messages_event<
    'a,
    Event,
    Key,
    EventIterator,
    DefinitionTokenCreator,
    KeyExtractor,
    ChannelMapper,
    MessageFormatter,
>(
    event_iter: EventIterator,
    mut id_manager: IdManager<Key>,
    def_token_creator: DefinitionTokenCreator,
    key_extractor: KeyExtractor,
    channel_mapper: ChannelMapper,
    message_formatter: MessageFormatter,
) -> EventProcessingResult<'a, Key>
where
    EventIterator: Iterator<Item = (&'a ObjTime, &'a Event)>,
    Event: Clone + 'a,
    Key: std::hash::Hash + Eq + Clone,
    DefinitionTokenCreator: Fn(ObjId, Key) -> Token<'a>,
    KeyExtractor: Fn(&Event) -> Key,
    ChannelMapper: Fn(&Event) -> Channel,
    MessageFormatter: Fn(&ObjId) -> MessageValue,
{
    let mut late_def_tokens: Vec<Token<'a>> = Vec::new();

    // Use iterator chain to process events and build track-channel-grouped data
    let by_track_channel: BTreeMap<(Track, Channel), Vec<(ObjTime, ObjId)>> = event_iter
        .map(|(&time, event)| {
            let key = key_extractor(event);
            let (id, maybe_token) = id_manager.get_or_allocate_id(key.clone(), &def_token_creator);
            if let Some(token) = maybe_token {
                late_def_tokens.push(token);
            }
            let channel = channel_mapper(event);
            ((time.track(), channel), (time, id))
        })
        .fold(BTreeMap::new(), |mut acc, ((track, channel), time_id)| {
            acc.entry((track, channel)).or_default().push(time_id);
            acc
        });

    let message_tokens = build_messages_from_track(
        by_track_channel
            .into_iter()
            .map(|((track, channel), items)| {
                // Group by track, but keep channel information in the items
                (
                    track,
                    items
                        .into_iter()
                        .map(move |(time, id)| (time, (channel, id))),
                )
            }),
        |(channel, _id)| *channel,
        |(_channel, id)| message_formatter(id),
    );

    EventProcessingResult {
        message_tokens,
        late_def_tokens,
        updated_value_to_id: id_manager.value_to_id,
        updated_used_ids: id_manager.used_ids,
    }
}

/// Helper function to build BPM change messages
fn build_bpm_change_messages<'a, T: KeyLayoutMapper>(
    bms: &'a Bms<T>,
    bpm_value_to_id: &mut HashMap<Decimal, ObjId>,
    used_bpm_ids: &mut HashSet<ObjId>,
    late_def_tokens: &mut Vec<Token<'a>>,
) -> Vec<Token<'a>> {
    // Process BPM changes using the build_messages_from_track function
    // First, collect all BPM changes and determine their channels and values
    let bpm_events: Vec<(ObjTime, Channel, String)> = bms
        .arrangers
        .bpm_changes
        .iter()
        .map(|(&time, ev)| {
            // Check if already defined
            if let Some(&id) = bpm_value_to_id.get(&ev.bpm) {
                return (time, Channel::BpmChange, id.to_string());
            }

            // Try to treat as u8 bpm
            if ev.bpm.fract() == Decimal::zero()
                && ev.bpm >= Decimal::one()
                && ev.bpm <= Decimal::from(0xFF)
            {
                let u8_value = ev.bpm.to_u64().expect("filtered bpm should be u64") as u8;
                return (time, Channel::BpmChangeU8, format!("{:02X}", u8_value));
            }

            // Otherwise, allocate new id definition
            let new_id = alloc_id(used_bpm_ids);
            bpm_value_to_id.insert(ev.bpm.clone(), new_id);
            late_def_tokens.push(Token::BpmChange(new_id, ev.bpm.clone()));
            (time, Channel::BpmChange, new_id.to_string())
        })
        .collect();

    // Group by track and channel
    let by_track_channel: BTreeMap<(Track, Channel), Vec<(ObjTime, String)>> = bpm_events
        .into_iter()
        .map(|(time, channel, value)| ((time.track(), channel), (time, value)))
        .fold(
            BTreeMap::new(),
            |mut acc, ((track, channel), time_value)| {
                acc.entry((track, channel)).or_default().push(time_value);
                acc
            },
        );

    // Build message tokens using the modified function
    build_messages_from_track(
        by_track_channel
            .into_iter()
            .map(|((track, channel), items)| {
                (
                    track,
                    items
                        .into_iter()
                        .map(move |(time, value)| (time, (channel, value))),
                )
            }),
        |(channel, _value)| *channel,
        |(_channel, value)| {
            let mut chars = value.chars();
            let char_array = [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')];
            // Try to parse as ObjId first, fallback to u8
            match ObjId::try_from(char_array) {
                Ok(obj_id) => MessageValue::ObjId(obj_id),
                Err(_) => MessageValue::U8(char_array[0].to_digit(16).unwrap_or(0) as u8),
            }
        },
    )
}

/// Helper function to build BGA change messages
fn build_bga_change_messages<'a, T: KeyLayoutMapper>(bms: &'a Bms<T>) -> Vec<Token<'a>> {
    // Build track-grouped BGA data using the modified function
    let by_track_channel: BTreeMap<(Track, Channel), Vec<(ObjTime, ObjId)>> = bms
        .graphics
        .bga_changes
        .iter()
        .map(|(&time, bga)| {
            let channel = bga.layer.to_channel();
            ((time.track(), channel), (time, bga.id))
        })
        .fold(BTreeMap::new(), |mut acc, ((track, channel), time_id)| {
            acc.entry((track, channel)).or_default().push(time_id);
            acc
        });

    // Build message tokens using the modified function
    build_messages_from_track(
        by_track_channel
            .into_iter()
            .map(|((track, channel), items)| {
                (
                    track,
                    items
                        .into_iter()
                        .map(move |(time, id)| (time, (channel, id))),
                )
            }),
        |(channel, _id)| *channel,
        |(_channel, id)| MessageValue::ObjId(*id),
    )
}

/// Helper function to build note and BGM messages
fn build_note_messages<'a, T: KeyLayoutMapper>(bms: &'a Bms<T>) -> Vec<Token<'a>> {
    let mut message_tokens = Vec::new();

    // Process each note/BGM object individually to preserve multiple objects at same time/channel
    for obj in bms.notes.all_notes_insertion_order() {
        let channel = if let Some(_map) = obj.channel_id.try_into_map::<T>() {
            Channel::Note {
                channel_id: obj.channel_id,
            }
        } else {
            Channel::Bgm
        };

        let track = obj.offset.track();

        // Create a single token for this specific object
        message_tokens.extend(build_messages_from_track(
            std::iter::once((track, std::iter::once((obj.offset, obj.wav_id)))),
            |_id| channel,
            |id| MessageValue::ObjId(*id),
        ));
    }

    message_tokens
}

/// Helper function to build BGM volume messages
fn build_bgm_volume_messages<'a, T: KeyLayoutMapper>(bms: &'a Bms<T>) -> Vec<Token<'a>> {
    // Build track-grouped volume data using iterator chains
    let by_track_bgm: BTreeMap<Track, Vec<(ObjTime, u8)>> = bms
        .notes
        .bgm_volume_changes
        .iter()
        .map(|(&time, ev)| (time.track(), (time, ev.volume)))
        .fold(BTreeMap::new(), |mut acc, (track, time_vol)| {
            acc.entry(track).or_default().push(time_vol);
            acc
        });

    build_messages_from_track(
        by_track_bgm
            .into_iter()
            .map(|(track, items)| (track, items.into_iter())),
        |_value| Channel::BgmVolume,
        |value| MessageValue::U8(*value),
    )
}

/// Helper function to build KEY volume messages
fn build_key_volume_messages<'a, T: KeyLayoutMapper>(bms: &'a Bms<T>) -> Vec<Token<'a>> {
    let by_track_key: BTreeMap<Track, Vec<(ObjTime, u8)>> = bms
        .notes
        .key_volume_changes
        .iter()
        .map(|(&time, ev)| (time.track(), (time, ev.volume)))
        .fold(BTreeMap::new(), |mut acc, (track, time_vol)| {
            acc.entry(track).or_default().push(time_vol);
            acc
        });

    build_messages_from_track(
        by_track_key
            .into_iter()
            .map(|(track, items)| (track, items.into_iter())),
        |_value| Channel::KeyVolume,
        |value| MessageValue::U8(*value),
    )
}

/// Helper function to build text messages
fn build_text_messages<'a, T: KeyLayoutMapper>(
    bms: &'a Bms<T>,
    text_value_to_id: &mut HashMap<&'a str, ObjId>,
    used_text_ids: &mut HashSet<ObjId>,
    late_def_tokens: &mut Vec<Token<'a>>,
) -> Vec<Token<'a>> {
    // Process text events and build track-grouped data using iterator chains
    let by_track_text: BTreeMap<Track, Vec<(ObjTime, ObjId)>> = bms
        .notes
        .text_events
        .iter()
        .map(|(&time, ev)| {
            let id = text_value_to_id
                .get(ev.text.as_str())
                .copied()
                .unwrap_or_else(|| {
                    let new_id = alloc_id(used_text_ids);
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

    build_messages_from_track(
        by_track_text
            .into_iter()
            .map(|(track, items)| (track, items.into_iter())),
        |_id| Channel::Text,
        |id| MessageValue::ObjId(*id),
    )
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
    ObjId::all_values()
        .find(|id| !used.contains(id))
        .inspect(|id| {
            used.insert(*id);
        })
        .unwrap_or_else(ObjId::null)
}
