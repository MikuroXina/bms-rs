//! Header information from parsed BMS file.
//! Note objects manager.

mod arrangers;
pub mod def;
mod graphics;
mod notes;
pub mod obj;

use std::{collections::HashMap, fmt::Debug, path::PathBuf};

#[cfg(feature = "minor-command")]
use num::BigUint;

#[cfg(feature = "minor-command")]
use crate::bms::command::{
    graphics::Argb,
    minor_command::{ExtChrEvent, SwBgaEvent, WavCmdEvent},
};
use crate::bms::{
    Decimal,
    command::{
        JudgeLevel, LnMode, LnType, ObjId, PlayerMode, Volume,
        channel::mapper::{KeyLayoutBeat, KeyLayoutMapper},
        time::ObjTime,
    },
};

use self::def::ExRankDef;
#[cfg(feature = "minor-command")]
use self::def::{AtBgaDef, BgaDef, ExWavDef};

pub use arrangers::Arrangers;
pub use graphics::Graphics;
pub use notes::Notes;

/// A score data aggregate of BMS format.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bms<T = KeyLayoutBeat> {
    /// The header data in the score.
    pub header: Header,
    /// The scope-defines in the score.
    pub scope_defines: ScopeDefines,
    /// The arranges in the score. Contains timing and arrangement data like BPM changes, stops, and scrolling factors.
    pub arrangers: Arrangers,
    /// The objects in the score. Contains all note objects, BGM events, and audio file definitions.
    pub notes: Notes<T>,
    /// The graphics part in the score. Contains background images, videos, BGA events, and visual elements.
    pub graphics: Graphics,
    /// The other part in the score. Contains miscellaneous data like text objects, options, and non-standard commands.
    pub others: Others,
}

impl<T: KeyLayoutMapper> Default for Bms<T> {
    fn default() -> Self {
        Self {
            header: Default::default(),
            scope_defines: Default::default(),
            arrangers: Default::default(),
            notes: Default::default(),
            graphics: Default::default(),
            others: Default::default(),
        }
    }
}

/// A header of the score, including the information that is usually used in music selection.
/// Parsed from [`TokenStream`](crate::lex::Token::TokenStream).
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Header {
    /// The play style of the score.
    pub player: Option<PlayerMode>,
    /// The genre of the score.
    pub genre: Option<String>,
    /// The title of the score.
    pub title: Option<String>,
    /// The subtitle of the score.
    pub subtitle: Option<String>,
    /// The artist of the music in the score.
    pub artist: Option<String>,
    /// The co-artist of the music in the score.
    pub sub_artist: Option<String>,
    /// Who placed the notes into the score.
    pub maker: Option<String>,
    /// The text messages of the score. It may be closed with double quotes.
    pub comment: Option<Vec<String>>,
    /// The email address of the author.
    pub email: Option<String>,
    /// The url of the author.
    pub url: Option<String>,
    /// The play level of the score.
    pub play_level: Option<u8>,
    /// The judgement level of the score.
    pub rank: Option<JudgeLevel>,
    /// The difficulty of the score.
    pub difficulty: Option<u8>,
    /// The total gauge percentage when all notes is got as PERFECT.
    pub total: Option<Decimal>,
    /// The volume of the score.
    pub volume: Volume,
    /// The LN notation type of the score.
    pub ln_type: LnType,
    /// The path of background image, which is shown while playing the score.
    /// This image is displayed behind the gameplay area.
    pub back_bmp: Option<PathBuf>,
    /// The path of splash screen image, which is shown before playing the score.
    /// This image is displayed during the loading screen.
    pub stage_file: Option<PathBuf>,
    /// The path of banner image.
    /// This image is used in music selection screens.
    pub banner: Option<PathBuf>,
    /// LN Mode. Defines the long note mode for this chart.
    /// - 1: LN (Long Note)
    /// - 2: CN (Charge Note)
    /// - 3: HCN (Hell Charge Note)
    pub ln_mode: LnMode,
    /// Preview Music. Defines the preview audio file for music selection.
    /// This file is played when hovering over the song in the music select screen.
    pub preview_music: Option<PathBuf>,
    /// Movie Define. Defines the global video file for the chart.
    /// - Video starts from section #000
    /// - Priority rules apply when conflicting with #xxx04
    /// - No loop, stays on last frame after playback
    /// - Audio track in video is not played
    pub movie: Option<PathBuf>,
}

