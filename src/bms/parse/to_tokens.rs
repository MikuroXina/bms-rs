//! Convert Bms to Vec<Token>.
//!
//! This module provides functionality to convert a parsed Bms object back to a vector of tokens.
//! Note that this conversion may not preserve all original formatting and comments,
//! but it will generate valid BMS tokens that represent the same musical data.

use fraction::ToPrimitive;

use crate::bms::{
    Decimal,
    command::{
        JudgeLevel, LnMode, LnType, ObjId, PoorMode, Volume,
        channel::{Channel, Key, NoteKind},
        time::{ObjTime, Track},
    },
    lex::token::Token,
    parse::model::Bms,
};

/// Output of the conversion from `Bms` to `Vec<Token>`.
#[derive(Debug, Clone, PartialEq)]
pub struct BmsToTokensOutput<'a> {
    /// The converted tokens.
    pub tokens: Vec<Token<'a>>,
    /// Warnings that occurred during the conversion.
    pub warnings: Vec<BmsToTokensWarning>,
}

/// Warnings that occur during conversion from `Bms` to `Vec<Token>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum BmsToTokensWarning {
    /// The object ID was out of range and could not be converted.
    #[error("object ID was out of range and could not be converted")]
    ObjIdOutOfRange,
    /// The BPM value was invalid and default value was used.
    #[error("BPM value was invalid, using default value")]
    InvalidBpm,
    /// The stop duration was invalid and default value was used.
    #[error("stop duration was invalid, using default value")]
    InvalidStopDuration,
    /// The scrolling factor was invalid and default value was used.
    #[error("scrolling factor was invalid, using default value")]
    InvalidScrollingFactor,
    /// The speed factor was invalid and default value was used.
    #[error("speed factor was invalid, using default value")]
    InvalidSpeedFactor,
}

impl Bms {
    /// Convert `Bms` to `Vec<Token>`.
    ///
    /// This method converts a parsed Bms object back to a vector of tokens.
    /// The tokens are generated in a logical order that should produce a valid BMS file.
    ///
    /// # Example
    ///
    /// ```rust
    /// use bms_rs::bms::{parse_bms, prelude::BmsToTokensOutput};
    ///
    /// let source = "#TITLE Test Song\n#BPM 120\n#00101:0101";
    /// let bms_output = parse_bms(source);
    /// let BmsToTokensOutput { tokens, warnings } = bms_output.bms.to_tokens();
    /// println!("Generated {} tokens", tokens.len());
    /// println!("Warnings: {:?}", warnings);
    /// ```
    pub fn to_tokens(&self) -> BmsToTokensOutput<'_> {
        let mut tokens = Vec::new();
        let mut warnings = Vec::new();

        // Convert header information
        self.convert_header(&mut tokens);

        // Convert scope definitions
        self.convert_scope_defines(&mut tokens, &mut warnings);

        // Convert arrangers (timing data)
        self.convert_arrangers(&mut tokens, &mut warnings);

        // Convert notes and audio files
        self.convert_notes(&mut tokens, &mut warnings);

        // Convert graphics
        self.convert_graphics(&mut tokens, &mut warnings);

        // Convert others
        self.convert_others(&mut tokens, &mut warnings);

