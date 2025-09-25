//! Unparse Bms model into Vec<Token> without duplicate parsing logic.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use fraction::{One, ToPrimitive, Zero};
use num::Integer;

use crate::bms::prelude::*;

/// Configuration for ID management in build_messages_event
struct ObjIdManager<K> {
    value_to_id: HashMap<K, ObjId>,
    used_ids: HashSet<ObjId>,
    unused_ids: VecDeque<ObjId>,
}

impl<K> ObjIdManager<K>
where
    K: std::hash::Hash + Eq + Clone,
{
    fn new(value_to_id: HashMap<K, ObjId>, used_ids: HashSet<ObjId>) -> Self {
        let unused_ids: VecDeque<ObjId> = ObjId::all_values()
            .filter(|id| !used_ids.contains(id))
            .collect();

        Self {
            value_to_id,
            used_ids,
            unused_ids,
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
            let new_id = self.unused_ids.pop_front().unwrap_or_else(ObjId::null);
            self.used_ids.insert(new_id);
            self.value_to_id.insert(key.clone(), new_id);
            let token = create_token(new_id, key);
            (new_id, Some(token))
        }
    }
}

/// Generic token generator for ObjId-based definition tokens
///
/// This struct integrates with ObjIdManager to provide centralized management
/// of definition token generation, combining ID allocation, key extraction, and token creation.
struct DefTokenGenerator<'a, Event: ?Sized, Key, TokenCreator, KeyExtractor>
where
    Key: std::hash::Hash + Eq + Clone,
    TokenCreator: Fn(ObjId, Key) -> Token<'a>,
    KeyExtractor: Fn(&Event) -> Key,
{
    id_manager: ObjIdManager<Key>,
    token_creator: TokenCreator,
    key_extractor: KeyExtractor,
    _phantom: std::marker::PhantomData<&'a Event>,
}

impl<'a, Event: ?Sized, Key, TokenCreator, KeyExtractor>
    DefTokenGenerator<'a, Event, Key, TokenCreator, KeyExtractor>
