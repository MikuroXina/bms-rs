//! This module provides functionality to convert a parsed [`Bms`] object back to a vector of tokens.
//! Note that this conversion may not preserve all original formatting and comments, but it will
//! generate valid BMS tokens that represent the same musical data.

use std::collections::HashMap;

#[cfg(feature = "minor-command")]
use crate::bms::command::{graphics::Argb, minor_command::SwBgaEvent};
use crate::bms::{
    Decimal,
    command::{
        JudgeLevel, LnMode, LnType, ObjId, PoorMode, Volume,
        channel::{Channel, NoteKind},
        time::{ObjTime, Track},
    },
    lex::token::Token,
    parse::model::{
        Bms,
        obj::{BpmChangeObj, Obj},
    },
};
use num::ToPrimitive;

/// Output of the conversion from `Bms` to `Vec<Token>`.
#[derive(Debug, Clone, PartialEq)]
pub struct BmsUnparseOutput<'a> {
    /// The converted tokens.
    pub tokens: Vec<Token<'a>>,
}

impl Bms {
    /// Convert [`Bms`] to [`Vec<Token>`].
    ///
    /// This method converts a parsed [`Bms`] object back to a vector of tokens.
    /// The tokens are generated in a logical order that should produce a valid BMS file.
    ///
    /// # Example
    ///
    /// ```rust
    /// use bms_rs::bms::{parse_bms, prelude::BmsUnparseOutput};
    ///
    /// let source = "#TITLE Test Song\n#BPM 120\n#00101:0101";
    /// let bms_output = parse_bms(source);
    /// let BmsUnparseOutput { tokens } = bms_output.bms.unparse();
    /// println!("Generated {} tokens", tokens.len());
    /// ```
    pub fn unparse(&self) -> BmsUnparseOutput<'_> {
        let mut tokens = Vec::new();

        // Build reverse lookup maps for efficient ObjId finding
        let convert_params = ConvertNotesParams::new(self);

        #[cfg(feature = "minor-command")]
        let argb_reverse_map: HashMap<&Argb, ObjId> = self
            .scope_defines
            .argb_defs
            .iter()
            .map(|(obj_id, argb)| (argb, *obj_id))
            .collect();

        // Convert header information
        self.convert_header(&mut tokens);

        // Convert scope definitions
        self.convert_scope_defines(&mut tokens);

        // Convert change option definitions
        #[cfg(feature = "minor-command")]
        for (obj_id, option) in &self.others.change_options {
            tokens.push(Token::ChangeOption(*obj_id, option));
        }

        // Convert WAV path root
        if let Some(wav_path_root) = &self.notes.wav_path_root {
            tokens.push(Token::PathWav(wav_path_root));
        }

        // Convert MIDI file
        #[cfg(feature = "minor-command")]
        if let Some(midi_file) = &self.notes.midi_file {
            tokens.push(Token::MidiFile(midi_file));
        }

        // Convert materials WAV
        #[cfg(feature = "minor-command")]
        for material_wav in &self.notes.materials_wav {
            tokens.push(Token::MaterialsWav(material_wav));
        }

        // Convert arrangers (timing data)
        self.convert_arrangers(&mut tokens);

        // Convert notes and audio files
        self.convert_notes(&mut tokens, convert_params);

        // Convert graphics
        self.convert_graphics(
            &mut tokens,
            #[cfg(feature = "minor-command")]
            &argb_reverse_map,
        );

        // Convert others
        self.convert_others(&mut tokens);