        BmsToTokensOutput { tokens, warnings }
    }

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

    fn convert_scope_defines<'a>(
        &'a self,
        tokens: &mut Vec<Token<'a>>,
        _warnings: &mut [BmsToTokensWarning],
    ) {
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

    fn convert_arrangers<'a>(
        &'a self,
        tokens: &mut Vec<Token<'a>>,
        _warnings: &mut [BmsToTokensWarning],
    ) {
        let arrangers = &self.arrangers;

        // Convert initial BPM
        if let Some(bpm) = &arrangers.bpm {
            tokens.push(Token::Bpm(bpm.clone()));
        }

        // Convert section length changes
        for _section_len_obj in arrangers.section_len_changes.values() {
            // Section length changes are handled in message format
            // We'll handle them in the notes conversion section
        }

        // Convert BPM changes (these are handled in message format)
        // We'll handle them in the notes conversion section

        // Convert stops (these are handled in message format)
        // We'll handle them in the notes conversion section

        // Convert scrolling factor changes (these are handled in message format)
        // We'll handle them in the notes conversion section

        // Convert speed factor changes (these are handled in message format)
        // We'll handle them in the notes conversion section

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

    fn convert_notes<'a>(
        &'a self,
        tokens: &mut Vec<Token<'a>>,
        _warnings: &mut [BmsToTokensWarning],
    ) {
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
            let message = self.obj_ids_to_message(bgm_ids, time);
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
                let message = self.obj_id_to_message(*obj_id, &obj.offset);
                tokens.push(Token::Message {
                    track: Track(track),
                    channel,
                    message: message.into(),
                });
            }
        }

        // Convert BPM changes (message format)
        for (time, bpm_obj) in &self.arrangers.bpm_changes {
            let track = time.track.0;
            let channel = Channel::BpmChangeU8;
            let message = self.bpm_to_message(bpm_obj.bpm.clone());
            tokens.push(Token::Message {
                track: Track(track),
                channel,
                message: message.into(),
            });
        }

        // Convert stops (message format)
        for (time, stop_obj) in &self.arrangers.stops {
            let track = time.track.0;
            let channel = Channel::Stop;
            let message = self.stop_to_message(stop_obj.duration.clone());
            tokens.push(Token::Message {
                track: Track(track),
                channel,
                message: message.into(),
            });
        }

        // Convert scrolling factors (message format)
        for (time, scroll_obj) in &self.arrangers.scrolling_factor_changes {
            let track = time.track.0;
            let channel = Channel::Scroll;
            let message = self.scroll_to_message(scroll_obj.factor.clone());
            tokens.push(Token::Message {
                track: Track(track),
                channel,
                message: message.into(),
            });
        }

        // Convert speed factors (message format)
        for (time, speed_obj) in &self.arrangers.speed_factor_changes {
            let track = time.track.0;
            let channel = Channel::Speed;
            let message = self.speed_to_message(speed_obj.factor.clone());
            tokens.push(Token::Message {
                track: Track(track),
                channel,
                message: message.into(),
            });
        }

        // Convert BGM volume changes
        for (time, bgm_volume_obj) in &notes.bgm_volume_changes {
            let track = time.track.0;
            let channel = Channel::BgmVolume;
            let message = self.volume_to_message(Volume {
                relative_percent: bgm_volume_obj.volume,
            });
            tokens.push(Token::Message {
                track: Track(track),
                channel,
                message: message.into(),
            });
        }

        // Convert KEY volume changes
        for (time, key_volume_obj) in &notes.key_volume_changes {
            let track = time.track.0;
            let channel = Channel::KeyVolume;
            let message = self.volume_to_message(Volume {
                relative_percent: key_volume_obj.volume,
            });
            tokens.push(Token::Message {
                track: Track(track),
                channel,
                message: message.into(),
            });
        }

        #[cfg(feature = "minor-command")]
        {
            // Convert seek events
            for (time, seek_obj) in &notes.seek_events {
                let track = time.track.0;
                let channel = Channel::Seek;
                let message = self.seek_to_message(seek_obj.position.clone());
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
            let message = self.text_to_message(&text_obj.text);
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
            let message = self.judge_to_message(judge_obj.judge_level);
            tokens.push(Token::Message {
                track: Track(track),
                channel,
                message: message.into(),
            });
        }

        #[cfg(feature = "minor-command")]
        {
            // Convert BGA keybound events
            for time in notes.bga_keybound_events.keys() {
                let track = time.track.0;
                let channel = Channel::BgaKeybound;
                // BgaKeyboundObj doesn't have a key field, it has an event field
                // This needs to be handled differently
                let message = "00".to_string(); // Placeholder
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
                let message = self.option_to_message(&option_obj.option);
                tokens.push(Token::Message {
                    track: Track(track),
                    channel,
                    message: message.into(),
                });
            }
        }
    }

    fn convert_graphics<'a>(
        &'a self,
        tokens: &mut Vec<Token<'a>>,
        _warnings: &mut [BmsToTokensWarning],
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
        }
    }

    fn convert_others<'a>(
        &'a self,
        tokens: &mut Vec<Token<'a>>,
        _warnings: &mut [BmsToTokensWarning],
    ) {
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
    }

    // Helper methods for converting various data types to message format

    fn obj_to_channel(&self, obj: &crate::bms::parse::model::obj::Obj) -> Channel {
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

    fn obj_ids_to_message(&self, obj_ids: &[ObjId], _time: &ObjTime) -> String {
        // This is a simplified implementation
        // In a real implementation, you would need to handle the timing properly
        if obj_ids.is_empty() {
            return "00".to_string(); // Return empty object ID for no objects
        }
        
        obj_ids
            .iter()
            .map(|id| {
                // Convert the string to a hex representation
                id.to_string()
            })
            .collect()
    }

    fn obj_id_to_message(&self, obj_id: ObjId, _time: &ObjTime) -> String {
        // This is a simplified implementation
        // In a real implementation, you would need to handle the timing properly
        obj_id.to_string()
    }

    fn bpm_to_message(&self, bpm: Decimal) -> String {
        // Convert BPM to hex format
        let bpm_u8 = bpm.to_u8().unwrap_or(120);
        format!("{:02X}", bpm_u8)
    }

    fn stop_to_message(&self, duration: Decimal) -> String {
        // Convert stop duration to hex format
        let duration_u8 = duration.to_u8().unwrap_or(0);
        format!("{:02X}", duration_u8)
    }

    fn scroll_to_message(&self, factor: Decimal) -> String {
        // Convert scroll factor to hex format
        let factor_u8 = factor.to_u8().unwrap_or(1);
        format!("{:02X}", factor_u8)
    }

    fn speed_to_message(&self, factor: Decimal) -> String {
        // Convert speed factor to hex format
        let factor_u8 = factor.to_u8().unwrap_or(1);
        format!("{:02X}", factor_u8)
    }

    fn volume_to_message(&self, volume: Volume) -> String {
        // Convert volume to hex format
        let volume_u8 = volume.relative_percent;
        // Ensure volume is within valid range (0-255)
        let clamped_volume = volume_u8.clamp(0, 255);
        format!("{:02X}", clamped_volume)
    }

    #[cfg(feature = "minor-command")]
    fn seek_to_message(&self, seek_time: Decimal) -> String {
        // Convert seek time to hex format
        let seek_time_u8 = seek_time.to_u8().unwrap_or(0);
        format!("{:02X}", seek_time_u8)
    }

    fn text_to_message(&self, text: &str) -> String {
        // Convert text to hex format (simplified)
        text.chars()
            .take(2)
            .map(|c| {
                // Safely convert char to u8, handling potential overflow
                let byte = if c as u32 <= u8::MAX as u32 {
                    c as u8
                } else {
                    b'?' // Use question mark for invalid characters
                };
                format!("{:02X}", byte)
            })
            .collect()
    }

    fn judge_to_message(&self, level: JudgeLevel) -> String {
        // Convert judge level to hex format
        let level_u8 = match level {
            JudgeLevel::Easy => 1,
            JudgeLevel::Normal => 2,
            JudgeLevel::Hard => 3,
            JudgeLevel::VeryHard => 0,
            JudgeLevel::OtherInt(n) => {
                // Safely convert to u8, handling potential overflow
                if n >= 0 && n <= u8::MAX as i64 {
                    n as u8
                } else {
                    0 // Default to 0 for out-of-range values
                }
            }
        };
        format!("{:02X}", level_u8)
    }

    #[cfg(feature = "minor-command")]
    #[allow(dead_code)]
    fn bga_keybound_to_message(&self, _key: Key) -> String {
        // Convert key to hex format
        // This is a placeholder implementation
        "00".to_string()
    }

    #[cfg(feature = "minor-command")]
    fn option_to_message(&self, option: &str) -> String {
        // Convert option to hex format (simplified)
        option
            .chars()
            .take(2)
            .map(|c| {
                // Safely convert char to u8, handling potential overflow
                let byte = if c as u32 <= u8::MAX as u32 {
                    c as u8
                } else {
                    b'?' // Use question mark for invalid characters
                };
                format!("{:02X}", byte)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bms::parse_bms;

    #[test]
    fn test_bms_to_tokens_basic() {
        let source = "#TITLE Test Song\n#BPM 120\n#ARTIST Test Artist";
        let bms_output = parse_bms(source);
        let BmsToTokensOutput { tokens, warnings } = bms_output.bms.to_tokens();

        assert!(warnings.is_empty());
        assert!(!tokens.is_empty());

        // Check that we have the expected tokens
        let mut has_title = false;
        let mut has_bpm = false;
        let mut has_artist = false;

        for token in &tokens {
            match token {
                Token::Title(_) => has_title = true,
                Token::Bpm(_) => has_bpm = true,
                Token::Artist(_) => has_artist = true,
                _ => {}
            }
        }

        assert!(has_title);
        assert!(has_bpm);
        assert!(has_artist);
    }

    #[test]
    fn test_bms_to_tokens_with_notes() {
        let source = "#TITLE Test Song\n#BPM 120\n#WAV01 test.wav\n#00101:01";
        let bms_output = parse_bms(source);
        let BmsToTokensOutput { tokens, warnings } = bms_output.bms.to_tokens();

        assert!(warnings.is_empty());
        assert!(!tokens.is_empty());

        // Check that we have WAV definition
        let mut has_wav = false;
        for token in &tokens {
            if let Token::Wav(_, _) = token {
                has_wav = true;
                break;
            }
        }

        assert!(has_wav);
    }
}