where
    Key: std::hash::Hash + Eq + Clone,
    TokenCreator: Fn(ObjId, Key) -> Token<'a>,
    KeyExtractor: Fn(&Event) -> Key,
{
    /// Create a new instance with an ObjIdManager, token creator function, and key extractor
    fn new(
        id_manager: ObjIdManager<Key>,
        token_creator: TokenCreator,
        key_extractor: KeyExtractor,
    ) -> Self {
        Self {
            id_manager,
            token_creator,
            key_extractor,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Process an event: extract key, get/allocate ID, and optionally create a definition token
    ///
    /// Returns (ObjId, Option<Token>) where the token is Some only if a new ID was allocated
    fn process_event(&mut self, event: &Event) -> (ObjId, Option<Token<'a>>) {
        let key = (self.key_extractor)(event);
        self.id_manager
            .get_or_allocate_id(key.clone(), &self.token_creator)
    }

    /// Get or allocate an ID for a key and optionally create a definition token
    ///
    /// Returns (ObjId, Option<Token>) where the token is Some only if a new ID was allocated
    fn get_or_allocate_id(&mut self, key: Key) -> (ObjId, Option<Token<'a>>) {
        self.id_manager
            .get_or_allocate_id(key.clone(), &self.token_creator)
    }

    /// Consume the generator and return the ObjIdManager
    fn into_id_manager(self) -> ObjIdManager<Key> {
        self.id_manager
    }
}

/// Convenience functions for creating common definition token generators
impl<'a, Event: ?Sized, Key, TokenCreator, KeyExtractor>
    DefTokenGenerator<'a, Event, Key, TokenCreator, KeyExtractor>
where
    Key: std::hash::Hash + Eq + Clone,
    TokenCreator: Fn(ObjId, Key) -> Token<'a>,
    KeyExtractor: Fn(&Event) -> Key,
{
    /// Create a token generator with all required components
    ///
    /// This function allows creating token generators by providing
    /// an ObjIdManager, token creator function, and key extractor function
    pub fn create_generator(
        id_manager: ObjIdManager<Key>,
        token_creator: TokenCreator,
        key_extractor: KeyExtractor,
    ) -> Self {
        Self::new(id_manager, token_creator, key_extractor)
    }
}

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
        let used_bpm_ids: HashSet<ObjId> = self.scope_defines.bpm_defs.keys().copied().collect();
        let used_stop_ids: HashSet<ObjId> = self.scope_defines.stop_defs.keys().copied().collect();
        let used_scroll_ids: HashSet<ObjId> =
            self.scope_defines.scroll_defs.keys().copied().collect();
        let used_speed_ids: HashSet<ObjId> =
            self.scope_defines.speed_defs.keys().copied().collect();
        let used_text_ids: HashSet<ObjId> = self.others.texts.keys().copied().collect();
        let used_exrank_ids: HashSet<ObjId> =
            self.scope_defines.exrank_defs.keys().copied().collect();

        let bpm_value_to_id: HashMap<Decimal, ObjId> = self
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
        let text_value_to_id: HashMap<&'a str, ObjId> = self
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
        let (bpm_messages, _updated_bpm_manager) = build_bpm_change_messages(
            self,
            ObjIdManager::new(bpm_value_to_id, used_bpm_ids),
            &mut late_def_tokens,
        );
        message_tokens.extend(bpm_messages);

        // Messages: STOP (#xxx09)
        let stop_manager = ObjIdManager::new(stop_value_to_id, used_stop_ids);
        let stop_def_generator = DefTokenGenerator::create_generator(
            stop_manager,
            |id, duration| Token::Stop(id, duration),
            |ev: &StopObj| ev.duration.clone(),
        );
        let stop_result = build_event_messages_with_def_token_generator(
            self.arrangers.stops.iter(),
            stop_def_generator,
            |_ev| Channel::Stop,
        );
        late_def_tokens.extend(stop_result.late_def_tokens);
        message_tokens.extend(stop_result.message_tokens);

        // Messages: SCROLL (#xxxSC)
        let scroll_manager = ObjIdManager::new(scroll_value_to_id, used_scroll_ids);
        let scroll_def_generator = DefTokenGenerator::create_generator(
            scroll_manager,
            |id, factor| Token::Scroll(id, factor),
            |ev: &ScrollingFactorObj| ev.factor.clone(),
        );
        let scroll_result = build_event_messages_with_def_token_generator(
            self.arrangers.scrolling_factor_changes.iter(),
            scroll_def_generator,
            |_ev| Channel::Scroll,
        );
        late_def_tokens.extend(scroll_result.late_def_tokens);
        message_tokens.extend(scroll_result.message_tokens);

        // Messages: SPEED (#xxxSP)
        let speed_manager = ObjIdManager::new(speed_value_to_id, used_speed_ids);
        let speed_def_generator = DefTokenGenerator::create_generator(
            speed_manager,
            |id, factor| Token::Speed(id, factor),
            |ev: &SpeedObj| ev.factor.clone(),
        );
        let speed_result = build_event_messages_with_def_token_generator(
            self.arrangers.speed_factor_changes.iter(),
            speed_def_generator,
            |_ev| Channel::Speed,
        );
        late_def_tokens.extend(speed_result.late_def_tokens);
        message_tokens.extend(speed_result.message_tokens);

        // Messages: BGA changes (#xxx04/#xxx07/#xxx06/#xxx0A)
        let bga_result: EventProcessingResult<'_, ()> =
            build_event_messages_without_def_token_generator(
                self.graphics.bga_changes.iter(),
                |bga| bga.layer.to_channel(),
                |bga| MessageValue::ObjId(bga.id),
            );
        message_tokens.extend(bga_result.message_tokens);

        // Messages: BGM (#xxx01) and Notes (various #xx)
        message_tokens.extend(build_note_messages(self));

        // Messages: BGM volume (#97)
        let bgm_volume_result: EventProcessingResult<'_, ()> =
            build_event_messages_without_def_token_generator(
                self.notes.bgm_volume_changes.iter(),
                |_ev| Channel::BgmVolume,
                |ev| MessageValue::U8(ev.volume),
            );
        message_tokens.extend(bgm_volume_result.message_tokens);

        // Messages: KEY volume (#98)
        let key_volume_result: EventProcessingResult<'_, ()> =
            build_event_messages_without_def_token_generator(
                self.notes.key_volume_changes.iter(),
                |_ev| Channel::KeyVolume,
                |ev| MessageValue::U8(ev.volume),
            );
        message_tokens.extend(key_volume_result.message_tokens);

        // Messages: TEXT (#99)
        let (text_messages, _updated_text_manager) = build_text_messages(
            self,
            ObjIdManager::new(text_value_to_id, used_text_ids),
            &mut late_def_tokens,
        );
        message_tokens.extend(text_messages);

        let exrank_manager = ObjIdManager::new(exrank_value_to_id, used_exrank_ids);
        let exrank_def_generator = DefTokenGenerator::create_generator(
            exrank_manager,
            |id, judge_level| Token::ExRank(id, judge_level),
            |ev: &JudgeObj| ev.judge_level,
        );
        let judge_result = build_event_messages_with_def_token_generator(
            self.notes.judge_events.iter(),
            exrank_def_generator,
            |_ev| Channel::Judge,
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
fn build_event_track_messages<
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

/// Generic function to process message types with ID allocation using DefTokenGenerator
///
/// This function processes time-indexed events from an iterator and converts them into message tokens.
/// It uses DefTokenGenerator for centralized ID allocation and definition token management.
///
/// Arguments:
///     events: An iterator yielding (&time, &event) pairs to process
///     def_token_generator: DefTokenGenerator with integrated key extraction for centralized ID and def token management
///     channel_mapper: Function to map events to channels
///
/// Returns:
///     EventProcessingResult containing message_tokens, late_def_tokens, and updated maps
///
/// The function leverages Rust's iterator chains for efficient processing and supports
/// ID-based event processing with automatic definition token generation.
fn build_event_messages_with_def_token_generator<
    'a,
    Event,
    Key,
    EventIterator,
    DefTokenCreator,
    DefKeyExtractor,
    ChannelMapper,
>(
    event_iter: EventIterator,
    mut def_token_generator: DefTokenGenerator<'a, Event, Key, DefTokenCreator, DefKeyExtractor>,
    channel_mapper: ChannelMapper,
) -> EventProcessingResult<'a, Key>
where
    EventIterator: Iterator<Item = (&'a ObjTime, &'a Event)>,
    Event: Clone + 'a,
    Key: std::hash::Hash + Eq + Clone,
    DefTokenCreator: Fn(ObjId, Key) -> Token<'a>,
    DefKeyExtractor: Fn(&Event) -> Key,
    ChannelMapper: Fn(&Event) -> Channel,
{
    let mut late_def_tokens: Vec<Token<'a>> = Vec::new();

    // ID allocation mode: process events with DefTokenGenerator
    let by_track_channel: BTreeMap<(Track, Channel), Vec<(ObjTime, ObjId)>> = event_iter
        .map(|(&time, event)| {
            let (id, maybe_def_token) = def_token_generator.process_event(event);
            if let Some(def_token) = maybe_def_token {
                late_def_tokens.push(def_token);
            }
            let channel = channel_mapper(event);
            ((time.track(), channel), (time, id))
        })
        .fold(BTreeMap::new(), |mut acc, ((track, channel), time_id)| {
            acc.entry((track, channel)).or_default().push(time_id);
            acc
        });

    // Extract updated state from the def token generator
    let def_id_manager = def_token_generator.into_id_manager();
    let updated_value_to_id = def_id_manager.value_to_id;
    let updated_used_ids = def_id_manager.used_ids;

    // Convert to unified format
    let processed_events: Vec<(Track, Vec<(ObjTime, (Channel, MessageValue))>)> = by_track_channel
        .into_iter()
        .map(|((track, channel), items)| {
            (
                track,
                items
                    .into_iter()
                    .map(|(time, id)| (time, (channel, MessageValue::ObjId(id))))
                    .collect(),
            )
        })
        .collect();

    // Single unified call to build_event_track_messages
    let message_tokens = build_event_track_messages(
        processed_events
            .into_iter()
            .map(|(track, events)| (track, events.into_iter())),
        |(channel, _msg_value)| *channel,
        |(_channel, msg_value)| *msg_value,
    );

    // Unified result building
    EventProcessingResult {
        message_tokens,
        late_def_tokens,
        updated_value_to_id,
        updated_used_ids,
    }
}

/// Generic function to process message types without ID allocation (direct mode)
///
/// This function processes time-indexed events from an iterator and converts them into message tokens.
/// It processes events with direct values without any ID allocation or definition token generation.
///
/// Arguments:
///     events: An iterator yielding (&time, &event) pairs to process
///     channel_mapper: Function to map events to channels
///     message_formatter: Function to format events into MessageValue
///
/// Returns:
///     EventProcessingResult containing message_tokens, late_def_tokens, and updated maps
///
/// The function leverages Rust's iterator chains for efficient processing and supports
/// direct value-based event processing without ID management.
fn build_event_messages_without_def_token_generator<
    'a,
    Event,
    Key,
    EventIterator,
    ChannelMapper,
    MessageFormatter,
>(
    event_iter: EventIterator,
    channel_mapper: ChannelMapper,
    message_formatter: MessageFormatter,
) -> EventProcessingResult<'a, Key>
where
    EventIterator: Iterator<Item = (&'a ObjTime, &'a Event)>,
    Event: Clone + 'a,
    Key: std::hash::Hash + Eq + Clone,
    ChannelMapper: Fn(&Event) -> Channel,
    MessageFormatter: Fn(&Event) -> MessageValue,
{
    let late_def_tokens: Vec<Token<'a>> = Vec::new();
    let updated_value_to_id: HashMap<Key, ObjId> = HashMap::new();
    let updated_used_ids: HashSet<ObjId> = HashSet::new();

    // Direct mode: process events with direct values
    let by_track_channel: BTreeMap<(Track, Channel), Vec<(ObjTime, Event)>> = event_iter
        .map(|(&time, event)| {
            let channel = channel_mapper(event);
            ((time.track(), channel), (time, event.clone()))
        })
        .fold(
            BTreeMap::new(),
            |mut acc, ((track, channel), time_event)| {
                acc.entry((track, channel)).or_default().push(time_event);
                acc
            },
        );

    // Convert to unified format
    let processed_events: Vec<(Track, Vec<(ObjTime, (Channel, MessageValue))>)> = by_track_channel
        .into_iter()
        .map(|((track, channel), items)| {
            (
                track,
                items
                    .into_iter()
                    .map(|(time, event)| (time, (channel, message_formatter(&event))))
                    .collect(),
            )
        })
        .collect();

    // Single unified call to build_event_track_messages
    let message_tokens = build_event_track_messages(
        processed_events
            .into_iter()
            .map(|(track, events)| (track, events.into_iter())),
        |(channel, _msg_value)| *channel,
        |(_channel, msg_value)| *msg_value,
    );

    // Unified result building
    EventProcessingResult {
        message_tokens,
        late_def_tokens,
        updated_value_to_id,
        updated_used_ids,
    }
}

/// Helper function to build BPM change messages
fn build_bpm_change_messages<'a, T: KeyLayoutMapper>(
    bms: &'a Bms<T>,
    mut id_manager: ObjIdManager<Decimal>,
    late_def_tokens: &mut Vec<Token<'a>>,
) -> (Vec<Token<'a>>, ObjIdManager<Decimal>) {
    let mut message_tokens = Vec::new();

    // Process BPM changes using the simplified approach
    // First, collect all BPM changes and determine their channels and values
    let bpm_events: Vec<(ObjTime, Channel, MessageValue)> = bms
        .arrangers
        .bpm_changes
        .iter()
        .map(|(&time, ev)| {
            // Check if already defined
            if let Some(&id) = id_manager.value_to_id.get(&ev.bpm) {
                return (time, Channel::BpmChange, MessageValue::ObjId(id));
            }

            // Try to treat as u8 bpm
            if ev.bpm.fract() == Decimal::zero()
                && ev.bpm >= Decimal::one()
                && ev.bpm <= Decimal::from(0xFF)
            {
                let u8_value = ev.bpm.to_u64().expect("filtered bpm should be u64") as u8;
                return (time, Channel::BpmChangeU8, MessageValue::U8(u8_value));
            }

            // Otherwise, allocate new id definition
            let (new_id, maybe_def_token) =
                id_manager.get_or_allocate_id(ev.bpm.clone(), |id, bpm| Token::BpmChange(id, bpm));
            if let Some(def_token) = maybe_def_token {
                late_def_tokens.push(def_token);
            }
            (time, Channel::BpmChange, MessageValue::ObjId(new_id))
        })
        .collect();

    // Group by track and channel
    let by_track_channel: BTreeMap<(Track, Channel), Vec<(ObjTime, MessageValue)>> = bpm_events
        .into_iter()
        .map(|(time, channel, value)| ((time.track(), channel), (time, value)))
        .fold(
            BTreeMap::new(),
            |mut acc, ((track, channel), time_value)| {
                acc.entry((track, channel)).or_default().push(time_value);
                acc
            },
        );

    // Build message tokens using the simplified function
    let tokens = build_event_track_messages(
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
        |(_channel, value)| *value,
    );

    message_tokens.extend(tokens);
    (message_tokens, id_manager)
}

/// Helper function to build note and BGM messages
fn build_note_messages<'a, T: KeyLayoutMapper>(bms: &'a Bms<T>) -> Vec<Token<'a>> {
    // Use build_event_messages to process note and BGM objects
    // We need to preserve the original insertion order, so we process each object individually
    let mut message_tokens = Vec::new();

    for obj in bms.notes.all_notes_insertion_order() {
        let result: EventProcessingResult<'_, ()> =
            build_event_messages_without_def_token_generator(
                std::iter::once((&obj.offset, obj)),
                |obj| {
                    // Channel mapping: determine channel based on channel_id
                    if let Some(_map) = obj.channel_id.try_into_map::<T>() {
                        Channel::Note {
                            channel_id: obj.channel_id,
                        }
                    } else {
                        Channel::Bgm
                    }
                },
                |obj| MessageValue::ObjId(obj.wav_id), // Message formatting: use wav_id
            );

        message_tokens.extend(result.message_tokens);
    }

    message_tokens
}