        BmsUnparseOutput { tokens }
    }

    /// Converts header information to tokens.
    ///
    /// This function handles the conversion of BMS header metadata including:
    /// - Player mode (#PLAYER)
    /// - Genre information (#GENRE)
    /// - Title and subtitle (#TITLE, #SUBTITLE)
    /// - Artist and sub-artist (#ARTIST, #SUBARTIST)
    /// - Maker information (#MAKER)
    /// - Comments (#COMMENT)
    /// - Contact information (#EMAIL, #URL)
    /// - Difficulty settings (#PLAYLEVEL, #RANK, #DIFFICULTY)
    /// - Total value (#TOTAL)
    /// - Volume settings (#VOLWAV)
    /// - Long note type (#LNTYPE)
    /// - Background image (#BACKBMP)
    /// - Stage file (#STAGEFILE)
    /// - Banner image (#BANNER)
    /// - Long note mode (#LNMODE)
    /// - Preview music (#PREVIEW)
    /// - Movie file (#MOVIE)
    fn convert_header<'a>(&'a self, tokens: &mut Vec<Token<'a>>) {
        let header = &self.header;

        if let Some(player) = header.player {
            tokens.push(Token::Player(player));
        }

        if let Some(genre) = &header.genre {
            tokens.push(Token::Genre(genre));
        }

        if let Some(title) = &header.title {
            tokens.push(Token::Title(title));
        }

        if let Some(subtitle) = &header.subtitle {
            tokens.push(Token::SubTitle(subtitle));
        }

        if let Some(artist) = &header.artist {
            tokens.push(Token::Artist(artist));
        }

        if let Some(sub_artist) = &header.sub_artist {
            tokens.push(Token::SubArtist(sub_artist));
        }

        if let Some(maker) = &header.maker {
            tokens.push(Token::Maker(maker));
        }

        if let Some(comment) = &header.comment {
            for comment_line in comment {
                tokens.push(Token::Comment(comment_line));
            }
        }

        if let Some(email) = &header.email {
            tokens.push(Token::Email(email));
        }

        if let Some(url) = &header.url {
            tokens.push(Token::Url(url));
        }

        if let Some(play_level) = header.play_level {
            tokens.push(Token::PlayLevel(play_level));
        }

        if let Some(rank) = header.rank {
            tokens.push(Token::Rank(rank));
        }

        if let Some(difficulty) = header.difficulty {
            tokens.push(Token::Difficulty(difficulty));
        }

        if let Some(total) = &header.total {
            tokens.push(Token::Total(total.clone()));
        }

        if header.volume != Volume::default() {
            tokens.push(Token::VolWav(header.volume));
        }

        if header.ln_type != LnType::default() {
            match header.ln_type {
                LnType::Rdm => tokens.push(Token::LnTypeRdm),
                LnType::Mgq => tokens.push(Token::LnTypeMgq),
            }
        }

        if let Some(back_bmp) = &header.back_bmp {
            tokens.push(Token::BackBmp(back_bmp));
        }

        if let Some(stage_file) = &header.stage_file {
            tokens.push(Token::StageFile(stage_file));
        }

        if let Some(banner) = &header.banner {
            tokens.push(Token::Banner(banner));
        }

        if header.ln_mode != LnMode::default() {
            tokens.push(Token::LnMode(header.ln_mode));
        }

        if let Some(preview_music) = &header.preview_music {
            tokens.push(Token::Preview(preview_music));
        }

        if let Some(movie) = &header.movie {
            tokens.push(Token::Movie(movie));
        }
    }

    /// Converts scope definitions to tokens.
    ///
    /// This function handles the conversion of BMS scope definitions including:
    /// - BPM definitions (#BPMxx)
    /// - Stop definitions (#STOPxx)
    /// - Scroll speed definitions (#SCROLLxx)
    /// - Speed factor definitions (#SPEEDxx)
    /// - EXRANK definitions (#EXRANKxx)
    /// - EXWAV definitions (#EXWAVxx) [minor-command feature]
    /// - WAVCMD events (#WAVCMDxx) [minor-command feature]
    /// - @BGA definitions (#@BGAxx) [minor-command feature]
    /// - BGA definitions (#BGAxx) [minor-command feature]
    /// - SWBGA events (#SWBGAxx) [minor-command feature]
    /// - ARGB definitions (#ARGBxx) [minor-command feature]
    fn convert_scope_defines<'a>(&'a self, tokens: &mut Vec<Token<'a>>) {
        let scope_defines = &self.scope_defines;

        // Convert BPM definitions
        for (obj_id, bpm) in &scope_defines.bpm_defs {
            tokens.push(Token::BpmChange(*obj_id, bpm.clone()));
        }

        // Convert stop definitions
        for (obj_id, duration) in &scope_defines.stop_defs {
            tokens.push(Token::Stop(*obj_id, duration.clone()));
        }

        // Convert scroll definitions
        for (obj_id, factor) in &scope_defines.scroll_defs {
            tokens.push(Token::Scroll(*obj_id, factor.clone()));
        }

        // Convert speed definitions
        for (obj_id, factor) in &scope_defines.speed_defs {
            tokens.push(Token::Speed(*obj_id, factor.clone()));
        }

        // Convert EXRANK definitions
        for (obj_id, exrank_def) in &scope_defines.exrank_defs {
            tokens.push(Token::ExRank(*obj_id, exrank_def.judge_level));
        }

        #[cfg(feature = "minor-command")]
        {
            // Convert EXWAV definitions
            for (obj_id, exwav_def) in &scope_defines.exwav_defs {
                tokens.push(Token::ExWav {
                    id: *obj_id,
                    frequency: exwav_def.frequency,
                    pan: exwav_def.pan,
                    volume: exwav_def.volume,
                    path: &exwav_def.path,
                });
            }

            // Convert WAVCMD events
            for wavcmd_event in scope_defines.wavcmd_events.values() {
                tokens.push(Token::WavCmd(*wavcmd_event));
            }

            // Convert @BGA definitions
            for (obj_id, atbga_def) in &scope_defines.atbga_defs {
                tokens.push(Token::AtBga {
                    id: *obj_id,
                    source_bmp: atbga_def.source_bmp,
                    trim_top_left: atbga_def.trim_top_left.into(),
                    trim_size: atbga_def.trim_size.into(),
                    draw_point: atbga_def.draw_point.into(),
                });
            }

            // Convert BGA definitions
            for (obj_id, bga_def) in &scope_defines.bga_defs {
                tokens.push(Token::Bga {
                    id: *obj_id,
                    source_bmp: bga_def.source_bmp,
                    trim_top_left: bga_def.trim_top_left.into(),
                    trim_bottom_right: bga_def.trim_bottom_right.into(),
                    draw_point: bga_def.draw_point.into(),
                });
            }

            // Convert SWBGA events
            for (obj_id, swbga_event) in &scope_defines.swbga_events {
                tokens.push(Token::SwBga(*obj_id, swbga_event.clone()));
            }

            // Convert ARGB definitions
            for (obj_id, argb) in &scope_defines.argb_defs {
                tokens.push(Token::Argb(*obj_id, *argb));
            }
        }
    }

    /// Converts arrangers (timing data) to tokens.
    ///
    /// This function handles the conversion of BMS timing and arrangement data including:
    /// - Initial BPM (#BPM)
    /// - Section length changes (#SECLEN)
    /// - STP events (#STP) [minor-command feature]
    /// - Base BPM (#BASEBPM) [minor-command feature]
    ///
    /// Note: The following timing events are handled in [`Self::convert_notes`]:
    /// - BPM changes (message format)
    /// - Stops (message format)
    /// - Scrolling factor changes (message format)
    /// - Speed factor changes (message format)
    fn convert_arrangers<'a>(&'a self, tokens: &mut Vec<Token<'a>>) {
        let arrangers = &self.arrangers;

        // Convert initial BPM
        if let Some(bpm) = &arrangers.bpm {
            tokens.push(Token::Bpm(bpm.clone()));
        }

        // Convert section length changes
        for (track, section_len_obj) in &arrangers.section_len_changes {
            let channel = Channel::SectionLen;
            let message = section_len_obj.length.to_string();
            tokens.push(Token::Message {
                track: *track,
                channel,
                message: message.into(),
            });
        }

        #[cfg(feature = "minor-command")]
        {
            // Convert STP events
            for stp_event in arrangers.stp_events.values() {
                tokens.push(Token::Stp(*stp_event));
            }

            // Convert base BPM
            if let Some(base_bpm) = &arrangers.base_bpm {
                tokens.push(Token::BaseBpm(base_bpm.clone()));
            }
        }
    }

    /// Converts notes and audio files to tokens.
    ///
    /// This function handles the conversion of BMS note data and audio resources including:
    /// - WAV file definitions (#WAVxx)
    /// - BMP file definitions (#BMPxx)
    /// - BGM objects (message format)
    /// - Note objects (message format)
    /// - BPM changes (message format)
    /// - Stops (message format)
    /// - Scrolling factor changes (message format)
    /// - Speed factor changes (message format)
    /// - BGM volume changes (message format)
    /// - Key volume changes (message format)
    /// - Seek events (message format) [minor-command feature]
    /// - Text events (message format)
    /// - Judge events (message format)
    /// - BGA keybound events (message format) [minor-command feature]
    /// - Option events (message format) [minor-command feature]
    fn convert_notes<'a>(&'a self, tokens: &mut Vec<Token<'a>>, params: ConvertNotesParams<'a>) {
        let ConvertNotesParams {
            bpm_reverse_map,
            stop_reverse_map,
            scroll_reverse_map,
            speed_reverse_map,
            judge_reverse_map,
            #[cfg(feature = "minor-command")]
            swbga_reverse_map,
            text_reverse_map,
            #[cfg(feature = "minor-command")]
            option_reverse_map,
            #[cfg(feature = "minor-command")]
            seek_reverse_map,
        } = params;
        let notes = &self.notes;

        // Convert WAV file definitions
        for (obj_id, wav_path) in &notes.wav_files {
            tokens.push(Token::Wav(*obj_id, wav_path));
        }

        // Convert BMP file definitions (from graphics)
        for (obj_id, bmp) in &self.graphics.bmp_files {
            tokens.push(Token::Bmp(Some(*obj_id), &bmp.file));
        }

        // Convert BGM objects
        for (time, bgm_ids) in &notes.bgms {
            let track = time.track.0;
            let channel = Channel::Bgm;
            let message = if bgm_ids.is_empty() {
                "00".to_string()
            } else {
                bgm_ids.iter().map(|id| id.to_string()).collect()
            };
            tokens.push(Token::Message {
                track: Track(track),
                channel,
                message: message.into(),
            });
        }

        // Convert note objects
        for (obj_id, objs) in &notes.objs {
            for obj in objs {
                let track = obj.offset.track.0;
                let channel = self.obj_to_channel(obj);
                let message = obj_id.to_string();
                tokens.push(Token::Message {
                    track: Track(track),
                    channel,
                    message: message.into(),
                });
            }
        }

        // Convert BPM changes (message format)
        // Group BPM changes by track and channel type
        let mut bpm_changes_by_track: HashMap<u64, Vec<(ObjTime, BpmChangeObj)>> = HashMap::new();
        for (time, bpm_obj) in &self.arrangers.bpm_changes {
            bpm_changes_by_track
                .entry(time.track.0)
                .or_default()
                .push((*time, bpm_obj.clone()));
        }

        for (track_num, bpm_changes) in bpm_changes_by_track {
            // Separate BPM changes by channel type
            let mut bpm_change_refs = Vec::new();
            let mut bpm_change_u8s = Vec::new();

            for (time, bpm_obj) in bpm_changes {
                // Try to find the ObjId that corresponds to this BPM value in scope_defines
                let obj_id = bpm_reverse_map.get(&bpm_obj.bpm).copied();

                if let Some(obj_id) = obj_id {
                    // Use BpmChange channel when we have a valid object ID reference
                    bpm_change_refs.push((time, obj_id));
                } else {
                    // Use BpmChangeU8 channel for direct BPM values (0-255 range)
                    let bpm_u8 = bpm_obj.bpm.to_f64().unwrap_or(0.0).round() as u8;
                    bpm_change_u8s.push((time, bpm_u8));
                }
            }

            // Generate message for BpmChange references
            if !bpm_change_refs.is_empty() {
                let message =
                    Self::generate_message_string(&bpm_change_refs, |obj_id| obj_id.to_string());
                tokens.push(Token::Message {
                    track: Track(track_num),
                    channel: Channel::BpmChange,
                    message: message.into(),
                });
            }

            // Generate message for BpmChangeU8 direct values
            if !bpm_change_u8s.is_empty() {
                let message = Self::generate_message_string(&bpm_change_u8s, |bpm_u8| {
                    format!("{:02X}", bpm_u8)
                });
                tokens.push(Token::Message {
                    track: Track(track_num),
                    channel: Channel::BpmChangeU8,
                    message: message.into(),
                });
            }
        }

        // Convert stops (message format)
        let mut stops_by_track: HashMap<u64, Vec<(ObjTime, ObjId)>> = HashMap::new();
        for (time, stop_obj) in &self.arrangers.stops {
            let obj_id = stop_reverse_map
                .get(&stop_obj.duration)
                .copied()
                .unwrap_or(ObjId::null());
            stops_by_track
                .entry(time.track.0)
                .or_default()
                .push((*time, obj_id));
        }

        for (track_num, stops) in stops_by_track {
            let message = Self::generate_message_string(&stops, |obj_id| obj_id.to_string());
            tokens.push(Token::Message {
                track: Track(track_num),
                channel: Channel::Stop,
                message: message.into(),
            });
        }

        // Convert scrolling factors (message format)
        let mut scrolls_by_track: HashMap<u64, Vec<(ObjTime, ObjId)>> = HashMap::new();
        for (time, scroll_obj) in &self.arrangers.scrolling_factor_changes {
            let obj_id = scroll_reverse_map
                .get(&scroll_obj.factor)
                .copied()
                .unwrap_or(ObjId::null());
            scrolls_by_track
                .entry(time.track.0)
                .or_default()
                .push((*time, obj_id));
        }

        for (track_num, scrolls) in scrolls_by_track {
            let message = Self::generate_message_string(&scrolls, |obj_id| obj_id.to_string());
            tokens.push(Token::Message {
                track: Track(track_num),
                channel: Channel::Scroll,
                message: message.into(),
            });
        }

        // Convert speed factors (message format)
        let mut speeds_by_track: HashMap<u64, Vec<(ObjTime, ObjId)>> = HashMap::new();
        for (time, speed_obj) in &self.arrangers.speed_factor_changes {
            let obj_id = speed_reverse_map
                .get(&speed_obj.factor)
                .copied()
                .unwrap_or(ObjId::null());
            speeds_by_track
                .entry(time.track.0)
                .or_default()
                .push((*time, obj_id));
        }

        for (track_num, speeds) in speeds_by_track {
            let message = Self::generate_message_string(&speeds, |obj_id| obj_id.to_string());
            tokens.push(Token::Message {
                track: Track(track_num),
                channel: Channel::Speed,
                message: message.into(),
            });
        }

        // Convert BGM volume changes
        let mut bgm_volumes_by_track: HashMap<u64, Vec<(ObjTime, u8)>> = HashMap::new();
        for (time, bgm_volume_obj) in &notes.bgm_volume_changes {
            let volume_u8 = bgm_volume_obj.volume;
            let clamped_volume = volume_u8.clamp(0, 255);
            bgm_volumes_by_track
                .entry(time.track.0)
                .or_default()
                .push((*time, clamped_volume));
        }

        for (track_num, bgm_volumes) in bgm_volumes_by_track {
            let message =
                Self::generate_message_string(&bgm_volumes, |volume| format!("{:02X}", volume));
            tokens.push(Token::Message {
                track: Track(track_num),
                channel: Channel::BgmVolume,
                message: message.into(),
            });
        }

        // Convert KEY volume changes
        let mut key_volumes_by_track: HashMap<u64, Vec<(ObjTime, u8)>> = HashMap::new();
        for (time, key_volume_obj) in &notes.key_volume_changes {
            let volume_u8 = key_volume_obj.volume;
            let clamped_volume = volume_u8.clamp(0, 255);
            key_volumes_by_track
                .entry(time.track.0)
                .or_default()
                .push((*time, clamped_volume));
        }

        for (track_num, key_volumes) in key_volumes_by_track {
            let message =
                Self::generate_message_string(&key_volumes, |volume| format!("{:02X}", volume));
            tokens.push(Token::Message {
                track: Track(track_num),
                channel: Channel::KeyVolume,
                message: message.into(),
            });
        }

        #[cfg(feature = "minor-command")]
        {
            // Convert seek events
            for (time, seek_obj) in &notes.seek_events {
                let track = time.track.0;
                let channel = Channel::Seek;
                // Find the ObjId that corresponds to this seek time in others.seek_events
                let obj_id = seek_reverse_map
                    .get(&seek_obj.position)
                    .copied()
                    .unwrap_or(ObjId::null());
                let message = obj_id.to_string();
                tokens.push(Token::Message {
                    track: Track(track),
                    channel,
                    message: message.into(),
                });
            }
        }

        // Convert text events
        for (time, text_obj) in &notes.text_events {
            let track = time.track.0;
            let channel = Channel::Text;
            // Find the ObjId that corresponds to this text in others.texts
            let obj_id = text_reverse_map
                .get(&text_obj.text)
                .copied()
                .unwrap_or(ObjId::null());
            let message = obj_id.to_string();
            tokens.push(Token::Message {
                track: Track(track),
                channel,
                message: message.into(),
            });
        }

        // Convert judge events
        for (time, judge_obj) in &notes.judge_events {
            let track = time.track.0;
            let channel = Channel::Judge;
            // Find the ObjId that corresponds to this judge level in scope_defines
            let obj_id = judge_reverse_map
                .get(&judge_obj.judge_level)
                .copied()
                .unwrap_or(ObjId::null());
            let message = obj_id.to_string();
            tokens.push(Token::Message {
                track: Track(track),
                channel,
                message: message.into(),
            });
        }

        #[cfg(feature = "minor-command")]
        {
            // Convert BGA keybound events
            for (time, keybound_obj) in &notes.bga_keybound_events {
                let track = time.track.0;
                let channel = Channel::BgaKeybound;
                // Find the ObjId that corresponds to this event in scope_defines.swbga_events
                let obj_id = swbga_reverse_map
                    .get(&keybound_obj.event)
                    .copied()
                    .unwrap_or(ObjId::null());
                let message = obj_id.to_string();
                tokens.push(Token::Message {
                    track: Track(track),
                    channel,
                    message: message.into(),
                });
            }

            // Convert option events
            for (time, option_obj) in &notes.option_events {
                let track = time.track.0;
                let channel = Channel::Option;
                // Find the ObjId that corresponds to this option in others.change_options
                let obj_id = option_reverse_map
                    .get(&option_obj.option)
                    .copied()
                    .unwrap_or(ObjId::null());
                let message = obj_id.to_string();
                tokens.push(Token::Message {
                    track: Track(track),
                    channel,
                    message: message.into(),
                });
            }
        }
    }

    /// Converts graphics and visual elements to tokens.
    ///
    /// This function handles the conversion of BMS graphics and visual data including:
    /// - Video file (#VIDEOFILE)
    /// - Poor background image (#POORBMP)
    /// - Poor BGA mode (#POORBGA)
    /// - Character file (#CHARFILE) [minor-command feature]
    /// - Video colors (#VIDEOCOLORS) [minor-command feature]
    /// - Video delay (#VIDEODLY) [minor-command feature]
    /// - Video frame rate (#VIDEOFS) [minor-command feature]
    /// - Materials BMP files (#MATERIALSBMP) [minor-command feature]
    /// - BGA changes (message format) [minor-command feature]
    /// - BGA opacity changes (message format) [minor-command feature]
    /// - BGA ARGB changes (message format) [minor-command feature]
    fn convert_graphics<'a>(
        &'a self,
        tokens: &mut Vec<Token<'a>>,
        #[cfg(feature = "minor-command")] argb_reverse_map: &HashMap<&Argb, ObjId>,
    ) {
        let graphics = &self.graphics;

        if let Some(video_file) = &graphics.video_file {
            tokens.push(Token::VideoFile(video_file));
        }

        if let Some(poor_bmp) = &graphics.poor_bmp {
            tokens.push(Token::Bmp(None, poor_bmp));
        }

        if graphics.poor_bga_mode != PoorMode::default() {
            tokens.push(Token::PoorBga(graphics.poor_bga_mode));
        }

        #[cfg(feature = "minor-command")]
        {
            if let Some(char_file) = &graphics.char_file {
                tokens.push(Token::CharFile(char_file));
            }

            if let Some(video_colors) = graphics.video_colors {
                tokens.push(Token::VideoColors(video_colors));
            }

            if let Some(video_dly) = &graphics.video_dly {
                tokens.push(Token::VideoDly(video_dly.clone()));
            }

            if let Some(video_fs) = &graphics.video_fs {
                tokens.push(Token::VideoFs(video_fs.clone()));
            }

            // Convert materials BMP
            for material_bmp in &graphics.materials_bmp {
                tokens.push(Token::MaterialsBmp(material_bmp));
            }

            // Convert BGA changes
            for (time, bga_obj) in &graphics.bga_changes {
                let track = time.track.0;
                let channel = bga_obj.layer.to_channel();
                let message = bga_obj.id.to_string();
                tokens.push(Token::Message {
                    track: Track(track),
                    channel,
                    message: message.into(),
                });
            }

            // Convert BGA opacity changes
            for (layer, opacity_changes) in &graphics.bga_opacity_changes {
                for (time, opacity_obj) in opacity_changes {
                    use crate::bms::parse::model::obj::BgaLayer;

                    let track = time.track.0;
                    let channel = match layer {
                        BgaLayer::Base => Channel::BgaBaseOpacity,
                        BgaLayer::Overlay => Channel::BgaLayerOpacity,
                        BgaLayer::Overlay2 => Channel::BgaLayer2Opacity,
                        BgaLayer::Poor => Channel::BgaPoorOpacity,
                    };
                    let message = format!("{:02X}", opacity_obj.opacity);
                    tokens.push(Token::Message {
                        track: Track(track),
                        channel,
                        message: message.into(),
                    });
                }
            }

            // Convert BGA ARGB changes
            for (layer, argb_changes) in &graphics.bga_argb_changes {
                for (time, argb_obj) in argb_changes {
                    use crate::bms::parse::model::obj::BgaLayer;

                    let track = time.track.0;
                    let channel = match layer {
                        BgaLayer::Base => Channel::BgaBaseArgb,
                        BgaLayer::Overlay => Channel::BgaLayerArgb,
                        BgaLayer::Overlay2 => Channel::BgaLayer2Argb,
                        BgaLayer::Poor => Channel::BgaPoorArgb,
                    };
                    // Find the ObjId that corresponds to this ARGB value in scope_defines
                    let obj_id = argb_reverse_map
                        .get(&argb_obj.argb)
                        .copied()
                        .unwrap_or(ObjId::null());
                    let message = obj_id.to_string();
                    tokens.push(Token::Message {
                        track: Track(track),
                        channel,
                        message: message.into(),
                    });
                }
            }
        }
    }

    /// Converts miscellaneous and other BMS elements to tokens.
    ///
    /// This function handles the conversion of various BMS elements including:
    /// - Options (#OPTION) [minor-command feature]
    /// - Octave flag (#OCTFP) [minor-command feature]
    /// - CDDA values (#CDDA) [minor-command feature]
    /// - Seek events (#SEEKxx) [minor-command feature]
    /// - Extended character events (#EXTCHRxx) [minor-command feature]
    /// - Text definitions (#TEXTxx)
    /// - Non-command lines
    /// - Unknown command lines
    /// - Divide property (#DIVIDEPROP) [minor-command feature]
    /// - Materials path (#MATERIALS) [minor-command feature]
    fn convert_others<'a>(&'a self, tokens: &mut Vec<Token<'a>>) {
        let others = &self.others;

        #[cfg(feature = "minor-command")]
        {
            if let Some(options) = &others.options {
                for option in options {
                    tokens.push(Token::Option(option));
                }
            }

            if others.is_octave {
                tokens.push(Token::OctFp);
            }

            for cdda_value in &others.cdda {
                tokens.push(Token::Cdda(cdda_value.clone()));
            }

            for (obj_id, seek_time) in &others.seek_events {
                tokens.push(Token::Seek(*obj_id, seek_time.clone()));
            }

            for extchr_event in &others.extchr_events {
                tokens.push(Token::ExtChr(*extchr_event));
            }
        }

        // Convert text definitions
        for (obj_id, text) in &others.texts {
            tokens.push(Token::Text(*obj_id, text));
        }

        // Convert non-command lines
        for non_command_line in &others.non_command_lines {
            tokens.push(Token::NotACommand(non_command_line));
        }

        // Convert unknown command lines
        for unknown_command_line in &others.unknown_command_lines {
            tokens.push(Token::UnknownCommand(unknown_command_line));
        }

        // Convert divide property
        #[cfg(feature = "minor-command")]
        if let Some(divide_prop) = &others.divide_prop {
            tokens.push(Token::DivideProp(divide_prop));
        }

        // Convert materials path
        #[cfg(feature = "minor-command")]
        if let Some(materials_path) = &others.materials_path {
            tokens.push(Token::Materials(materials_path));
        }
    }

    // Helper methods for converting various data types to message format

    /// Generate a message string for a given track and time-based events
    fn generate_message_string<T>(
        events: &[(ObjTime, T)],
        value_to_string: impl Fn(&T) -> String,
    ) -> String {
        if events.is_empty() {
            return "00".to_string();
        }

        // Sort events by time to ensure correct ordering
        let mut sorted_events: Vec<_> = events.iter().collect();
        sorted_events.sort_by_key(|(time, _)| *time);

        // Use a standard BMS message length of 16 characters (8 positions * 2 chars each)
        // This is the most common format for BMS files
        let message_length = 16;
        let mut message = vec!['0'; message_length];

        for (time, value) in sorted_events {
            // Calculate position based on numerator and denominator
            // Scale to 8 positions (0-7) for the message
            let position = if time.denominator <= 8 {
                (time.numerator * 2) as usize
            } else {
                // Scale the position to match 8 positions
                let scaled_numerator = time.numerator * 8 / time.denominator;
                (scaled_numerator * 2) as usize
            };

            let value_str = value_to_string(value);
            if position + 1 < message.len() {
                message[position] = value_str.chars().nth(0).unwrap_or('0');
                message[position + 1] = value_str.chars().nth(1).unwrap_or('0');
            }
        }

        message.into_iter().collect()
    }

    fn obj_to_channel(&self, obj: &Obj) -> Channel {
        match obj.kind {
            NoteKind::Visible => Channel::Note {
                kind: NoteKind::Visible,
                side: obj.side,
                key: obj.key,
            },
            NoteKind::Invisible => Channel::Note {
                kind: NoteKind::Invisible,
                side: obj.side,
                key: obj.key,
            },
            NoteKind::Long => Channel::Note {
                kind: NoteKind::Long,
                side: obj.side,
                key: obj.key,
            },
            NoteKind::Landmine => Channel::Note {
                kind: NoteKind::Landmine,
                side: obj.side,
                key: obj.key,
            },
        }
    }
}