/// Stores the original scope-defines like `#WAVXX`. Using [`HashMap`].
/// Only stores the original scope-defines, not the parsed ones.
/// Only stores which affects playing.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScopeDefines {
    /// BPM change definitions, indexed by [`ObjId`]. `#BPM[01-ZZ]`
    pub bpm_defs: HashMap<ObjId, Decimal>,
    /// Stop definitions, indexed by [`ObjId`]. `#STOP[01-ZZ]`
    pub stop_defs: HashMap<ObjId, Decimal>,
    /// Scroll speed change definitions, indexed by [`ObjId`]. `#SCROLL[01-ZZ]`
    pub scroll_defs: HashMap<ObjId, Decimal>,
    /// Spacing change definitions, indexed by [`ObjId`]. `#SPEED[01-ZZ]`
    pub speed_defs: HashMap<ObjId, Decimal>,
    /// Storage for #EXRANK definitions
    pub exrank_defs: HashMap<ObjId, ExRankDef>,
    /// Storage for #EXWAV definitions
    #[cfg(feature = "minor-command")]
    pub exwav_defs: HashMap<ObjId, ExWavDef>,
    /// WAVCMD events, indexed by `wav_index`. `#WAVCMD`
    #[cfg(feature = "minor-command")]
    pub wavcmd_events: HashMap<ObjId, WavCmdEvent>,
    /// Storage for #@BGA definitions
    #[cfg(feature = "minor-command")]
    pub atbga_defs: HashMap<ObjId, AtBgaDef>,
    /// Storage for #BGA definitions
    #[cfg(feature = "minor-command")]
    pub bga_defs: HashMap<ObjId, BgaDef>,
    /// SWBGA events, indexed by [`ObjId`]. `#SWBGA`
    #[cfg(feature = "minor-command")]
    pub swbga_events: HashMap<ObjId, SwBgaEvent>,
    /// ARGB definitions, indexed by [`ObjId`]. `#ARGB`
    #[cfg(feature = "minor-command")]
    pub argb_defs: HashMap<ObjId, Argb>,
}

/// The other objects that are used in the score. May be arranged in play.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Others {
    /// The message for overriding options of some BMS player.
    #[cfg(feature = "minor-command")]
    pub options: Option<Vec<String>>,
    /// Whether the score is the octave mode.
    /// In octave mode, the chart may have different note arrangements or gameplay mechanics.
    #[cfg(feature = "minor-command")]
    pub is_octave: bool,
    /// CDDA events, indexed by value. `#CDDA`
    #[cfg(feature = "minor-command")]
    pub cdda: Vec<BigUint>,
    /// Seek events, indexed by [`ObjId`]. `#SEEK`
    #[cfg(feature = "minor-command")]
    pub seek_events: HashMap<ObjId, Decimal>,
    /// Extended-character events. `#ExtChr`
    #[cfg(feature = "minor-command")]
    pub extchr_events: Vec<ExtChrEvent>,
    /// Storage for `#TEXT` definitions
    /// The texts corresponding to the id of the text object.
    pub texts: HashMap<ObjId, String>,
    /// The option messages corresponding to the id of the change option object.
    #[cfg(feature = "minor-command")]
    pub change_options: HashMap<ObjId, String>,
    /// Lines that not starts with `'#'`.
    pub non_command_lines: Vec<String>,
    /// Lines that starts with `'#'`, but not recognized as vaild command.
    pub unknown_command_lines: Vec<String>,
    /// Divide property. #DIVIDEPROP
    #[cfg(feature = "minor-command")]
    pub divide_prop: Option<String>,
    /// Material path definition. #MATERIALS
    #[cfg(feature = "minor-command")]
    pub materials_path: Option<PathBuf>,
}

impl<T> Bms<T> {
    /// Returns the sound note objects information.
    #[must_use]
    pub fn notes(&self) -> &Notes<T> {
        &self.notes
    }

    /// Gets the time of last any object including visible, BGM, BPM change, section length change and so on.
    ///
    /// You can't use this to find the length of music. Because this doesn't consider that the length of sound.
    #[must_use]
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        let obj_last = self.notes.last_obj_time();
        let bpm_last = self
            .arrangers
            .bpm_changes
            .last_key_value()
            .map(|(&time, _)| time);
        let section_len_last =
            self.arrangers
                .section_len_changes
                .last_key_value()
                .map(|(&time, _)| ObjTime {
                    track: time,
                    numerator: 0,
                    denominator: 4,
                });
        let stop_last = self.arrangers.stops.last_key_value().map(|(&time, _)| time);
        let bga_last = self
            .graphics
            .bga_changes
            .last_key_value()
            .map(|(&time, _)| time);
        [obj_last, bpm_last, section_len_last, stop_last, bga_last]
            .into_iter()
            .max()
            .flatten()
    }

    /// Calculates a required resolution to convert the notes time into pulses, which split one quarter note evenly.
    #[must_use]
    pub fn resolution_for_pulses(&self) -> u64 {
        use num::Integer;

        let mut hyp_resolution = 1u64;
        for obj in self.notes.all_notes() {
            hyp_resolution = hyp_resolution.lcm(&obj.offset.denominator);
        }
        for bpm_change in self.arrangers.bpm_changes.values() {
            hyp_resolution = hyp_resolution.lcm(&bpm_change.time.denominator);
        }
        hyp_resolution
    }
}
