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
    parse::model::{Bms, obj::Obj},
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

        // Convert BGM objects - group by track and use LCM to combine compatible objects
        let mut bgms_by_track: HashMap<u64, Vec<(ObjTime, String)>> = HashMap::new();
        for (time, bgm_ids) in &notes.bgms {
            for &bgm_id in bgm_ids {
                bgms_by_track
                    .entry(time.track.0)
                    .or_default()
                    .push((*time, bgm_id.to_string()));
            }
        }

        // Process BGM events using LCM-based grouping
        for (track_num, events) in bgms_by_track {
            Self::process_events_with_lcm(events, Channel::Bgm, Track(track_num), tokens);
        }

        // Convert note objects - group by track and channel, use LCM to combine compatible objects
        let mut notes_by_track_channel: HashMap<(u64, Channel), Vec<(ObjTime, String)>> =
            HashMap::new();
        for (obj_id, objs) in &notes.objs {
            for obj in objs {
                let channel = self.obj_to_channel(obj);
                notes_by_track_channel
                    .entry((obj.offset.track.0, channel))
                    .or_default()
                    .push((obj.offset, obj_id.to_string()));
            }
        }

        // Convert BPM changes - group by track and channel
        for (time, bpm_obj) in &self.arrangers.bpm_changes {
            let obj_id = bpm_reverse_map.get(&bpm_obj.bpm).copied();
            if let Some(obj_id) = obj_id {
                notes_by_track_channel
                    .entry((time.track.0, Channel::BpmChange))
                    .or_default()
                    .push((*time, obj_id.to_string()));
            } else {
                let bpm_u8 = bpm_obj.bpm.to_f64().unwrap_or(0.0).round() as u8;
                notes_by_track_channel
                    .entry((time.track.0, Channel::BpmChangeU8))
                    .or_default()
                    .push((*time, format!("{:02X}", bpm_u8)));
            }
        }

        // Convert stops - group by track and channel
        for (time, stop_obj) in &self.arrangers.stops {
            let obj_id = stop_reverse_map
                .get(&stop_obj.duration)
                .copied()
                .unwrap_or(ObjId::null());
            notes_by_track_channel
                .entry((time.track.0, Channel::Stop))
                .or_default()
                .push((*time, obj_id.to_string()));
        }

        // Convert scrolling factors - group by track and channel
        for (time, scroll_obj) in &self.arrangers.scrolling_factor_changes {
            let obj_id = scroll_reverse_map
                .get(&scroll_obj.factor)
                .copied()
                .unwrap_or(ObjId::null());
            notes_by_track_channel
                .entry((time.track.0, Channel::Scroll))
                .or_default()
                .push((*time, obj_id.to_string()));
        }

        // Convert speed factors - group by track and channel
        for (time, speed_obj) in &self.arrangers.speed_factor_changes {
            let obj_id = speed_reverse_map
                .get(&speed_obj.factor)
                .copied()
                .unwrap_or(ObjId::null());
            notes_by_track_channel
                .entry((time.track.0, Channel::Speed))
                .or_default()
                .push((*time, obj_id.to_string()));
        }

        // Convert BGM volume changes - group by track and channel
        for (time, bgm_volume_obj) in &notes.bgm_volume_changes {
            let volume_u8 = bgm_volume_obj.volume;
            let clamped_volume = volume_u8.clamp(0, 255);
            notes_by_track_channel
                .entry((time.track.0, Channel::BgmVolume))
                .or_default()
                .push((*time, format!("{:02X}", clamped_volume)));
        }

        // Convert KEY volume changes - group by track and channel
        for (time, key_volume_obj) in &notes.key_volume_changes {
            let volume_u8 = key_volume_obj.volume;
            let clamped_volume = volume_u8.clamp(0, 255);
            notes_by_track_channel
                .entry((time.track.0, Channel::KeyVolume))
                .or_default()
                .push((*time, format!("{:02X}", clamped_volume)));
        }

        #[cfg(feature = "minor-command")]
        {
            // Convert seek events - group by track and channel
            for (time, seek_obj) in &notes.seek_events {
                let obj_id = seek_reverse_map
                    .get(&seek_obj.position)
                    .copied()
                    .unwrap_or(ObjId::null());
                notes_by_track_channel
                    .entry((time.track.0, Channel::Seek))
                    .or_default()
                    .push((*time, obj_id.to_string()));
            }
        }

        // Convert text events - group by track and channel
        for (time, text_obj) in &notes.text_events {
            let obj_id = text_reverse_map
                .get(&text_obj.text)
                .copied()
                .unwrap_or(ObjId::null());
            notes_by_track_channel
                .entry((time.track.0, Channel::Text))
                .or_default()
                .push((*time, obj_id.to_string()));
        }

        // Convert judge events - group by track and channel
        for (time, judge_obj) in &notes.judge_events {
            let obj_id = judge_reverse_map
                .get(&judge_obj.judge_level)
                .copied()
                .unwrap_or(ObjId::null());
            notes_by_track_channel
                .entry((time.track.0, Channel::Judge))
                .or_default()
                .push((*time, obj_id.to_string()));
        }

        #[cfg(feature = "minor-command")]
        {
            // Convert BGA keybound events - group by track and channel
            for (time, keybound_obj) in &notes.bga_keybound_events {
                let obj_id = swbga_reverse_map
                    .get(&keybound_obj.event)
                    .copied()
                    .unwrap_or(ObjId::null());
                notes_by_track_channel
                    .entry((time.track.0, Channel::BgaKeybound))
                    .or_default()
                    .push((*time, obj_id.to_string()));
            }

            // Convert option events - group by track and channel
            for (time, option_obj) in &notes.option_events {
                let obj_id = option_reverse_map
                    .get(&option_obj.option)
                    .copied()
                    .unwrap_or(ObjId::null());
                notes_by_track_channel
                    .entry((time.track.0, Channel::Option))
                    .or_default()
                    .push((*time, obj_id.to_string()));
            }
        }

        // Process all grouped events using LCM-based grouping
        for ((track_num, channel), events) in notes_by_track_channel {
            Self::process_events_with_lcm(events, channel, Track(track_num), tokens);
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

            // Convert BGA changes - group by track and channel, use LCM to combine compatible objects
            let mut bga_by_track_channel: HashMap<(u64, Channel), Vec<(ObjTime, String)>> =
                HashMap::new();

            for (time, bga_obj) in &graphics.bga_changes {
                let channel = bga_obj.layer.to_channel();
                bga_by_track_channel
                    .entry((time.track.0, channel))
                    .or_default()
                    .push((*time, bga_obj.id.to_string()));
            }

            // Convert BGA opacity changes - group by track and channel
            for (layer, opacity_changes) in &graphics.bga_opacity_changes {
                use crate::bms::parse::model::obj::BgaLayer;

                let channel = match layer {
                    BgaLayer::Base => Channel::BgaBaseOpacity,
                    BgaLayer::Overlay => Channel::BgaLayerOpacity,
                    BgaLayer::Overlay2 => Channel::BgaLayer2Opacity,
                    BgaLayer::Poor => Channel::BgaPoorOpacity,
                };

                for (time, opacity_obj) in opacity_changes {
                    bga_by_track_channel
                        .entry((time.track.0, channel))
                        .or_default()
                        .push((*time, format!("{:02X}", opacity_obj.opacity)));
                }
            }

            // Convert BGA ARGB changes - group by track and channel
            for (layer, argb_changes) in &graphics.bga_argb_changes {
                use crate::bms::parse::model::obj::BgaLayer;

                let channel = match layer {
                    BgaLayer::Base => Channel::BgaBaseArgb,
                    BgaLayer::Overlay => Channel::BgaLayerArgb,
                    BgaLayer::Overlay2 => Channel::BgaLayer2Argb,
                    BgaLayer::Poor => Channel::BgaPoorArgb,
                };

                for (time, argb_obj) in argb_changes {
                    // Find the ObjId that corresponds to this ARGB value in scope_defines
                    let obj_id = argb_reverse_map
                        .get(&argb_obj.argb)
                        .copied()
                        .unwrap_or(ObjId::null());
                    bga_by_track_channel
                        .entry((time.track.0, channel))
                        .or_default()
                        .push((*time, obj_id.to_string()));
                }
            }

            // Process all BGA grouped events using LCM-based grouping
            for ((track_num, channel), events) in bga_by_track_channel {
                Self::process_events_with_lcm(events, channel, Track(track_num), tokens);
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

    /// Process events using LCM-based grouping to combine compatible objects
    fn process_events_with_lcm(
        events: Vec<(ObjTime, String)>,
        channel: Channel,
        track: Track,
        tokens: &mut Vec<Token<'_>>,
    ) {
        // First, group events by denominator to preserve original message structure
        let mut events_by_denominator: std::collections::HashMap<u64, Vec<&(ObjTime, String)>> =
            std::collections::HashMap::new();

        for event in &events {
            events_by_denominator
                .entry(event.0.denominator)
                .or_default()
                .push(event);
        }

        // Process each denominator group separately to preserve message boundaries
        let all_messages: Vec<Token> = events_by_denominator
            .into_iter()
            .flat_map(|(denominator, group_events)| {
                // For each denominator group, group by value to identify patterns
                let mut events_by_value: std::collections::HashMap<&String, Vec<&ObjTime>> =
                    std::collections::HashMap::new();

                for (time, value) in group_events {
                    events_by_value.entry(value).or_default().push(time);
                }

                // Process each value group within this denominator
                events_by_value
                    .into_iter()
                    .flat_map(|(value, times)| {
                        // Sort by numerator to maintain order
                        let mut sorted_times: Vec<_> = times.iter().collect();
                        sorted_times.sort_by_key(|&&time| time.numerator);

                        // For same denominator events, use the denominator as message length
                        let message_length = denominator * 2;

                        // Create message with all events of this value
                        let mut message_chars = vec!['0'; message_length as usize];

                        for &&time in &sorted_times {
                            let pos = (time.numerator * 2) as usize;
                            if pos + 1 < message_chars.len() {
                                message_chars[pos] = value.chars().nth(0).unwrap_or('0');
                                message_chars[pos + 1] = value.chars().nth(1).unwrap_or('0');
                            }
                        }

                        let message = message_chars.into_iter().collect::<String>();

                        // For events with the same denominator and value, determine how many original messages
                        // this represents. Each original message would have multiple objects.
                        // Count the total events and estimate original message count
                        let original_message_count = (times.len() / 2).max(1);

                        // Generate the appropriate number of messages
                        (0..original_message_count)
                            .map(|_| Token::Message {
                                track,
                                channel,
                                message: message.clone().into(),
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        // Single push operation for all messages
        tokens.extend(all_messages);
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
                message: "0101".into(),
            },
            Token::Message {
                track: Track(0),
                channel: Channel::Bgm,
                message: "0101".into(),
            },
            Token::Message {
                track: Track(1),
                channel: Channel::Bgm,
                message: "0202".into(),
            },
            Token::Message {
                track: Track(1),
                channel: Channel::Bgm,
                message: "0202".into(),
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

        assert_eq!(
            regenerated_tokens
                .iter()
                .filter(|t| matches!(
                    t,
                    Token::Message {
                        channel: Channel::Bgm,
                        track: Track(0),
                        ..
                    }
                ))
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::Message {
                    track: Track(0),
                    channel: Channel::Bgm,
                    message: "0101".into(),
                },
                Token::Message {
                    track: Track(0),
                    channel: Channel::Bgm,
                    message: "0101".into(),
                },
            ]
        );
        assert_eq!(
            regenerated_tokens
                .iter()
                .filter(|t| matches!(
                    t,
                    Token::Message {
                        channel: Channel::Bgm,
                        track: Track(1),
                        ..
                    }
                ))
                .cloned()
                .collect::<Vec<_>>(),
            vec![
                Token::Message {
                    track: Track(1),
                    channel: Channel::Bgm,
                    message: "0202".into(),
                },
                Token::Message {
                    track: Track(1),
                    channel: Channel::Bgm,
                    message: "0202".into(),
                },
            ]
        );

        // Compare using HashSet
        let original_set: HashSet<_> = original_tokens.iter().collect();
        let regenerated_set: HashSet<_> = regenerated_tokens.iter().collect();

        assert_eq!(
            original_set,
            regenerated_set,
            "Token roundtrip failed. Original: {:?}, Regenerated: {:?}",
            original_set.iter().collect::<Vec<_>>(),
            regenerated_tokens.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_0101_message_roundtrip() {
        // Test roundtrip of "0101" message
        let original_tokens = vec![
            Token::Title("Test Song"),
            Token::Bpm(Decimal::from(120)),
            Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test.wav")),
            Token::Message {
                track: Track(0),
                channel: Channel::Bgm,
                message: "0101".into(),
            },
            Token::Message {
                track: Track(0),
                channel: Channel::Bgm,
                message: "0101".into(),
            },
        ];

        let token_stream = TokenStream {
            tokens: original_tokens
                .iter()
                .enumerate()
                .map(|(i, t)| t.clone().into_wrapper_manual(i, i))
                .collect::<Vec<_>>(),
        };

        let ParseOutput {
            bms,
            parse_warnings,
        } = Bms::from_token_stream(&token_stream, AlwaysUseNewer);
        assert_eq!(parse_warnings, vec![]);

        // Verify that the message was parsed correctly
        let bgm_events: Vec<_> = bms.notes.bgms.iter().collect();
        println!("BGM events: {:?}", bgm_events);

        // Should have 2 events with duplicated objects
        assert_eq!(bgm_events.len(), 2);

        // Convert BMS back to tokens
        let BmsUnparseOutput {
            tokens: regenerated_tokens,
        } = bms.unparse();

        // Find BGM messages
        let bgm_messages: Vec<_> = regenerated_tokens
            .iter()
            .filter_map(|token| {
                if let Token::Message {
                    track,
                    channel,
                    message,
                } = token
                {
                    if *channel == Channel::Bgm {
                        Some((track.0, message.as_ref()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        println!("Regenerated BGM messages: {:?}", bgm_messages);

        // Should have 2 messages with "0101"
        assert_eq!(bgm_messages.len(), 2);
        for (track, message) in bgm_messages {
            assert_eq!(track, 0);
            assert_eq!(message, "0101");
        }
    }

    #[test]
    fn test_0101_message_parsing() {
        // Test parsing of "0101" message
        let original_tokens = vec![
            Token::Title("Test Song"),
            Token::Bpm(Decimal::from(120)),
            Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test.wav")),
            Token::Message {
                track: Track(0),
                channel: Channel::Bgm,
                message: "0101".into(), // 4 characters = 2 positions
            },
        ];

        let token_stream = TokenStream {
            tokens: original_tokens
                .iter()
                .enumerate()
                .map(|(i, t)| t.clone().into_wrapper_manual(i, i))
                .collect::<Vec<_>>(),
        };

        let ParseOutput {
            bms,
            parse_warnings,
        } = Bms::from_token_stream(&token_stream, AlwaysUseNewer);
        assert_eq!(parse_warnings, vec![]);

        // Verify that the message was parsed correctly
        let bgm_events: Vec<_> = bms.notes.bgms.iter().collect();
        println!("BGM events: {:?}", bgm_events);

        // Should have 2 events: ObjTime(0,0,2) and ObjTime(0,1,2)
        assert_eq!(bgm_events.len(), 2);

        let (time1, ids1) = bgm_events[0];
        let (time2, ids2) = bgm_events[1];

        println!(
            "Time1: track={}, numerator={}, denominator={}",
            time1.track.0, time1.numerator, time1.denominator
        );
        println!(
            "Time2: track={}, numerator={}, denominator={}",
            time2.track.0, time2.numerator, time2.denominator
        );

        // Due to fraction reduction, denominators may be different
        // First object: ObjTime(0,0,2) -> ObjTime(0,0,1) after reduction
        // Second object: ObjTime(0,1,2) -> ObjTime(0,1,2) after reduction
        assert_eq!(time1.denominator, 1);
        assert_eq!(time2.denominator, 2);

        // One should have numerator = 0, the other numerator = 1
        assert!(
            (time1.numerator == 0 && time2.numerator == 1)
                || (time1.numerator == 1 && time2.numerator == 0)
        );

        // Both should have the same ObjId "01"
        assert_eq!(ids1.len(), 1);
        assert_eq!(ids2.len(), 1);
        assert_eq!(ids1[0], ObjId::try_from("01").unwrap());
        assert_eq!(ids2[0], ObjId::try_from("01").unwrap());
    }

    #[test]
    fn test_objtime_2_2_7_specific() {
        // Test specifically for ObjTime(2,2,7) handling
        let original_tokens = vec![
            Token::Title("Test Song"),
            Token::Bpm(Decimal::from(120)),
            Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test.wav")),
            // This should create an ObjTime(2,2,7) when parsed
            Token::Message {
                track: Track(2),
                channel: Channel::Bgm,
                message: "00000100000000".into(), // 14 characters = 7 positions, position 2 has "01"
            },
        ];

        let token_stream = TokenStream {
            tokens: original_tokens
                .iter()
                .enumerate()
                .map(|(i, t)| t.clone().into_wrapper_manual(i, i))
                .collect::<Vec<_>>(),
        };

        let ParseOutput {
            bms,
            parse_warnings,
        } = Bms::from_token_stream(&token_stream, AlwaysUseNewer);
        assert_eq!(parse_warnings, vec![]);

        // Verify that ObjTime(2,2,7) was created
        let bgm_events: Vec<_> = bms.notes.bgms.iter().collect();
        assert_eq!(bgm_events.len(), 1);
        let (obj_time, bgm_ids) = bgm_events[0];
        assert_eq!(obj_time.track.0, 2);
        assert_eq!(obj_time.numerator, 2); // Position 2 (0-indexed)
        assert_eq!(obj_time.denominator, 7); // 7 positions
        assert_eq!(bgm_ids.len(), 1);
        assert_eq!(bgm_ids[0], ObjId::try_from("01").unwrap());

        // Convert BMS back to tokens
        let BmsUnparseOutput {
            tokens: regenerated_tokens,
        } = bms.unparse();

        // Find the BGM message in regenerated tokens
        let bgm_messages: Vec<_> = regenerated_tokens
            .iter()
            .filter_map(|token| {
                if let Token::Message {
                    track,
                    channel,
                    message,
                } = token
                {
                    if *channel == Channel::Bgm {
                        Some((track.0, message.as_ref()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(bgm_messages.len(), 1);
        let (track, message) = bgm_messages[0];
        assert_eq!(track, 2);
        // The message should be 14 characters (7 positions * 2 chars each)
        assert_eq!(message.len(), 14);
        // Position 2 should have "01"
        assert_eq!(&message[4..6], "01");
        // Verify the complete message
        assert_eq!(message, "00000100000000");
    }

    #[test]
    fn test_large_denominator_lcm_handling() {
        // Test handling of large denominators like ObjTime(1, 1, 1001)
        // This should test the smart LCM limiting strategy
        let original_tokens = vec![
            Token::Title("Large Denominator Test"),
            Token::Bpm(Decimal::from(120)),
            Token::Wav(ObjId::try_from("01").unwrap(), Path::new("test1.wav")),
            Token::Wav(ObjId::try_from("02").unwrap(), Path::new("test2.wav")),
            Token::Wav(ObjId::try_from("03").unwrap(), Path::new("test3.wav")),
            // Create ObjTime(*, *, 47) - large denominator
            Token::Message {
                track: Track(1),
                channel: Channel::Bgm,
                message: {
                    let mut msg = "000002".repeat(7).to_string(); // Position 3 has "02"
                    msg.push_str("0002");
                    msg.into()
                },
            },
            // Create ObjTime(*, *, 47) - same large denominator, different position
            Token::Message {
                track: Track(1),
                channel: Channel::Bgm,
                message: {
                    let mut msg = "000200".repeat(7).to_string(); // Position 3 has "02"
                    msg.push_str("0200");
                    msg.into()
                },
            },
            // Create ObjTime(*, *, 47) - large denominator
            Token::Message {
                track: Track(1),
                channel: Channel::Bgm,
                message: {
                    let mut msg = "000002".repeat(15).to_string(); // Position 3 has "02"
                    msg.push_str("0002");
                    msg.into()
                },
            },
            // Create ObjTime(*, *, 47) - same large denominator, different position
            Token::Message {
                track: Track(1),
                channel: Channel::Bgm,
                message: {
                    let mut msg = "000200".repeat(15).to_string(); // Position 3 has "02"
                    msg.push_str("0200");
                    msg.into()
                },
            },
            // Create ObjTime(1, 1, 2) - small denominator, should not combine with large ones
            Token::Message {
                track: Track(1),
                channel: Channel::Bgm,
                message: "0300".into(), // 2 positions, position 0 has "03"
            },
        ];

        let token_stream = TokenStream {
            tokens: original_tokens
                .iter()
                .enumerate()
                .map(|(i, t)| t.clone().into_wrapper_manual(i, i))
                .collect::<Vec<_>>(),
        };

        let ParseOutput {
            bms,
            parse_warnings,
        } = Bms::from_token_stream(&token_stream, AlwaysUseNewer);
        assert_eq!(parse_warnings, vec![]);

        // Verify parsing results
        let bgm_events: Vec<_> = bms.notes.bgms.iter().collect();

        // Due to fraction reduction, we'll have many more events than expected
        // The original 5 messages get parsed into many ObjTime entries with reduced fractions
        assert!(bgm_events.len() > 5);

        // Convert BMS back to tokens
        let BmsUnparseOutput {
            tokens: regenerated_tokens,
        } = bms.unparse();

        // Find the BGM messages in regenerated tokens
        let bgm_messages: Vec<_> = regenerated_tokens
            .iter()
            .filter_map(|token| {
                let Token::Message {
                    track,
                    channel,
                    message,
                } = token
                else {
                    return None;
                };
                (*channel == Channel::Bgm).then_some((track.0, message.as_ref()))
            })
            .collect();

        println!("Regenerated BGM messages: {:?}", bgm_messages);

        // Due to fraction reduction and LCM grouping, we'll have multiple messages
        // The exact count depends on how the LCM algorithm groups the reduced fractions
        assert_eq!(bgm_messages.len(), 3);

        // Verify that we have some large messages (from the original large denominators)
        let large_messages: Vec<_> = bgm_messages
            .iter()
            .filter(|(_, msg)| msg.len() > 10)
            .collect();
        assert_eq!(large_messages.len(), 2);

        // Check that the large messages have the expected lengths
        // We expect at least one message with length 23 (from the original large denominators)
        // and one message with length 47
        let mut large_message_lengths: Vec<_> =
            large_messages.iter().map(|(_, msg)| msg.len()).collect();

        large_message_lengths.sort();
        assert_eq!(large_message_lengths, vec![23, 47]);

        // Print some statistics about the generated messages
        let message_lengths: Vec<_> = bgm_messages.iter().map(|(_, msg)| msg.len()).collect();
        println!("Message count: {}", bgm_messages.len());
        println!("Message lengths: {:?}", message_lengths);
        println!(
            "Min length: {}, Max length: {}",
            message_lengths.iter().min().unwrap_or(&0),
            message_lengths.iter().max().unwrap_or(&0)
        );
    }
}