#[derive(Debug)]
struct ConvertNotesParams<'a> {
    bpm_reverse_map: HashMap<&'a Decimal, ObjId>,
    stop_reverse_map: HashMap<&'a Decimal, ObjId>,
    scroll_reverse_map: HashMap<&'a Decimal, ObjId>,
    speed_reverse_map: HashMap<&'a Decimal, ObjId>,
    judge_reverse_map: HashMap<&'a JudgeLevel, ObjId>,
    #[cfg(feature = "minor-command")]
    swbga_reverse_map: HashMap<&'a SwBgaEvent, ObjId>,
    text_reverse_map: HashMap<&'a String, ObjId>,
    #[cfg(feature = "minor-command")]
    option_reverse_map: HashMap<&'a String, ObjId>,
    #[cfg(feature = "minor-command")]
    seek_reverse_map: HashMap<&'a Decimal, ObjId>,
}

impl<'a> ConvertNotesParams<'a> {
    fn new(bms: &'a Bms) -> Self {
        let bpm_reverse_map: HashMap<&'a Decimal, ObjId> = bms
            .scope_defines
            .bpm_defs
            .iter()
            .map(|(obj_id, bpm)| (bpm, *obj_id))
            .collect();

        let stop_reverse_map: HashMap<&'a Decimal, ObjId> = bms
            .scope_defines
            .stop_defs
            .iter()
            .map(|(obj_id, duration)| (duration, *obj_id))
            .collect();

        let scroll_reverse_map: HashMap<&'a Decimal, ObjId> = bms
            .scope_defines
            .scroll_defs
            .iter()
            .map(|(obj_id, factor)| (factor, *obj_id))
            .collect();

        let speed_reverse_map: HashMap<&'a Decimal, ObjId> = bms
            .scope_defines
            .speed_defs
            .iter()
            .map(|(obj_id, factor)| (factor, *obj_id))
            .collect();

        let judge_reverse_map: HashMap<&'a JudgeLevel, ObjId> = bms
            .scope_defines
            .exrank_defs
            .iter()
            .map(|(obj_id, exrank_def)| (&exrank_def.judge_level, *obj_id))
            .collect();

        #[cfg(feature = "minor-command")]
        let swbga_reverse_map: HashMap<&'a SwBgaEvent, ObjId> = bms
            .scope_defines
            .swbga_events
            .iter()
            .map(|(obj_id, swbga_event)| (swbga_event, *obj_id))
            .collect();

        let text_reverse_map: HashMap<&'a String, ObjId> = bms
            .others
            .texts
            .iter()
            .map(|(obj_id, text)| (text, *obj_id))
            .collect();

        #[cfg(feature = "minor-command")]
        let option_reverse_map: HashMap<&'a String, ObjId> = bms
            .others
            .change_options
            .iter()
            .map(|(obj_id, option)| (option, *obj_id))
            .collect();

        #[cfg(feature = "minor-command")]
        let seek_reverse_map: HashMap<&'a Decimal, ObjId> = bms
            .others
            .seek_events
            .iter()
            .map(|(obj_id, seek_time)| (seek_time, *obj_id))
            .collect();

        Self {
            bpm_reverse_map,
            stop_reverse_map,
            scroll_reverse_map,
            speed_reverse_map,
            judge_reverse_map,
            #[cfg(feature = "minor-command")]
            swbga_reverse_map,
            text_reverse_map,
            #[cfg(feature = "minor-command")]
            option_reverse_map,
            #[cfg(feature = "minor-command")]
            seek_reverse_map,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bms::prelude::{AlwaysUseNewer, SourcePosMixinExt};
    use crate::lex::TokenStream;
    use crate::parse::ParseOutput;

    use super::*;

    use std::collections::HashSet;
    use std::path::Path;

    #[test]
    fn test_token_roundtrip_comprehensive() {
        // Define original tokens directly - comprehensive test with multiple token types
        let original_tokens = vec![
            // Header tokens
            Token::Title("Comprehensive Test Song"),
            Token::Artist("Test Artist"),
            Token::Genre("Test Genre"),
            Token::Bpm(Decimal::from(120)),
            // WAV file definitions
            Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test1.wav")),
            Token::Wav(ObjId::try_from("02").unwrap(), Path::new("test2.wav")),
            // WAV Message
            Token::Message {
                track: Track(0),
                channel: Channel::Bgm,
                message: "01000200010002".into(),
            },
            Token::Message {
                track: Track(0),
                channel: Channel::Bgm,
                message: "02000100020001".into(),
            },
            // BMP file definitions
            Token::Bmp(Some(ObjId::try_from("01").unwrap()), Path::new("test1.bmp")),
            Token::Bmp(Some(ObjId::try_from("02").unwrap()), Path::new("test2.bmp")),
            // BPM change definitions
            Token::BpmChange(ObjId::try_from("01").unwrap(), Decimal::from(150)),
            Token::BpmChange(ObjId::try_from("02").unwrap(), Decimal::from(180)),
            // Stop definitions
            Token::Stop(ObjId::try_from("01").unwrap(), Decimal::from(100)),
            Token::Stop(ObjId::try_from("02").unwrap(), Decimal::from(200)),
            // Scroll definitions
            Token::Scroll(ObjId::try_from("01").unwrap(), Decimal::from(1.5)),
            Token::Scroll(ObjId::try_from("02").unwrap(), Decimal::from(2.0)),
            // Speed definitions
            Token::Speed(ObjId::try_from("01").unwrap(), Decimal::from(1.2)),
            Token::Speed(ObjId::try_from("02").unwrap(), Decimal::from(1.5)),
        ];

        let token_stream = TokenStream {
            tokens: original_tokens
                .iter()
                .enumerate()
                .map(|(i, t)| t.clone().into_wrapper_manual(i, i))
                .collect::<Vec<_>>(),
        };

        // Create a comprehensive BMS from tokens
        let ParseOutput {
            bms,
            parse_warnings,
        } = Bms::from_token_stream(&token_stream, AlwaysUseNewer);
        assert_eq!(parse_warnings, vec![]);

        // Convert BMS back to tokens
        let BmsUnparseOutput {
            tokens: regenerated_tokens,
        } = bms.unparse();

        // Compare using HashSet
        let original_set: HashSet<_> = original_tokens.iter().collect();
        let regenerated_set: HashSet<_> = regenerated_tokens.iter().collect();

        assert_eq!(
            original_set, regenerated_set,
            "Token roundtrip failed. Original: {:?}, Regenerated: {:?}",
            original_set.iter().collect::<Vec<_>>(), regenerated_tokens.iter().collect::<Vec<_>>()
        );
    }
}
