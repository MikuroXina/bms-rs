//! Unparse Bms model into Vec<Token> without duplicate parsing logic.

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};

use fraction::{Integer, One, ToPrimitive, Zero};

use crate::bms::prelude::*;

use crate::bms::command::ObjIdManager;

impl<T: KeyLayoutMapper> Bms<T> {
    /// Convert Bms to Vec<Token> (in conventional order: header -> definitions -> resources -> messages).
    /// - Avoid duplicate parsing: directly construct Tokens using model data;
    /// - For messages requiring ObjId, prioritize reusing existing definitions; if missing, allocate new ObjId and add definition Token (only reflected in returned Token list).
    #[must_use]
    pub fn unparse<'a>(&'a self) -> Vec<Token<'a>> {
        let mut tokens: Vec<Token<'a>> = Vec::new();

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
                tokens.push(Token::ExtChr(*extchr));
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
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        def_tokens.extend(
            self.scope_defines
                .stop_defs
                .iter()
                .map(|(id, v)| (*id, Token::Stop(*id, v.clone())))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        #[cfg(feature = "minor-command")]
        def_tokens.extend(
            self.others
                .seek_events
                .iter()
                .map(|(id, v)| (*id, Token::Seek(*id, v.clone())))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        def_tokens.extend(
            self.scope_defines
                .scroll_defs
                .iter()
                .map(|(id, v)| (*id, Token::Scroll(*id, v.clone())))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        def_tokens.extend(
            self.scope_defines
                .speed_defs
                .iter()
                .map(|(id, v)| (*id, Token::Speed(*id, v.clone())))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        def_tokens.extend(
            self.others
                .texts
                .iter()
                .map(|(id, text)| (*id, Token::Text(*id, text.as_str())))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        def_tokens.extend(
            self.scope_defines
                .exrank_defs
                .iter()
                .map(|(id, exrank)| (*id, Token::ExRank(*id, exrank.judge_level)))
                .collect::<BTreeMap<_, _>>()
                .into_values(),
        );

        #[cfg(feature = "minor-command")]
        {
            def_tokens.extend(
                self.scope_defines
                    .exwav_defs
                    .iter()
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
        let bpm_id_manager =
            ObjIdManager::from_entries(bpm_value_to_id.iter().map(|(k, v)| (*k, *v)));
        let mut bpm_message_tokens = Vec::new();

        // Split BPM changes into two types: U8 (not in value list and is u8) and others
        let mut u8_bpm_events: Vec<(&ObjTime, &BpmChangeObj)> = Vec::new();
        let mut other_bpm_events: Vec<(&ObjTime, &BpmChangeObj)> = Vec::new();

        for (time, ev) in &self.arrangers.bpm_changes {
            // Check if already defined
            if bpm_id_manager.is_assigned(&ev.bpm) {
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
            ..
        } = build_event_messages(
            u8_bpm_events.into_iter(),
            None::<(
                fn(ObjId, &()) -> Token,
                fn(&_) -> &(),
                &mut ObjIdManager<()>,
            )>,
            |_ev| Channel::BpmChangeU8,
            |ev, _id| {
                let u8_value = ev.bpm.to_u64().unwrap_or(1) as u8;
                let s = format!("{:02X}", u8_value);
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
        bpm_message_tokens.extend(bpm_u8_message_tokens);

        // Process other type BPM changes using build_event_messages
        let mut bpm_manager = bpm_id_manager;
        let EventProcessingResult {
            late_def_tokens: other_late_def_tokens,
            message_tokens: other_message_tokens,
            ..
        } = build_event_messages(
            other_bpm_events.into_iter(),
            Some((
                |id, bpm: &Decimal| Token::BpmChange(id, (*bpm).clone()),
                |ev: &'a BpmChangeObj| &ev.bpm,
                &mut bpm_manager,
            )),
            |_ev| Channel::BpmChange,
            |_ev, id| {
                let id = id.unwrap_or(ObjId::null());
                let s = id.to_string();
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );

        // Update id_manager with the results
        late_def_tokens.extend(other_late_def_tokens);
        bpm_message_tokens.extend(other_message_tokens);

        message_tokens.extend(bpm_message_tokens);

        // Messages: STOP (#xxx09)
        let mut stop_manager =
            ObjIdManager::from_entries(stop_value_to_id.iter().map(|(k, v)| (*k, *v)));
        let EventProcessingResult {
            late_def_tokens: stop_late_def_tokens,
            message_tokens: stop_message_tokens,
            ..
        } = build_event_messages(
            self.arrangers.stops.iter(),
            Some((
                |id, duration: &Decimal| Token::Stop(id, (*duration).clone()),
                |ev: &'a StopObj| &ev.duration,
                &mut stop_manager,
            )),
            |_ev| Channel::Stop,
            |_ev, id| {
                let id = id.unwrap_or(ObjId::null());
                let s = id.to_string();
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
        late_def_tokens.extend(stop_late_def_tokens);
        message_tokens.extend(stop_message_tokens);

        // Messages: SCROLL (#xxxSC)
        let mut scroll_manager =
            ObjIdManager::from_entries(scroll_value_to_id.iter().map(|(k, v)| (*k, *v)));
        let EventProcessingResult {
            late_def_tokens: scroll_late_def_tokens,
            message_tokens: scroll_message_tokens,
            ..
        } = build_event_messages(
            self.arrangers.scrolling_factor_changes.iter(),
            Some((
                |id, factor: &Decimal| Token::Scroll(id, factor.clone()),
                |ev: &'a ScrollingFactorObj| &ev.factor,
                &mut scroll_manager,
            )),
            |_ev| Channel::Scroll,
            |_ev, id| {
                let id = id.unwrap_or(ObjId::null());
                let s = id.to_string();
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
        late_def_tokens.extend(scroll_late_def_tokens);
        message_tokens.extend(scroll_message_tokens);

        // Messages: SPEED (#xxxSP)
        let mut speed_manager =
            ObjIdManager::from_entries(speed_value_to_id.iter().map(|(k, v)| (*k, *v)));
        let EventProcessingResult {
            late_def_tokens: speed_late_def_tokens,
            message_tokens: speed_message_tokens,
            ..
        } = build_event_messages(
            self.arrangers.speed_factor_changes.iter(),
            Some((
                |id, factor: &Decimal| Token::Speed(id, factor.clone()),
                |ev: &'a SpeedObj| &ev.factor,
                &mut speed_manager,
            )),
            |_ev| Channel::Speed,
            |_ev, id| {
                let id = id.unwrap_or(ObjId::null());
                let s = id.to_string();
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
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
            ..
        } = build_event_messages(
            self.graphics.bga_changes.iter(),
            None::<(
                fn(ObjId, &'a ()) -> Token<'a>,
                fn(&_) -> &'a (),
                &mut ObjIdManager<()>,
            )>,
            |bga| bga.layer.to_channel(),
            |bga, _id| {
                let s = bga.id.to_string();
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
        message_tokens.extend(bga_message_tokens);

        #[cfg(feature = "minor-command")]
        {
            // Messages: BGA opacity changes (#xxx0B/#xxx0C/#xxx0D/#xxx0E)
            for (layer, opacity_changes) in &self.graphics.bga_opacity_changes {
                let EventProcessingResult {
                    message_tokens: opacity_message_tokens,
                    ..
                } = build_event_messages(
                    opacity_changes.iter(),
                    None::<(
                        fn(ObjId, &'a ()) -> Token<'a>,
                        fn(&_) -> &'a (),
                        &mut ObjIdManager<()>,
                    )>,
                    |_ev| match layer {
                        BgaLayer::Base => Channel::BgaBaseOpacity,
                        BgaLayer::Poor => Channel::BgaPoorOpacity,
                        BgaLayer::Overlay => Channel::BgaLayerOpacity,
                        BgaLayer::Overlay2 => Channel::BgaLayer2Opacity,
                    },
                    |ev, _id| {
                        let s = format!("{:02X}", ev.opacity);
                        let mut chars = s.chars();
                        [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
                    },
                );
                message_tokens.extend(opacity_message_tokens);
            }

            // Messages: BGA ARGB changes (#xxxA1/#xxxA2/#xxxA3/#xxxA4)
            for (layer, argb_changes) in &self.graphics.bga_argb_changes {
                let EventProcessingResult {
                    message_tokens: argb_message_tokens,
                    ..
                } = build_event_messages(
                    argb_changes.iter(),
                    None::<(
                        fn(ObjId, &'a ()) -> Token<'a>,
                        fn(&_) -> &'a (),
                        &mut ObjIdManager<()>,
                    )>,
                    |_ev| match layer {
                        BgaLayer::Base => Channel::BgaBaseArgb,
                        BgaLayer::Poor => Channel::BgaPoorArgb,
                        BgaLayer::Overlay => Channel::BgaLayerArgb,
                        BgaLayer::Overlay2 => Channel::BgaLayer2Argb,
                    },
                    |ev, _id| {
                        let s = format!("{:02X}", ev.argb.alpha);
                        let mut chars = s.chars();
                        [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
                    },
                );
                message_tokens.extend(argb_message_tokens);
            }
        }

        // Messages: BGM (#xxx01) and Notes (various #xx)
        // Use build_event_messages to process note and BGM objects
        // We need to preserve the original insertion order, so we process each object individually
        let EventProcessingResult {
            message_tokens: notes_message_tokens,
            ..
        } = build_event_messages(
            self.notes
                .all_notes_insertion_order()
                .map(|obj| (&obj.offset, obj)),
            None::<(
                fn(ObjId, &()) -> Token,
                fn(&_) -> &(),
                &mut ObjIdManager<()>,
            )>,
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
            |obj, _id| {
                let s = obj.wav_id.to_string();
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            }, // Message formatting: use wav_id
        );

        message_tokens.extend(notes_message_tokens);

        // Messages: BGM volume (#97)
        let EventProcessingResult {
            message_tokens: bgm_volume_message_tokens,
            ..
        } = build_event_messages(
            self.notes.bgm_volume_changes.iter(),
            None::<(
                fn(ObjId, &'a ()) -> Token<'a>,
                fn(&_) -> &'a (),
                &mut ObjIdManager<()>,
            )>,
            |_ev| Channel::BgmVolume,
            |ev, _id| {
                let s = format!("{:02X}", ev.volume);
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
        message_tokens.extend(bgm_volume_message_tokens);

        // Messages: KEY volume (#98)
        let EventProcessingResult {
            message_tokens: key_volume_message_tokens,
            ..
        } = build_event_messages(
            self.notes.key_volume_changes.iter(),
            None::<(
                fn(ObjId, &'a ()) -> Token<'a>,
                fn(&_) -> &'a (),
                &mut ObjIdManager<()>,
            )>,
            |_ev| Channel::KeyVolume,
            |ev, _id| {
                let s = format!("{:02X}", ev.volume);
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
        message_tokens.extend(key_volume_message_tokens);

        // Messages: TEXT (#99)
        let mut text_manager =
            ObjIdManager::from_entries(text_value_to_id.iter().map(|(k, v)| (*k, *v)));
        let EventProcessingResult {
            late_def_tokens: text_late_def_tokens,
            message_tokens: text_message_tokens,
            ..
        } = build_event_messages(
            self.notes.text_events.iter(),
            Some((
                |id, text| Token::Text(id, text),
                |ev: &'a TextObj| ev.text.as_str(),
                &mut text_manager,
            )),
            |_ev| Channel::Text,
            |_ev, id| {
                let id = id.unwrap_or(ObjId::null());
                let s = id.to_string();
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
        late_def_tokens.extend(text_late_def_tokens);
        message_tokens.extend(text_message_tokens);

        let mut exrank_manager =
            ObjIdManager::from_entries(exrank_value_to_id.iter().map(|(k, v)| (*k, *v)));
        let EventProcessingResult {
            late_def_tokens: judge_late_def_tokens,
            message_tokens: judge_message_tokens,
            ..
        } = build_event_messages(
            self.notes.judge_events.iter(),
            Some((
                |id, judge_level: &JudgeLevel| Token::ExRank(id, *judge_level),
                |ev: &'a JudgeObj| &ev.judge_level,
                &mut exrank_manager,
            )),
            |_ev| Channel::Judge,
            |_ev, id| {
                let id = id.unwrap_or(ObjId::null());
                let s = id.to_string();
                let mut chars = s.chars();
                [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
            },
        );
        late_def_tokens.extend(judge_late_def_tokens);
        message_tokens.extend(judge_message_tokens);

        #[cfg(feature = "minor-command")]
        let seek_manager = {
            // Messages: SEEK (#xxx05)
            let mut seek_manager =
                ObjIdManager::from_entries(seek_value_to_id.iter().map(|(k, v)| (*k, *v)));
            let EventProcessingResult {
                late_def_tokens: seek_late_def_tokens,
                message_tokens: seek_message_tokens,
                ..
            } = build_event_messages(
                self.notes.seek_events.iter(),
                Some((
                    |id, position: &Decimal| Token::Seek(id, (*position).clone()),
                    |ev: &'a SeekObj| &ev.position,
                    &mut seek_manager,
                )),
                |_ev| Channel::Seek,
                |_ev, id| {
                    let id = id.unwrap_or(ObjId::null());
                    let s = id.to_string();
                    let mut chars = s.chars();
                    [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
                },
            );
            late_def_tokens.extend(seek_late_def_tokens);
            message_tokens.extend(seek_message_tokens);

            // Messages: BGA keybound (#xxxA5)
            let EventProcessingResult {
                message_tokens: bga_keybound_message_tokens,
                ..
            } = build_event_messages(
                self.notes.bga_keybound_events.iter(),
                None::<(
                    fn(ObjId, &'a ()) -> Token<'a>,
                    fn(&_) -> &'a (),
                    &mut ObjIdManager<()>,
                )>,
                |_ev| Channel::BgaKeybound,
                |ev, _id| {
                    let s = format!("{:02X}", ev.event.line);
                    let mut chars = s.chars();
                    [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
                },
            );
            message_tokens.extend(bga_keybound_message_tokens);

            // Messages: OPTION (#xxxA6)
            let EventProcessingResult {
                message_tokens: option_message_tokens,
                ..
            } = build_event_messages(
                self.notes.option_events.iter(),
                None::<(
                    fn(ObjId, &'a ()) -> Token<'a>,
                    fn(&_) -> &'a (),
                    &mut ObjIdManager<()>,
                )>,
                |_ev| Channel::Option,
                |_ev, _id| {
                    let s = format!("{:02X}", 0);
                    let mut chars = s.chars();
                    [chars.next().unwrap_or('0'), chars.next().unwrap_or('0')]
                }, // Option events don't use values
            );
            message_tokens.extend(option_message_tokens);

            seek_manager
        };

        // Assembly: header/definitions/resources/others -> late definitions -> messages
        if !late_def_tokens.is_empty() {
            tokens.extend(late_def_tokens);
        }
        if !message_tokens.is_empty() {
            tokens.extend(message_tokens);
        }

        // Add Base62 token if needed
        // Check if any of the used IDs require base62 (not base36 but valid base62)
        let mut all_used_ids: HashSet<ObjId> = HashSet::new();
        all_used_ids.extend(bpm_manager.get_used_ids());
        all_used_ids.extend(stop_manager.get_used_ids());
        all_used_ids.extend(scroll_manager.get_used_ids());
        all_used_ids.extend(speed_manager.get_used_ids());
        all_used_ids.extend(text_manager.get_used_ids());
        all_used_ids.extend(exrank_manager.get_used_ids());
        #[cfg(feature = "minor-command")]
        all_used_ids.extend(seek_manager.get_used_ids());

        // Add ObjIds from definition tokens that are not covered by managers
        // ExWav definitions
        #[cfg(feature = "minor-command")]
        all_used_ids.extend(self.scope_defines.exwav_defs.keys());

        // WavCmd events (use wav_index ObjId)
        #[cfg(feature = "minor-command")]
        all_used_ids.extend(
            self.scope_defines
                .wavcmd_events
                .values()
                .map(|ev| ev.wav_index),
        );

        // AtBga definitions (use both id and source_bmp ObjIds)
        #[cfg(feature = "minor-command")]
        {
            all_used_ids.extend(self.scope_defines.atbga_defs.keys());
            all_used_ids.extend(
                self.scope_defines
                    .atbga_defs
                    .values()
                    .map(|def| def.source_bmp),
            );
        }

        // Bga definitions (use both id and source_bmp ObjIds)
        #[cfg(feature = "minor-command")]
        {
            all_used_ids.extend(self.scope_defines.bga_defs.keys());
            all_used_ids.extend(
                self.scope_defines
                    .bga_defs
                    .values()
                    .map(|def| def.source_bmp),
            );
        }

        // Argb definitions
        #[cfg(feature = "minor-command")]
        all_used_ids.extend(self.scope_defines.argb_defs.keys());

        // SwBga events
        #[cfg(feature = "minor-command")]
        all_used_ids.extend(self.scope_defines.swbga_events.keys());

        // Wav resource files
        all_used_ids.extend(self.notes.wav_files.keys());

        // Bmp/ExBmp resource files
        all_used_ids.extend(self.graphics.bmp_files.keys());

        let needs_base62 = all_used_ids
            .iter()
            .any(|id| !id.is_base36() && id.is_base62());
        if needs_base62 {
            tokens.push(Token::Base62);
        }

        tokens
    }
}

/// A unit of event processing containing all necessary information for token generation
#[derive(Debug, Clone)]
struct EventUnit<'a, Event> {
    time: ObjTime,
    event: &'a Event,
    channel: Channel,
    id: Option<ObjId>,
}

/// Complete result from build_messages_event containing all processing outputs
struct EventProcessingResult<'a> {
    message_tokens: Vec<Token<'a>>,
    late_def_tokens: Vec<Token<'a>>,
}

/// Generic function to process message types with optional ID allocation
///
/// This function processes time-indexed events from an iterator and converts them into message tokens.
/// It supports both ID allocation mode (using token_creator and key_extractor) and direct mode (without ID allocation).
///
/// # PROCESSING FLOW OVERVIEW:
/// 1. **GROUP EVENTS**: Events are grouped by track, channel, and non-strictly increasing time
/// 2. **SPLIT INTO MESSAGE SEGMENTS**: Each group is further split into message segments with stricter rules:
///    - Strictly increasing time (prevents overlaps)
///    - Consistent denominators (ensures accurate representation)
/// 3. **GENERATE TOKENS**: Each message segment becomes one Token::Message with all events encoded
///
/// Arguments:
///     events: An iterator yielding (&time, &event) pairs to process
///     id_allocation: Optional tuple containing (token_creator, key_extractor, id_manager) for ID allocation mode
///     channel_mapper: Function to map events to channels
///     message_formatter: Function to format events into [char; 2]
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
    mut id_allocation: Option<(TokenCreator, KeyExtractor, &mut ObjIdManager<'a, Key>)>,
    channel_mapper: ChannelMapper,
    message_formatter: MessageFormatter,
) -> EventProcessingResult<'a>
where
    EventIterator: Iterator<Item = (&'a ObjTime, &'a Event)>,
    TokenCreator: Fn(ObjId, &'a Key) -> Token<'a>,
    KeyExtractor: Fn(&'a Event) -> &'a Key,
    ChannelMapper: Fn(&'a Event) -> Channel,
    MessageFormatter: Fn(&'a Event, Option<ObjId>) -> [char; 2],
{
    let mut late_def_tokens: Vec<Token<'a>> = Vec::new();

    // Process events based on whether id_allocation tuple is provided
    // Keep original order from event_iter instead of grouping by track/channel
    let processed_events: Vec<EventUnit<'a, Event>> = event_iter
        .map(|(&time, event)| {
            let id = if let Some((token_creator, key_extractor, manager)) = &mut id_allocation {
                // ID allocation mode: process events with token creator and key extractor
                let key = key_extractor(event);
                let (id, maybe_def_token) = manager.get_or_allocate_id(key, &*token_creator);
                if let Some(def_token) = maybe_def_token {
                    late_def_tokens.push(def_token);
                }
                Some(id)
            } else {
                None
            };
            EventUnit {
                time,
                event,
                channel: channel_mapper(event),
                id,
            }
        })
        .collect();

    // === STEP 1: GROUP EVENTS BY TRACK, CHANNEL, AND TIME ===
    // Group events by adjacent same track, channel and non-strictly increasing time
    //
    // This creates the first level of grouping where events that share:
    // - Preserve the original event iterator order
    // - Same track number
    // - Same channel type
    // - Non-strictly increasing time (last_time <= current_time)
    // ...are grouped together. This is the foundation for efficient message generation.
    let grouped_events = group_events_by_track_channel_time(processed_events);

    // === STEP 2: SPLIT GROUPS INTO MESSAGE SEGMENTS ===
    // Split each group into message segments based on time ordering and denominator consistency
    //
    // This creates the second level of grouping with stricter rules:
    // - Not preserve the original event iterator order
    // - Time must be strictly increasing (last_time < current_time)
    // - Denominators must be the same starting from the second element
    // - First element can have 0 numerator, or the same denominator as elements after it
    //
    // The purpose is to ensure that events within a message segment can be represented
    // in a single message string without conflicts or information loss.
    let message_segmented_events: Vec<Vec<_>> = grouped_events
        .into_iter()
        .flat_map(split_group_into_message_segments)
        .collect();

    // === STEP 3: GENERATE MESSAGE TOKENS FROM MESSAGE SEGMENTS ===
    // Generate message tokens: each message segment generates one Token::Message
    //
    // This is the final step where each message segment is converted into a single Token::Message.
    // The process ensures that all events in a message segment are represented in one message string
    // with correct timing and without information loss.
    let message_tokens: Vec<Token<'a>> = message_segmented_events
        .into_iter()
        .map(|message_segment| {
            convert_message_segment_to_token(message_segment, &message_formatter)
        })
        .collect();

    EventProcessingResult {
        message_tokens,
        late_def_tokens,
    }
}

/// Group events by track, channel, and non-strictly increasing time
fn group_events_by_track_channel_time<'a, Event>(
    processed_events: Vec<EventUnit<'a, Event>>,
) -> Vec<Vec<EventUnit<'a, Event>>> {
    let mut groups = Vec::new();
    let mut current_group = Vec::new();

    for event_unit in processed_events {
        let should_join = current_group
            .last()
            .map(|last_unit: &EventUnit<'a, Event>| {
                event_unit.time.track() == last_unit.time.track()
                    && last_unit.channel == event_unit.channel
                    && last_unit.time <= event_unit.time
            })
            .unwrap_or(false);

        if should_join {
            current_group.push(event_unit);
        } else {
            if !current_group.is_empty() {
                groups.push(current_group);
            }
            current_group = vec![event_unit];
        }
    }

    if !current_group.is_empty() {
        groups.push(current_group);
    }
    groups
}

/// Split a group into message segments based on time ordering and denominator consistency
fn split_group_into_message_segments<'a, Event>(
    group: Vec<EventUnit<'a, Event>>,
) -> Vec<Vec<EventUnit<'a, Event>>> {
    let mut message_segments = Vec::new();
    let mut current_message_segment = Vec::new();

    for event_unit in group {
        let should_join = current_message_segment
            .last()
            .map(|last_unit: &EventUnit<'a, Event>| {
                // MESSAGE SEGMENT JOINING RULES:
                // 1. Time must be strictly increasing (prevents overlapping events)
                // 2. Denominators must be compatible:
                //    - If current message segment is empty, accept any denominator
                //    - Otherwise, denominators must share a factor relationship (either is a factor of the other)
                //    - Reference denominator is the maximum denominator currently in the message segment
                (last_unit.time < event_unit.time)
                    && (current_message_segment.is_empty()
                        || is_denominator_compatible(&event_unit, &current_message_segment))
            })
            .unwrap_or(true); // Empty message segment always accepts the first event

        if should_join {
            current_message_segment.push(event_unit);
        } else {
            if !current_message_segment.is_empty() {
                message_segments.push(current_message_segment);
            }
            current_message_segment = vec![event_unit];
        }
    }

    if !current_message_segment.is_empty() {
        message_segments.push(current_message_segment);
    }
    message_segments
}

/// Check if an event unit's denominator is compatible with the current message segment
/// Two denominators are compatible if either is a factor of the other
fn is_denominator_compatible<'a, Event>(
    event_unit: &EventUnit<'a, Event>,
    message_segment: &[EventUnit<'a, Event>],
) -> bool {
    // Find the maximum denominator from the current message segment as reference
    let reference_denominator = message_segment
        .iter()
        .map(|event_unit| event_unit.time.denominator_u64())
        .max()
        .unwrap_or(1);

    // Check if the event unit's denominator shares a common factor relationship
    let event_denominator = event_unit.time.denominator_u64();
    reference_denominator.is_multiple_of(event_denominator)
        || event_denominator.is_multiple_of(reference_denominator)
}

/// Convert a message segment of events into a single Token::Message
fn convert_message_segment_to_token<'a, Event, MessageFormatter>(
    message_segment: Vec<EventUnit<'a, Event>>,
    message_formatter: &MessageFormatter,
) -> Token<'a>
where
    MessageFormatter: Fn(&'a Event, Option<ObjId>) -> [char; 2],
{
    if message_segment.is_empty() {
        return Token::Message {
            track: Track(0),
            channel: Channel::Bgm,
            message: Cow::Borrowed(""),
        };
    }

    // EXTRACT METADATA FROM MESSAGE SEGMENT
    // All events in message segment should have same track and channel (guaranteed by grouping logic)
    let first_event = &message_segment[0];
    let (track, channel) = (first_event.time.track(), first_event.channel);

    // CALCULATE MESSAGE LENGTH
    // Find the least common multiple (LCM) of all denominators to determine message length - this ensures
    // all events in the message segment can be accurately positioned in the message string.
    // Example: if we have events at 1/3 and 1/5, LCM(3,5)=15, so we need length 15 to represent them both accurately.
    let denominators: Vec<u64> = message_segment
        .iter()
        .map(|event_unit| event_unit.time.denominator_u64())
        .collect();
    let lcm_denom = lcm_slice(&denominators);

    let message_len = lcm_denom as usize;
    let mut message_parts: Vec<String> = vec!["00".to_string(); message_len];

    // PLACE EVENTS IN MESSAGE STRING
    // For each event in the message segment, calculate its exact position in the message
    // and place its value there. The time_idx calculation converts fractional time
    // to array index using the formula: (numerator * lcm_denom / denominator)
    for event_unit in message_segment {
        let EventUnit {
            event, id, time, ..
        } = event_unit;
        let chars = message_formatter(event, id);
        let denom_u64 = time.denominator_u64();

        // Calculate exact position: convert fraction to index in the message array
        // Example: time=3/4, lcm_denom=4: (3 * 4 / 4) = 3, so place at index 3
        // Example: time=1/3, lcm_denom=15: (1 * 15 / 3) = 5, so place at index 5
        let time_idx = (time.numerator() * (lcm_denom / denom_u64)) as usize;

        // Ensure we don't go out of bounds (safety check)
        if time_idx < message_len {
            message_parts[time_idx] = chars.iter().collect::<String>();
        }
    }

    Token::Message {
        track,
        channel,
        message: Cow::Owned(message_parts.join("")),
    }
}

/// Calculate the least common multiple (LCM) of a slice of u64 values
/// Returns 1 if the slice is empty
fn lcm_slice(denominators: &[u64]) -> u64 {
    denominators.iter().fold(1, |acc, denom| acc.lcm(denom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lcm_slice() {
        // Test empty slice
        assert_eq!(lcm_slice(&[]), 1);

        // Test single value
        assert_eq!(lcm_slice(&[3]), 3);
        assert_eq!(lcm_slice(&[5]), 5);

        // Test two values
        assert_eq!(lcm_slice(&[3, 5]), 15);
        assert_eq!(lcm_slice(&[4, 6]), 12);
        assert_eq!(lcm_slice(&[2, 4, 8]), 8);

        // Test multiple values
        assert_eq!(lcm_slice(&[2, 3, 4]), 12);
        assert_eq!(lcm_slice(&[3, 5, 7]), 105);
        assert_eq!(lcm_slice(&[6, 8, 10]), 120);

        // Test with 1
        assert_eq!(lcm_slice(&[1, 3]), 3);
        assert_eq!(lcm_slice(&[3, 1]), 3);
    }
}
