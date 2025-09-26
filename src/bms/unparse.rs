//! Unparse Bms model into Vec<Token> without duplicate parsing logic.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use fraction::{One, ToPrimitive, Zero};

use crate::bms::prelude::*;

/// Configuration for ID management in build_messages_event
struct ObjIdManager<'a, K: ?Sized> {
    value_to_id: HashMap<&'a K, ObjId>,
    used_ids: HashSet<ObjId>,
    unused_ids: VecDeque<ObjId>,
}

impl<'a, K: ?Sized> ObjIdManager<'a, K>
where
    K: std::hash::Hash + Eq,
{
    fn new(value_to_id: HashMap<&'a K, ObjId>, used_ids: HashSet<ObjId>) -> Self {
        let unused_ids: VecDeque<ObjId> = ObjId::all_values()
            .filter(|id| !used_ids.contains(id))
            .collect();

        Self {
            value_to_id,
            used_ids,
            unused_ids,
        }
    }

    fn get_or_allocate_id(
        &mut self,
        key: &'a K,
        create_token: impl Fn(ObjId, &'a K) -> Token<'a>,
    ) -> (ObjId, Option<Token<'a>>) {
        if let Some(&id) = self.value_to_id.get(key) {
            (id, None)
        } else {
            let new_id = self.unused_ids.pop_front().unwrap_or_else(ObjId::null);
            self.used_ids.insert(new_id);
            self.value_to_id.insert(key, new_id);
            let token = create_token(new_id, key);
            (new_id, Some(token))
        }
    }
}

/// Generic token generator for ObjId-based definition tokens
///
/// This struct integrates with ObjIdManager to provide centralized management
/// of definition token generation, combining ID allocation, key extraction, and token creation.
struct DefTokenGenerator<'a, Event: 'a, Key: ?Sized + 'a, TokenCreator, KeyExtractor>
where
    Key: std::hash::Hash + Eq,
    TokenCreator: Fn(ObjId, &'a Key) -> Token<'a>,
    KeyExtractor: Fn(&'a Event) -> &'a Key,
{
    id_manager: ObjIdManager<'a, Key>,
    token_creator: TokenCreator,
    key_extractor: KeyExtractor,
    _phantom: std::marker::PhantomData<&'a Event>,
}

impl<'a, Event: 'a, Key: ?Sized + 'a, TokenCreator, KeyExtractor>
    DefTokenGenerator<'a, Event, Key, TokenCreator, KeyExtractor>
where
    Key: std::hash::Hash + Eq,
    TokenCreator: Fn(ObjId, &'a Key) -> Token<'a>,
    KeyExtractor: Fn(&'a Event) -> &'a Key,
{
    /// Create a new instance with an ObjIdManager, token creator function, and key extractor
    fn new(
        id_manager: ObjIdManager<'a, Key>,
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
    fn process_event(&mut self, event: &'a Event) -> (ObjId, Option<Token<'a>>) {
        let key = (self.key_extractor)(event);
        self.id_manager.get_or_allocate_id(key, &self.token_creator)
    }

    /// Consume the generator and return the ObjIdManager
    fn into_id_manager(self) -> ObjIdManager<'a, Key> {
        self.id_manager
    }
}

/// Convenience functions for creating common definition token generators
impl<'a, Event: 'a, Key: ?Sized + 'a, TokenCreator, KeyExtractor>
    DefTokenGenerator<'a, Event, Key, TokenCreator, KeyExtractor>
where
    Key: std::hash::Hash + Eq,
    TokenCreator: Fn(ObjId, &'a Key) -> Token<'a>,
    KeyExtractor: Fn(&'a Event) -> &'a Key,
{
    /// Create a token generator with all required components
    ///
    /// This function allows creating token generators by providing
    /// an ObjIdManager, token creator function, and key extractor function
    pub fn create_generator(
        id_manager: ObjIdManager<'a, Key>,
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
        let mut needs_base62_token = false;

        // Helper function to check if ObjId needs base62
        let mut check_base62 = |id: &ObjId| {
            needs_base62_token = needs_base62_token || (!id.is_base36() && id.is_base62());
        };

        // Others section lines FIRST to preserve order equality on roundtrip
        #[cfg(feature = "minor-command")]
        {
            // Options
            if let Some(options) = self.others.options.as_ref() {
                for option in options {
                    tokens.push(Token::Option(option.as_str()));
                }
            }
            // Octave mode
            if self.others.is_octave {
                tokens.push(Token::OctFp);
            }
            // CDDA events
            for cdda in &self.others.cdda {
                tokens.push(Token::Cdda(cdda.clone()));
            }
            // Extended character events
            for extchr in &self.others.extchr_events {
                tokens.push(Token::ExtChr(extchr.clone()));
            }
            // Change options
            for (id, option) in &self.others.change_options {
                tokens.push(Token::ChangeOption(*id, option.as_str()));
            }
            // Divide property
            if let Some(divide_prop) = self.others.divide_prop.as_ref() {
                tokens.push(Token::DivideProp(divide_prop.as_str()));
            }
            // Materials path
            if let Some(materials_path) = self.others.materials_path.as_ref()
                && !materials_path.as_path().as_os_str().is_empty()
            {
                tokens.push(Token::Materials(materials_path.as_ref()));
            }
        }
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
            tokens.push(Token::Genre(genre));
        }
        if let Some(title) = self.header.title.as_deref() {
            tokens.push(Token::Title(title));
        }
        if let Some(subtitle) = self.header.subtitle.as_deref() {
            tokens.push(Token::SubTitle(subtitle));
        }
        if let Some(artist) = self.header.artist.as_deref() {
            tokens.push(Token::Artist(artist));
        }
        if let Some(sub_artist) = self.header.sub_artist.as_deref() {
            tokens.push(Token::SubArtist(sub_artist));
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

        // PoorBga mode
        #[cfg(feature = "minor-command")]
        if self.graphics.poor_bga_mode != PoorMode::default() {
            tokens.push(Token::PoorBga(self.graphics.poor_bga_mode));
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

        // Collect definition tokens using iterator chains (sorted by ID for consistent output)
        def_tokens.extend(
            self.scope_defines
                .bpm_defs
                .iter()
                .map(|(id, v)| (*id, Token::BpmChange(*id, v.clone())))
                .inspect(|(id, _)| check_base62(id))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        def_tokens.extend(
            self.scope_defines
                .stop_defs
                .iter()
                .map(|(id, v)| (*id, Token::Stop(*id, v.clone())))
                .inspect(|(id, _)| check_base62(id))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        #[cfg(feature = "minor-command")]
        def_tokens.extend(
            self.others
                .seek_events
                .iter()
                .map(|(id, v)| (*id, Token::Seek(*id, v.clone())))
                .inspect(|(id, _)| check_base62(id))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        def_tokens.extend(
            self.scope_defines
                .scroll_defs
                .iter()
                .map(|(id, v)| (*id, Token::Scroll(*id, v.clone())))
                .inspect(|(id, _)| check_base62(id))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        def_tokens.extend(
            self.scope_defines
                .speed_defs
                .iter()
                .map(|(id, v)| (*id, Token::Speed(*id, v.clone())))
                .inspect(|(id, _)| check_base62(id))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        def_tokens.extend(
            self.others
                .texts
                .iter()
                .map(|(id, text)| (*id, Token::Text(*id, text.as_str())))
                .inspect(|(id, _)| check_base62(id))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        def_tokens.extend(
            self.scope_defines
                .exrank_defs
                .iter()
                .map(|(id, exrank)| (*id, Token::ExRank(*id, exrank.judge_level)))
                .inspect(|(id, _)| check_base62(id))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        #[cfg(feature = "minor-command")]
        {
            def_tokens.extend(
                self.scope_defines
                    .exwav_defs
                    .iter()
                    .inspect(|(id, _)| check_base62(id))
                    .map(|(id, def)| {
                        (
                            *id,
                            Token::ExWav {
                                id: *id,
                                pan: def.pan,
                                volume: def.volume,
                                frequency: def.frequency,
                                path: def.path.as_ref(),
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>()
                    .into_values(),
            );

            // wavcmd_events should be sorted by wav_index for consistent output
            let mut wavcmd_events: Vec<_> = self.scope_defines.wavcmd_events.values().collect();
            wavcmd_events.sort_by_key(|ev| ev.wav_index);
            def_tokens.extend(wavcmd_events.into_iter().map(|ev| Token::WavCmd(*ev)));

            def_tokens.extend(
                self.scope_defines
                    .atbga_defs
                    .iter()
                    .inspect(|(id, def)| {
                        check_base62(id);
                        check_base62(&def.source_bmp)
                    })
                    .map(|(id, def)| {
                        (
                            *id,
                            Token::AtBga {
                                id: *id,
                                source_bmp: def.source_bmp,
                                trim_top_left: def.trim_top_left.into(),
                                trim_size: def.trim_size.into(),
                                draw_point: def.draw_point.into(),
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>()
                    .into_values(),
            );

            def_tokens.extend(
                self.scope_defines
                    .bga_defs
                    .iter()
                    .inspect(|(id, def)| {
                        check_base62(id);
                        check_base62(&def.source_bmp)
                    })
                    .map(|(id, def)| {
                        (
                            *id,
                            Token::Bga {
                                id: *id,
                                source_bmp: def.source_bmp,
                                trim_top_left: def.trim_top_left.into(),
                                trim_bottom_right: def.trim_bottom_right.into(),
                                draw_point: def.draw_point.into(),
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>()
                    .into_values(),
            );

            def_tokens.extend(
                self.scope_defines
                    .argb_defs
                    .iter()
                    .map(|(id, argb)| (*id, Token::Argb(*id, *argb)))
                    .inspect(|(id, _)| check_base62(id))
                    .collect::<BTreeMap<_, _>>()
                    .into_values(),
            );

            // SWBGA events, sorted by ObjId for consistent output
            let mut swbga_events: Vec<_> = self.scope_defines.swbga_events.iter().collect();
            swbga_events.sort_by_key(|(id, _)| *id);
            def_tokens.extend(
                swbga_events
                    .into_iter()
                    .map(|(id, ev)| Token::SwBga(*id, ev.clone())),
            );
        }

        tokens.extend(def_tokens);

        // Resources - Use iterator chains to efficiently collect resource tokens
        let mut resource_tokens: Vec<Token> = Vec::new();

        // Add basic resource tokens
        if let Some(path_root) = self.notes.wav_path_root.as_ref() {
            resource_tokens.push(Token::PathWav(path_root.as_ref()));
        }

        #[cfg(feature = "minor-command")]
        {
            if let Some(midi_file) = self.notes.midi_file.as_ref()
                && !midi_file.as_path().as_os_str().is_empty()
            {
                resource_tokens.push(Token::MidiFile(midi_file.as_ref()));
            }
            if let Some(materials_wav) = self.notes.materials_wav.first()
                && !materials_wav.as_path().as_os_str().is_empty()
            {
                resource_tokens.push(Token::MaterialsWav(materials_wav.as_ref()));
            }
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
            if let Some(char_file) = self.graphics.char_file.as_ref()
                && !char_file.as_path().as_os_str().is_empty()
            {
                resource_tokens.push(Token::CharFile(char_file.as_ref()));
            }
            if let Some(materials_bmp) = self.graphics.materials_bmp.first()
                && !materials_bmp.as_path().as_os_str().is_empty()
            {
                resource_tokens.push(Token::MaterialsBmp(materials_bmp.as_ref()));
            }
        }

        // Collect WAV and BMP file tokens using iterator chains (sorted by ID for consistent output)
        resource_tokens.extend(
            self.notes
                .wav_files
                .iter()
                .filter(|(_, path)| !path.as_path().as_os_str().is_empty())
                .map(|(id, path)| (*id, Token::Wav(*id, path.as_ref())))
                .inspect(|(id, _)| check_base62(id))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        resource_tokens.extend(
            self.graphics
                .bmp_files
                .iter()
                .filter(|(_, bmp)| !bmp.file.as_path().as_os_str().is_empty())
                .map(|(id, bmp)| {
                    (
                        *id,
                        if bmp.transparent_color == Argb::default() {
                            Token::Bmp(Some(*id), bmp.file.as_ref())
                        } else {
                            Token::ExBmp(*id, bmp.transparent_color, bmp.file.as_ref())
                        },
                    )
                })
                .inspect(|(id, _)| check_base62(id))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        tokens.extend(resource_tokens);

        // Collect late definition tokens and message tokens
        let mut late_def_tokens: Vec<Token<'a>> = Vec::new();
        let mut message_tokens: Vec<Token<'a>> = Vec::new();

        // Messages: Section length - Use iterator chain to collect tokens (sorted by track for consistent output)
        let mut section_len_tokens: Vec<_> = self
            .arrangers
            .section_len_changes
            .values()
            .map(|obj| Token::Message {
                track: obj.track,
                channel: Channel::SectionLen,
                message: Cow::Owned(obj.length.to_string()),
            })
            .collect();
        section_len_tokens.sort_by_key(|token| match token {
            Token::Message { track, .. } => *track,
            _ => Track(0),
        });
        message_tokens.extend(section_len_tokens);

        // Helper closures for mapping definitions
        let used_bpm_ids: HashSet<ObjId> = self.scope_defines.bpm_defs.keys().copied().collect();
        let used_stop_ids: HashSet<ObjId> = self.scope_defines.stop_defs.keys().copied().collect();
        #[cfg(feature = "minor-command")]
        let used_seek_ids: HashSet<ObjId> = self.others.seek_events.keys().copied().collect();
        let used_scroll_ids: HashSet<ObjId> =
            self.scope_defines.scroll_defs.keys().copied().collect();
        let used_speed_ids: HashSet<ObjId> =
            self.scope_defines.speed_defs.keys().copied().collect();
        let used_text_ids: HashSet<ObjId> = self.others.texts.keys().copied().collect();
        let used_exrank_ids: HashSet<ObjId> =
            self.scope_defines.exrank_defs.keys().copied().collect();

        let bpm_value_to_id: HashMap<&'a Decimal, ObjId> = self
            .scope_defines
            .bpm_defs
            .iter()
            .map(|(k, v)| (v, *k))
            .collect();
        let stop_value_to_id: HashMap<&'a Decimal, ObjId> = self
            .scope_defines
            .stop_defs
            .iter()
            .map(|(k, v)| (v, *k))
            .collect();
        let scroll_value_to_id: HashMap<&'a Decimal, ObjId> = self
            .scope_defines
            .scroll_defs
            .iter()
            .map(|(k, v)| (v, *k))
            .collect();
        let speed_value_to_id: HashMap<&'a Decimal, ObjId> = self
            .scope_defines
            .speed_defs
            .iter()
            .map(|(k, v)| (v, *k))
            .collect();
        let text_value_to_id: HashMap<&'a str, ObjId> = self
            .others
            .texts
            .iter()
            .map(|(k, v)| (v.as_str(), *k))
            .collect();
        let exrank_value_to_id: HashMap<&'a JudgeLevel, ObjId> = self
            .scope_defines
            .exrank_defs
            .iter()
            .map(|(k, v)| (&v.judge_level, *k))
            .collect();
        #[cfg(feature = "minor-command")]
        let seek_value_to_id: HashMap<&'a Decimal, ObjId> = self
            .others
            .seek_events
            .iter()
            .map(|(k, v)| (v, *k))
            .collect();

        // Messages: BPM change (#xxx08 or #xxx03)
        let mut bpm_id_manager = ObjIdManager::new(bpm_value_to_id, used_bpm_ids);
        let mut bpm_message_tokens = Vec::new();

        // Split BPM changes into two types: U8 (not in value list and is u8) and others
        let mut u8_bpm_events: Vec<(&ObjTime, &BpmChangeObj)> = Vec::new();
        let mut other_bpm_events: Vec<(&ObjTime, &BpmChangeObj)> = Vec::new();

        for (time, ev) in &self.arrangers.bpm_changes {
            // Check if already defined
            if bpm_id_manager.value_to_id.contains_key(&ev.bpm) {
                // Already defined, treat as other type
                other_bpm_events.push((time, ev));
            } else
            // Not in value list, check if it's U8 type
            if ev.bpm.fract() == Decimal::zero()
                && ev.bpm >= Decimal::one()
                && ev.bpm <= Decimal::from(0xFF)
            {
                // U8 type: not in value list and is u8
                u8_bpm_events.push((time, ev));
            } else {
                // Other type: needs ID allocation
                other_bpm_events.push((time, ev));
            }
        }

        // Process U8 type BPM changes
        let EventProcessingResult {
            message_tokens: bpm_u8_message_tokens,
            needs_base62: is_base62,
            ..
        } = build_event_messages(
            u8_bpm_events.into_iter(),
            None::<DefTokenGenerator<_, (), fn(ObjId, &()) -> Token, fn(&_) -> &()>>,
            |_ev| Channel::BpmChangeU8,
            |ev, _id| {
                let u8_value = ev.bpm.to_u64().unwrap_or(1) as u8;
                MessageValue::U8(u8_value)
            },
        );
        needs_base62_token = needs_base62_token || is_base62;
        bpm_message_tokens.extend(bpm_u8_message_tokens);

        // Process other type BPM changes using build_event_messages
        let bpm_manager = ObjIdManager::new(bpm_id_manager.value_to_id, bpm_id_manager.used_ids);
        let bpm_def_generator = DefTokenGenerator::create_generator(
            bpm_manager,
            |id, bpm| Token::BpmChange(id, (*bpm).clone()),
            |ev: &BpmChangeObj| &ev.bpm,
        );
        let EventProcessingResult {
            updated_value_to_id: other_updated_value_to_id,
            updated_used_ids: other_updated_used_ids,
            late_def_tokens: other_late_def_tokens,
            message_tokens: other_message_tokens,
            needs_base62: is_base62,
        } = build_event_messages(
            other_bpm_events.into_iter(),
            Some(bpm_def_generator),
            |_ev| Channel::BpmChange,
            |_ev, id| MessageValue::ObjId(id.unwrap_or(ObjId::null())),
        );
        needs_base62_token = needs_base62_token || is_base62;

        // Update id_manager with the results
        bpm_id_manager.value_to_id = other_updated_value_to_id;
        bpm_id_manager.used_ids = other_updated_used_ids;

        late_def_tokens.extend(other_late_def_tokens);
        bpm_message_tokens.extend(other_message_tokens);

        message_tokens.extend(bpm_message_tokens);

        // Messages: STOP (#xxx09)
        let stop_manager = ObjIdManager::new(stop_value_to_id, used_stop_ids);
        let stop_def_generator = DefTokenGenerator::create_generator(
            stop_manager,
            |id, duration| Token::Stop(id, (*duration).clone()),
            |ev: &StopObj| &ev.duration,
        );
        let EventProcessingResult {
            late_def_tokens: stop_late_def_tokens,
            message_tokens: stop_message_tokens,
            needs_base62: is_base62,
            ..
        } = build_event_messages(
            self.arrangers.stops.iter(),
            Some(stop_def_generator),
            |_ev| Channel::Stop,
            |_ev, id| MessageValue::ObjId(id.unwrap_or(ObjId::null())),
        );
        needs_base62_token = needs_base62_token || is_base62;
        late_def_tokens.extend(stop_late_def_tokens);
        message_tokens.extend(stop_message_tokens);

        // Messages: SCROLL (#xxxSC)
        let scroll_manager = ObjIdManager::new(scroll_value_to_id, used_scroll_ids);
        let scroll_def_generator = DefTokenGenerator::create_generator(
            scroll_manager,
            |id, factor| Token::Scroll(id, factor.clone()),
            |ev: &ScrollingFactorObj| &ev.factor,
        );
        let EventProcessingResult {
            late_def_tokens: scroll_late_def_tokens,
            message_tokens: scroll_message_tokens,
            needs_base62: is_base62,
            ..
        } = build_event_messages(
            self.arrangers.scrolling_factor_changes.iter(),
            Some(scroll_def_generator),
            |_ev| Channel::Scroll,
            |_ev, id| MessageValue::ObjId(id.unwrap_or(ObjId::null())),
        );
        needs_base62_token = needs_base62_token || is_base62;
        late_def_tokens.extend(scroll_late_def_tokens);
        message_tokens.extend(scroll_message_tokens);

        // Messages: SPEED (#xxxSP)
        let speed_manager = ObjIdManager::new(speed_value_to_id, used_speed_ids);
        let speed_def_generator = DefTokenGenerator::create_generator(
            speed_manager,
            |id, factor| Token::Speed(id, factor.clone()),
            |ev: &SpeedObj| &ev.factor,
        );
        let EventProcessingResult {
            late_def_tokens: speed_late_def_tokens,
            message_tokens: speed_message_tokens,
            needs_base62: is_base62,
            ..
        } = build_event_messages(
            self.arrangers.speed_factor_changes.iter(),
            Some(speed_def_generator),
            |_ev| Channel::Speed,
            |_ev, id| MessageValue::ObjId(id.unwrap_or(ObjId::null())),
        );
        needs_base62_token = needs_base62_token || is_base62;
        late_def_tokens.extend(speed_late_def_tokens);
        message_tokens.extend(speed_message_tokens);

        #[cfg(feature = "minor-command")]
        {
            // STP events, sorted by time for consistent output
            let mut stp_events: Vec<_> = self.arrangers.stp_events.values().collect();
            stp_events.sort_by_key(|ev| ev.time);
            tokens.extend(stp_events.into_iter().map(|ev| Token::Stp(*ev)));
        }

        // Messages: BGA changes (#xxx04/#xxx07/#xxx06/#xxx0A)
        let EventProcessingResult {
            message_tokens: bga_message_tokens,
            needs_base62: is_base62,
            ..
        } = build_event_messages(
            self.graphics.bga_changes.iter(),
            None::<DefTokenGenerator<_, (), fn(ObjId, &'a ()) -> Token<'a>, fn(&_) -> &'a ()>>,
            |bga| bga.layer.to_channel(),
            |bga, _id| MessageValue::ObjId(bga.id),
        );
        needs_base62_token = needs_base62_token || is_base62;
        message_tokens.extend(bga_message_tokens);

        #[cfg(feature = "minor-command")]
        {
            // Messages: BGA opacity changes (#xxx0B/#xxx0C/#xxx0D/#xxx0E)
            for (layer, opacity_changes) in &self.graphics.bga_opacity_changes {
                let EventProcessingResult {
                    message_tokens: opacity_message_tokens,
                    needs_base62: is_base62,
                    ..
                } = build_event_messages(
                    opacity_changes.iter(),
                    None::<
                        DefTokenGenerator<_, (), fn(ObjId, &'a ()) -> Token<'a>, fn(&_) -> &'a ()>,
                    >,
                    |_ev| match layer {
                        BgaLayer::Base => Channel::BgaBaseOpacity,
                        BgaLayer::Poor => Channel::BgaPoorOpacity,
                        BgaLayer::Overlay => Channel::BgaLayerOpacity,
                        BgaLayer::Overlay2 => Channel::BgaLayer2Opacity,
                    },
                    |ev, _id| MessageValue::U8(ev.opacity),
                );
                needs_base62_token = needs_base62_token || is_base62;
                message_tokens.extend(opacity_message_tokens);
            }

            // Messages: BGA ARGB changes (#xxxA1/#xxxA2/#xxxA3/#xxxA4)
            for (layer, argb_changes) in &self.graphics.bga_argb_changes {
                let EventProcessingResult {
                    message_tokens: argb_message_tokens,
                    needs_base62: is_base62,
                    ..
                } = build_event_messages(
                    argb_changes.iter(),
                    None::<
                        DefTokenGenerator<_, (), fn(ObjId, &'a ()) -> Token<'a>, fn(&_) -> &'a ()>,
                    >,
                    |_ev| match layer {
                        BgaLayer::Base => Channel::BgaBaseArgb,
                        BgaLayer::Poor => Channel::BgaPoorArgb,
                        BgaLayer::Overlay => Channel::BgaLayerArgb,
                        BgaLayer::Overlay2 => Channel::BgaLayer2Argb,
                    },
                    |ev, _id| MessageValue::U8(ev.argb.alpha),
                );
                needs_base62_token = needs_base62_token || is_base62;
                message_tokens.extend(argb_message_tokens);
            }
        }

        // Messages: BGM (#xxx01) and Notes (various #xx)
        // Use build_event_messages to process note and BGM objects
        // We need to preserve the original insertion order, so we process each object individually
        let EventProcessingResult {
            message_tokens: notes_message_tokens,
            needs_base62: is_base62,
            ..
        } = build_event_messages(
            self.notes
                .all_notes_insertion_order()
                .map(|obj| (&obj.offset, obj)),
            None::<DefTokenGenerator<_, (), fn(ObjId, &()) -> Token, fn(&_) -> &()>>,
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
            |obj, _id| MessageValue::ObjId(obj.wav_id), // Message formatting: use wav_id
        );
        needs_base62_token = needs_base62_token || is_base62;

        message_tokens.extend(notes_message_tokens);

        // Messages: BGM volume (#97)
        let EventProcessingResult {
            message_tokens: bgm_volume_message_tokens,
            needs_base62: is_base62,
            ..
        } = build_event_messages(
            self.notes.bgm_volume_changes.iter(),
            None::<DefTokenGenerator<_, (), fn(ObjId, &'a ()) -> Token<'a>, fn(&_) -> &'a ()>>,
            |_ev| Channel::BgmVolume,
            |ev, _id| MessageValue::U8(ev.volume),
        );
        needs_base62_token = needs_base62_token || is_base62;
        message_tokens.extend(bgm_volume_message_tokens);

        // Messages: KEY volume (#98)
        let EventProcessingResult {
            message_tokens: key_volume_message_tokens,
            needs_base62: is_base62,
            ..
        } = build_event_messages(
            self.notes.key_volume_changes.iter(),
            None::<DefTokenGenerator<_, (), fn(ObjId, &'a ()) -> Token<'a>, fn(&_) -> &'a ()>>,
            |_ev| Channel::KeyVolume,
            |ev, _id| MessageValue::U8(ev.volume),
        );
        needs_base62_token = needs_base62_token || is_base62;
        message_tokens.extend(key_volume_message_tokens);

        // Messages: TEXT (#99)
        let text_manager = ObjIdManager::new(text_value_to_id, used_text_ids);
        let text_def_generator =
            DefTokenGenerator::create_generator(text_manager, Token::Text, |ev: &TextObj| {
                ev.text.as_str()
            });
        let EventProcessingResult {
            late_def_tokens: text_late_def_tokens,
            message_tokens: text_message_tokens,
            needs_base62: is_base62,
            ..
        } = build_event_messages(
            self.notes.text_events.iter(),
            Some(text_def_generator),
            |_ev| Channel::Text,
            |_ev, id| MessageValue::ObjId(id.unwrap_or(ObjId::null())),
        );
        needs_base62_token = needs_base62_token || is_base62;
        late_def_tokens.extend(text_late_def_tokens);
        message_tokens.extend(text_message_tokens);

        let exrank_manager = ObjIdManager::new(exrank_value_to_id, used_exrank_ids);
        let exrank_def_generator = DefTokenGenerator::create_generator(
            exrank_manager,
            |id, judge_level| Token::ExRank(id, *judge_level),
            |ev: &JudgeObj| &ev.judge_level,
        );
        let EventProcessingResult {
            late_def_tokens: judge_late_def_tokens,
            message_tokens: judge_message_tokens,
            needs_base62: is_base62,
            ..
        } = build_event_messages(
            self.notes.judge_events.iter(),
            Some(exrank_def_generator),
            |_ev| Channel::Judge,
            |_ev, id| MessageValue::ObjId(id.unwrap_or(ObjId::null())),
        );
        needs_base62_token = needs_base62_token || is_base62;
        late_def_tokens.extend(judge_late_def_tokens);
        message_tokens.extend(judge_message_tokens);

        #[cfg(feature = "minor-command")]
        {
            // Messages: SEEK (#xxx05)
            let seek_manager = ObjIdManager::new(seek_value_to_id, used_seek_ids);
            let seek_def_generator = DefTokenGenerator::create_generator(
                seek_manager,
                |id, position| Token::Seek(id, (*position).clone()),
                |ev: &SeekObj| &ev.position,
            );
            let EventProcessingResult {
                late_def_tokens: seek_late_def_tokens,
                message_tokens: seek_message_tokens,
                needs_base62: is_base62,
                ..
            } = build_event_messages(
                self.notes.seek_events.iter(),
                Some(seek_def_generator),
                |_ev| Channel::Seek,
                |_ev, id| MessageValue::ObjId(id.unwrap_or(ObjId::null())),
            );
            needs_base62_token = needs_base62_token || is_base62;
            late_def_tokens.extend(seek_late_def_tokens);
            message_tokens.extend(seek_message_tokens);

            // Messages: BGA keybound (#xxxA5)
            let EventProcessingResult {
                message_tokens: bga_keybound_message_tokens,
                needs_base62: is_base62,
                ..
            } = build_event_messages(
                self.notes.bga_keybound_events.iter(),
                None::<DefTokenGenerator<_, (), fn(ObjId, &'a ()) -> Token<'a>, fn(&_) -> &'a ()>>,
                |_ev| Channel::BgaKeybound,
                |ev, _id| MessageValue::U8(ev.event.line),
            );
            needs_base62_token = needs_base62_token || is_base62;
            message_tokens.extend(bga_keybound_message_tokens);

            // Messages: OPTION (#xxxA6)
            let EventProcessingResult {
                message_tokens: option_message_tokens,
                needs_base62: is_base62,
                ..
            } = build_event_messages(
                self.notes.option_events.iter(),
                None::<DefTokenGenerator<_, (), fn(ObjId, &'a ()) -> Token<'a>, fn(&_) -> &'a ()>>,
                |_ev| Channel::Option,
                |_ev, _id| MessageValue::U8(0), // Option events don't use values
            );
            needs_base62_token = needs_base62_token || is_base62;
            message_tokens.extend(option_message_tokens);
        }

        // Assembly: header/definitions/resources/others -> late definitions -> messages
        if !late_def_tokens.is_empty() {
            tokens.extend(late_def_tokens);
        }
        if !message_tokens.is_empty() {
            tokens.extend(message_tokens);
        }

        // Add Base62 token if needed
        if needs_base62_token {
            tokens.push(Token::Base62);
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

/// Complete result from build_messages_event containing all processing outputs
struct EventProcessingResult<'a, K: ?Sized> {
    message_tokens: Vec<Token<'a>>,
    late_def_tokens: Vec<Token<'a>>,
    updated_value_to_id: HashMap<&'a K, ObjId>,
    updated_used_ids: HashSet<ObjId>,
    needs_base62: bool,
}

/// Generic function to process message types with optional ID allocation
///
/// This function processes time-indexed events from an iterator and converts them into message tokens.
/// It supports both ID allocation mode (using DefTokenGenerator) and direct mode (without ID allocation).
///
/// # PROCESSING FLOW OVERVIEW:
/// 1. **GROUP EVENTS**: Events are grouped by track, channel, and non-strictly increasing time
/// 2. **SPLIT INTO SUBGROUPS**: Each group is further split into subgroups with stricter rules:
///    - Strictly increasing time (prevents overlaps)
///    - Consistent denominators (ensures accurate representation)
/// 3. **GENERATE TOKENS**: Each subgroup becomes one Token::Message with all events encoded
///
/// Arguments:
///     events: An iterator yielding (&time, &event) pairs to process
///     def_token_generator: Optional DefTokenGenerator for centralized ID and def token management
///     channel_mapper: Function to map events to channels
///     message_formatter: Function to format events into MessageValue
///
/// Returns:
///     EventProcessingResult containing message_tokens, late_def_tokens, and updated maps
///
/// The function leverages Rust's iterator chains for efficient processing and supports
/// both ID-based and direct value-based event processing.
fn build_event_messages<
    'a,
    Event: 'a,
    Key: 'a + ?Sized + std::hash::Hash + Eq,
    EventIterator,
    TokenCreator,
    KeyExtractor,
    ChannelMapper,
    MessageFormatter,
>(
    event_iter: EventIterator,
    def_token_generator: Option<DefTokenGenerator<'a, Event, Key, TokenCreator, KeyExtractor>>,
    channel_mapper: ChannelMapper,
    message_formatter: MessageFormatter,
) -> EventProcessingResult<'a, Key>
where
    EventIterator: Iterator<Item = (&'a ObjTime, &'a Event)>,
    TokenCreator: Fn(ObjId, &'a Key) -> Token<'a>,
    KeyExtractor: Fn(&'a Event) -> &'a Key,
    ChannelMapper: Fn(&'a Event) -> Channel,
    MessageFormatter: Fn(&'a Event, Option<ObjId>) -> MessageValue,
{
    let mut late_def_tokens: Vec<Token<'a>> = Vec::new();
    let mut id_map: HashMap<ObjTime, ObjId> = HashMap::new();
    let mut needs_base62 = false;
    let updated_value_to_id: HashMap<&'a Key, ObjId>;
    let updated_used_ids: HashSet<ObjId>;

    // Process events based on whether DefTokenGenerator is provided
    // Keep original order from event_iter instead of grouping by track/channel
    let processed_events: Vec<(ObjTime, &'a Event, Channel, Option<ObjId>)> =
        if let Some(mut generator) = def_token_generator {
            // ID allocation mode: process events with DefTokenGenerator
            let events: Vec<_> = event_iter
                .map(|(&time, event)| {
                    let (id, maybe_def_token) = generator.process_event(event);
                    if let Some(def_token) = maybe_def_token {
                        late_def_tokens.push(def_token);
                    }
                    id_map.insert(time, id);
                    // Check if this ObjId requires Base62
                    needs_base62 = needs_base62 || (!id.is_base36() && id.is_base62());
                    (time, event, channel_mapper(event), Some(id))
                })
                .collect();

            // Extract updated state from the def token generator
            let def_id_manager = generator.into_id_manager();
            updated_value_to_id = def_id_manager.value_to_id;
            updated_used_ids = def_id_manager.used_ids;

            events
        } else {
            // Direct mode: process events with direct values
            updated_value_to_id = HashMap::new();
            updated_used_ids = HashSet::new();

            event_iter
                .map(|(&time, event)| (time, event, channel_mapper(event), None))
                .collect()
        };

    // === STEP 1: GROUP EVENTS BY TRACK, CHANNEL, AND TIME ===
    // Group events by adjacent same track, channel and non-strictly increasing time
    //
    // This creates the first level of grouping where events that share:
    // - Preserve the original event iterator order
    // - Same track number
    // - Same channel type
    // - Non-strictly increasing time (last_time <= current_time)
    // ...are grouped together. This is the foundation for efficient message generation.
    let grouped_events: Vec<Vec<_>> = {
        let (mut groups, current_group) = processed_events.into_iter().fold(
            (
                Vec::<Vec<(ObjTime, &Event, Channel, Option<ObjId>)>>::new(),
                Vec::<(ObjTime, &Event, Channel, Option<ObjId>)>::new(),
            ),
            |(mut groups, mut current), (time, event, channel, id)| {
                let should_join = current
                    .last()
                    .map(|&(last_time, _last_event, last_channel, _last_id)| {
                        time.track() == last_time.track()
                            && last_channel == channel
                            && last_time <= time
                    })
                    .unwrap_or(false);

                if should_join {
                    current.push((time, event, channel, id));
                } else {
                    if !current.is_empty() {
                        groups.push(current);
                    }
                    current = vec![(time, event, channel, id)];
                }

                (groups, current)
            },
        );

        if !current_group.is_empty() {
            groups.push(current_group);
        }
        groups
    };

    // === STEP 2: SPLIT GROUPS INTO SUBGROUPS ===
    // Split each group into subgroups based on time ordering and denominator consistency
    //
    // This creates the second level of grouping with stricter rules:
    // - Not preserve the original event iterator order
    // - Time must be strictly increasing (last_time < current_time)
    // - Denominators must be the same starting from the second element
    // - First element can have 0 numerator, or the same denominator as elements after it
    //
    // The purpose is to ensure that events within a subgroup can be represented
    // in a single message string without conflicts or information loss.
    let sub_grouped_events: Vec<Vec<_>> = grouped_events
        .into_iter()
        .flat_map(|group| {
            let (mut sub_groups, current_sub_group) = group.into_iter().fold(
                (
                    Vec::<Vec<(ObjTime, &Event, Channel, Option<ObjId>)>>::new(),
                    Vec::<(ObjTime, &Event, Channel, Option<ObjId>)>::new(),
                ),
                |(mut sub_groups, mut current), (time, event, channel, id)| {
                    // Determine if current event should join the current subgroup
                    let should_join = current
                        .last()
                        .map(|&(last_time, _last_event, _last_channel, _last_id)| {
                            // SUBGROUP JOINING RULES:
                            // 1. Time must be strictly increasing (prevents overlapping events)
                            // 2. Denominators must be the same starting from the second element
                            //    - First element (current.is_empty()) can have any denominator
                            //    - Subsequent elements must match the first element's denominator
                            (last_time < time)
                                && (current.is_empty()
                                    || time.denominator() == last_time.denominator())
                        })
                        .unwrap_or(true); // Empty subgroup always accepts the first event

                    if should_join {
                        current.push((time, event, channel, id));
                    } else {
                        if !current.is_empty() {
                            sub_groups.push(current);
                        }
                        current = vec![(time, event, channel, id)];
                    }

                    (sub_groups, current)
                },
            );

            if !current_sub_group.is_empty() {
                sub_groups.push(current_sub_group);
            }
            sub_groups
        })
        .collect();

    // === STEP 3: GENERATE MESSAGE TOKENS FROM SUBGROUPS ===
    // Generate message tokens: each subgroup generates one Token::Message
    //
    // This is the final step where each subgroup is converted into a single Token::Message.
    // The process ensures that all events in a subgroup are represented in one message string
    // with correct timing and without information loss.
    let message_tokens: Vec<Token<'a>> = sub_grouped_events
        .into_iter()
        .map(|sub_group| {
            if sub_group.is_empty() {
                return Token::Message {
                    track: Track(0),
                    channel: Channel::Bgm,
                    message: Cow::Borrowed(""),
                };
            }

            // EXTRACT METADATA FROM SUBGROUP
            // All events in subgroup should have same track and channel (guaranteed by grouping logic)
            let first_event = &sub_group[0];
            let (track, channel) = (first_event.0.track(), first_event.2);

            // CALCULATE MESSAGE LENGTH
            // Find the maximum denominator to determine message length - this ensures
            // all events in the subgroup can be accurately positioned in the message string.
            // Example: if we have events at 1/4, 1/2, 3/4, we need length 4 to represent them all.
            let max_denom = sub_group
                .iter()
                .map(|&(time, _, _, _)| time.denominator_u64())
                .max()
                .unwrap_or(1);

            let message_len = max_denom as usize;
            let mut message_parts: Vec<String> = vec!["00".to_string(); message_len];

            // PLACE EVENTS IN MESSAGE STRING
            // For each event in the subgroup, calculate its exact position in the message
            // and place its value there. The time_idx calculation converts fractional time
            // to array index using the formula: (numerator * max_denom / denominator)
            for (time, event, _, id_opt) in sub_group {
                let message_value = message_formatter(event, id_opt);
                // Check if this message value contains an ObjId that requires Base62
                if let MessageValue::ObjId(id) = message_value {
                    needs_base62 = needs_base62 || (!id.is_base36() && id.is_base62());
                }
                let denom_u64 = time.denominator_u64();

                // Calculate exact position: convert fraction to index in the message array
                // Example: time=3/4, max_denom=4: (3 * 4 / 4) = 3, so place at index 3
                let time_idx = (time.numerator() * (max_denom / denom_u64)) as usize;

                // Ensure we don't go out of bounds (safety check)
                if time_idx < message_len {
                    let chars = message_value.to_chars();
                    message_parts[time_idx] = chars.iter().collect::<String>();
                }
            }

            Token::Message {
                track,
                channel,
                message: Cow::Owned(message_parts.join("")),
            }
        })
        .collect();

    EventProcessingResult {
        message_tokens,
        late_def_tokens,
        updated_value_to_id,
        updated_used_ids,
        needs_base62,
    }
}