/// Helper function to build text messages
fn build_text_messages<'a, T: KeyLayoutMapper>(
    bms: &'a Bms<T>,
    mut id_manager: ObjIdManager<&'a str>,
    late_def_tokens: &mut Vec<Token<'a>>,
) -> (Vec<Token<'a>>, ObjIdManager<&'a str>) {
    // Process text events and build track-grouped data using iterator chains
    let by_track_text: BTreeMap<Track, Vec<(ObjTime, ObjId)>> = bms
        .notes
        .text_events
        .iter()
        .map(|(&time, ev)| {
            let id = id_manager
                .value_to_id
                .get(ev.text.as_str())
                .copied()
                .unwrap_or_else(|| {
                    let (new_id, maybe_token) = id_manager
                        .get_or_allocate_id(ev.text.as_str(), &|id, text| Token::Text(id, text));
                    if let Some(token) = maybe_token {
                        late_def_tokens.push(token);
                    }
                    new_id
                });
            (time.track(), (time, id))
        })
        .fold(BTreeMap::new(), |mut acc, (track, time_id)| {
            acc.entry(track).or_default().push(time_id);
            acc
        });

    let message_tokens = build_event_track_messages(
        by_track_text
            .into_iter()
            .map(|(track, items)| (track, items.into_iter())),
        |_id| Channel::Text,
        |id| MessageValue::ObjId(*id),
    );

    (message_tokens, id_manager)
}
