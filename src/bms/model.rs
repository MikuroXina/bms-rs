//! Header information from parsed BMS file.
//! Note objects manager.

pub mod def;
pub mod obj;

use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Debug,
    marker::PhantomData,
    ops::Bound,
    path::PathBuf,
};

use itertools::Itertools;
#[cfg(feature = "minor-command")]
use num::BigUint;

#[cfg(feature = "minor-command")]
use crate::bms::command::minor_command::{ExtChrEvent, StpEvent, SwBgaEvent, WavCmdEvent};
use crate::bms::{
    Decimal,
    command::{
        JudgeLevel, LnMode, LnType, ObjId, PlayerMode, PoorMode, Volume,
        channel::{
            Channel, Key, NoteKind, PlayerSide,
            converter::KeyLayoutConverter,
            mapper::{KeyLayoutBeat, KeyLayoutMapper},
        },
        graphics::Argb,
        time::{ObjTime, Track},
    },
};

#[cfg(feature = "minor-command")]
use self::def::{AtBgaDef, BgaDef, ExWavDef};
use self::{
    def::{Bmp, ExRankDef},
    obj::{
        BgaLayer, BgaObj, BgmVolumeObj, BpmChangeObj, JudgeObj, KeyVolumeObj, Obj,
        ScrollingFactorObj, SectionLenChangeObj, SpeedObj, StopObj, TextObj,
    },
};

#[cfg(feature = "minor-command")]
use self::obj::{BgaArgbObj, BgaKeyboundObj, BgaOpacityObj, OptionObj, SeekObj};
use super::{
    command::channel::ChannelId,
    parse::{
        Result,
        prompt::{ChannelDuplication, PromptHandler, TrackDuplication},
    },
};

/// A score data aggregate of BMS format.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bms<T: KeyLayoutMapper = KeyLayoutBeat> {
    /// The header data in the score.
    pub header: Header,
    /// The scope-defines in the score.
    pub scope_defines: ScopeDefines,
    /// The arranges in the score. Contains timing and arrangement data like BPM changes, stops, and scrolling factors.
    pub arrangers: Arrangers,
    /// The objects in the score. Contains all note objects, BGM events, and audio file definitions.
    pub(crate) notes: Notes<T>,
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

/// The objects that arrange the playing panel running or showing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Arrangers {
    /// Section length change events, indexed by track. `#SECLEN`
    pub section_len_changes: BTreeMap<Track, SectionLenChangeObj>,
    /// The initial BPM of the score.
    pub bpm: Option<Decimal>,
    /// The BPMs corresponding to the id of the BPM change object.
    /// BPM change events, indexed by time. `#BPM[01-ZZ]` in message
    pub bpm_changes: BTreeMap<ObjTime, BpmChangeObj>,
    /// Record of used BPM change ids from `#BPMxx` messages, for validity checks.
    pub bpm_change_ids_used: HashSet<ObjId>,
    /// Stop lengths by stop object id.
    pub stops: BTreeMap<ObjTime, StopObj>,
    /// Record of used STOP ids from `#STOPxx` messages, for validity checks.
    pub stop_ids_used: HashSet<ObjId>,
    /// The scrolling factors corresponding to the id of the scroll speed change object.
    pub scrolling_factor_changes: BTreeMap<ObjTime, ScrollingFactorObj>,
    /// The spacing factors corresponding to the id of the spacing change object.
    pub speed_factor_changes: BTreeMap<ObjTime, SpeedObj>,
    /// bemaniaDX STP events, indexed by [`ObjTime`]. `#STP`
    #[cfg(feature = "minor-command")]
    pub stp_events: BTreeMap<ObjTime, StpEvent>,
    /// `#BASEBPM` for LR. Replaced by bpm match in LR2.
    #[cfg(feature = "minor-command")]
    pub base_bpm: Option<Decimal>,
}

/// The playable objects set for querying by lane or time.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Notes<T> {
    /// The path to override the base path of the WAV file path.
    /// This allows WAV files to be referenced relative to a different directory.
    pub wav_path_root: Option<PathBuf>,
    /// The WAV file paths corresponding to the id of the note object.
    pub wav_files: HashMap<ObjId, PathBuf>,
    // objects stored in obj is sorted, so it can be searched by bisection method
    /// BGM objects, indexed by time. `#XXX01:ZZ...` (BGM placement)
    pub bgms: BTreeMap<ObjTime, Vec<ObjId>>,
    /// All note objects, indexed by [`ObjId`]. `#XXXYY:ZZ...` (note placement)
    pub objs: HashMap<ObjId, Vec<Obj>>,
    /// Index for fast key lookup. Used for LN/landmine logic and so on.
    /// Maps each [`ChannelId`] to a sorted map of times and [`ObjId`]s for efficient note lookup.
    pub ids_by_channel: HashMap<ChannelId, BTreeMap<ObjTime, ObjId>>,
    /// The path of MIDI file, which is played as BGM while playing the score.
    #[cfg(feature = "minor-command")]
    pub midi_file: Option<PathBuf>,
    /// Material WAV file paths. #MATERIALSWAV
    #[cfg(feature = "minor-command")]
    pub materials_wav: Vec<PathBuf>,
    /// BGM volume change events, indexed by time. #97
    pub bgm_volume_changes: BTreeMap<ObjTime, BgmVolumeObj>,
    /// KEY volume change events, indexed by time. #98
    pub key_volume_changes: BTreeMap<ObjTime, KeyVolumeObj>,
    /// Seek events, indexed by time. #05
    #[cfg(feature = "minor-command")]
    pub seek_events: BTreeMap<ObjTime, SeekObj>,
    /// Text events, indexed by time. #99
    pub text_events: BTreeMap<ObjTime, TextObj>,
    /// Judge events, indexed by time. #A0
    pub judge_events: BTreeMap<ObjTime, JudgeObj>,
    /// BGA keybound events, indexed by time. #A5
    #[cfg(feature = "minor-command")]
    pub bga_keybound_events: BTreeMap<ObjTime, BgaKeyboundObj>,
    /// Option events, indexed by time. #A6
    #[cfg(feature = "minor-command")]
    pub option_events: BTreeMap<ObjTime, OptionObj>,
    _marker: PhantomData<fn() -> T>,
}

/// The graphics objects that are used in the score.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Graphics {
    /// The path of the background video. The video should be started the playing from the section 000.
    /// This video is displayed behind the gameplay area.
    /// Its audio track should not be played.
    pub video_file: Option<PathBuf>,
    /// The BMP file paths corresponding to the id of the background image/video object.
    pub bmp_files: HashMap<ObjId, Bmp>,
    /// BGA change events, indexed by time. #BGA, #BGAPOOR, #BGALAYER
    pub bga_changes: BTreeMap<ObjTime, BgaObj>,
    /// The path of image, which is shown when the player got POOR.
    /// This image is displayed when the player misses a note or gets a poor judgment.
    pub poor_bmp: Option<PathBuf>,
    /// The display mode for background image/video.
    pub poor_bga_mode: PoorMode,
    /// Material BMP file paths. #MATERIALSBMP
    #[cfg(feature = "minor-command")]
    pub materials_bmp: Vec<PathBuf>,
    /// Character file path. #CHARFILE
    #[cfg(feature = "minor-command")]
    pub char_file: Option<PathBuf>,
    /// Video color depth. #VIDEOCOLORS
    #[cfg(feature = "minor-command")]
    pub video_colors: Option<u8>,
    /// Video delay. #VIDEODLY
    #[cfg(feature = "minor-command")]
    pub video_dly: Option<Decimal>,
    /// Video frame rate. #VIDEOF/S
    #[cfg(feature = "minor-command")]
    pub video_fs: Option<Decimal>,
    /// BGA opacity change events, indexed by time. #0B, #0C, #0D, #0E
    #[cfg(feature = "minor-command")]
    pub bga_opacity_changes: HashMap<BgaLayer, BTreeMap<ObjTime, BgaOpacityObj>>,
    /// BGA ARGB color change events, indexed by time. #A1, #A2, #A3, #A4
    #[cfg(feature = "minor-command")]
    pub bga_argb_changes: HashMap<BgaLayer, BTreeMap<ObjTime, BgaArgbObj>>,
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

impl<T: KeyLayoutMapper> Bms<T> {
    /// Returns the sound note objects information.
    pub fn notes(&self) -> &Notes<T> {
        &self.notes
    }

    /// Gets the time of last any object including visible, BGM, BPM change, section length change and so on.
    ///
    /// You can't use this to find the length of music. Because this doesn't consider that the length of sound.
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        let obj_last = self
            .notes
            .objs
            .values()
            .flatten()
            .map(Reverse)
            .sorted()
            .next()
            .map(|Reverse(obj)| obj.offset);
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
        for obj in self.notes.objs.values().flatten() {
            hyp_resolution = hyp_resolution.lcm(&obj.offset.denominator);
        }
        for bpm_change in self.arrangers.bpm_changes.values() {
            hyp_resolution = hyp_resolution.lcm(&bpm_change.time.denominator);
        }
        hyp_resolution
    }
}

impl Arrangers {
    /// Adds a new BPM change object to the notes.
    pub fn push_bpm_change(
        &mut self,
        bpm_change: BpmChangeObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.bpm_changes.entry(bpm_change.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(bpm_change);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::BpmChangeEvent {
                        time: bpm_change.time,
                        older: existing,
                        newer: &bpm_change,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        bpm_change.clone(),
                        bpm_change.time,
                        Channel::BpmChange,
                    )
            }
        }
    }

    /// Adds a new scrolling factor change object to the notes.
    pub fn push_scrolling_factor_change(
        &mut self,
        scrolling_factor_change: ScrollingFactorObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self
            .scrolling_factor_changes
            .entry(scrolling_factor_change.time)
        {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(scrolling_factor_change);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::ScrollingFactorChangeEvent {
                        time: scrolling_factor_change.time,
                        older: existing,
                        newer: &scrolling_factor_change,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        scrolling_factor_change.clone(),
                        scrolling_factor_change.time,
                        Channel::Scroll,
                    )
            }
        }
    }

    /// Adds a new spacing factor change object to the notes.
    pub fn push_speed_factor_change(
        &mut self,
        speed_factor_change: SpeedObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.speed_factor_changes.entry(speed_factor_change.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(speed_factor_change);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::SpeedFactorChangeEvent {
                        time: speed_factor_change.time,
                        older: existing,
                        newer: &speed_factor_change,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        speed_factor_change.clone(),
                        speed_factor_change.time,
                        Channel::Speed,
                    )
            }
        }
    }

    /// Adds a new section length change object to the notes.
    pub fn push_section_len_change(
        &mut self,
        section_len_change: SectionLenChangeObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.section_len_changes.entry(section_len_change.track) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(section_len_change);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_track_duplication(TrackDuplication::SectionLenChangeEvent {
                        track: section_len_change.track,
                        older: existing,
                        newer: &section_len_change,
                    })
                    .apply_track(
                        entry.get_mut(),
                        section_len_change.clone(),
                        section_len_change.track,
                        Channel::SectionLen,
                    )
            }
        }
    }

    /// Adds a new stop object to the notes.
    pub fn push_stop(&mut self, stop: StopObj) {
        self.stops
            .entry(stop.time)
            .and_modify(|existing| {
                existing.duration = &existing.duration + &stop.duration;
            })
            .or_insert_with(|| stop);
    }
}

impl Graphics {
    /// Returns the bga change objects.
    #[must_use]
    pub const fn bga_changes(&self) -> &BTreeMap<ObjTime, BgaObj> {
        &self.bga_changes
    }

    /// Adds a new bga change object to the notes.
    pub fn push_bga_change(
        &mut self,
        bga: BgaObj,
        channel: Channel,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.bga_changes.entry(bga.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(bga);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::BgaChangeEvent {
                        time: bga.time,
                        older: existing,
                        newer: &bga,
                    })
                    .apply_channel(entry.get_mut(), bga, bga.time, channel)
            }
        }
    }

    /// Adds a new BGA opacity change object to the graphics.
    #[cfg(feature = "minor-command")]
    pub fn push_bga_opacity_change(
        &mut self,
        opacity_obj: BgaOpacityObj,
        channel: Channel,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        let this_layer_map = self
            .bga_opacity_changes
            .entry(opacity_obj.layer)
            .or_default();
        match this_layer_map.entry(opacity_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(opacity_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::BgaOpacityChangeEvent {
                        time: opacity_obj.time,
                        older: existing,
                        newer: &opacity_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        opacity_obj.clone(),
                        opacity_obj.time,
                        channel,
                    )
            }
        }
    }

    /// Adds a new BGA ARGB color change object to the graphics.
    #[cfg(feature = "minor-command")]
    pub fn push_bga_argb_change(
        &mut self,
        argb_obj: BgaArgbObj,
        channel: Channel,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        let this_layer_map = self.bga_argb_changes.entry(argb_obj.layer).or_default();
        match this_layer_map.entry(argb_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(argb_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::BgaArgbChangeEvent {
                        time: argb_obj.time,
                        older: existing,
                        newer: &argb_obj,
                    })
                    .apply_channel(entry.get_mut(), argb_obj.clone(), argb_obj.time, channel)
            }
        }
    }
}

impl<T> Default for Notes<T> {
    fn default() -> Self {
        Self {
            wav_path_root: Default::default(),
            wav_files: Default::default(),
            bgms: Default::default(),
            objs: Default::default(),
            ids_by_channel: Default::default(),
            midi_file: Default::default(),
            materials_wav: Default::default(),
            bgm_volume_changes: Default::default(),
            key_volume_changes: Default::default(),
            seek_events: Default::default(),
            text_events: Default::default(),
            judge_events: Default::default(),
            bga_keybound_events: Default::default(),
            option_events: Default::default(),
            _marker: PhantomData,
        }
    }
}

impl<T> Notes<T> {
    /// Converts into the notes sorted by time.
    #[must_use]
    pub fn into_all_notes(self) -> Vec<Obj> {
        self.objs.into_values().flatten().sorted().collect()
    }

    /// Returns the iterator having all of the notes sorted by time.
    pub fn all_notes(&self) -> impl Iterator<Item = &Obj> {
        self.objs.values().flatten().sorted()
    }

    /// Returns all the bgms in the score.
    #[must_use]
    pub const fn bgms(&self) -> &BTreeMap<ObjTime, Vec<ObjId>> {
        &self.bgms
    }
}

impl<T: KeyLayoutMapper> Notes<T> {
    /// Finds next object on the key `Key` from the time `ObjTime`.
    #[must_use]
    pub fn next_obj_by_key(
        &self,
        side: PlayerSide,
        kind: NoteKind,
        key: Key,
        time: ObjTime,
    ) -> Option<&Obj> {
        let id = T::new(side, kind, key).to_channel_id();
        self.ids_by_channel
            .get(&id)?
            .range((Bound::Excluded(time), Bound::Unbounded))
            .next()
            .and_then(|(_, id)| {
                let objs = self.objs.get(id)?;
                let idx = objs
                    .binary_search_by(|probe| probe.offset.cmp(&time))
                    .unwrap_or_else(|idx| idx);
                objs.get(idx)
            })
    }

    /// Adds the new note object to the notes.
    pub fn push_note(&mut self, note: Obj) {
        let entry_key = T::new(note.side, note.kind, note.key).to_channel_id();
        let offset = note.offset;
        let obj = note.obj;
        self.objs.entry(obj).or_default().push(note);
        self.ids_by_channel
            .entry(entry_key)
            .or_default()
            .insert(offset, obj);
    }

    /// Removes the latest note from the notes.
    pub fn remove_latest_note(&mut self, id: ObjId) -> Option<Obj> {
        self.objs.entry(id).or_default().pop().inspect(|removed| {
            let entry_key = T::new(removed.side, removed.kind, removed.key).to_channel_id();
            if let Some(key_map) = self.ids_by_channel.get_mut(&entry_key) {
                key_map.remove(&removed.offset);
            }
        })
    }

    /// Removes the note from the notes.
    pub fn remove_note(&mut self, id: ObjId) -> Vec<Obj> {
        self.objs.remove(&id).map_or(vec![], |removed| {
            for item in &removed {
                let entry_key = T::new(item.side, item.kind, item.key).to_channel_id();
                if let Some(key_map) = self.ids_by_channel.get_mut(&entry_key) {
                    key_map.remove(&item.offset);
                }
            }
            removed
        })
    }

    /// Gets the time of last visible object.
    pub fn last_visible_time(&self) -> Option<ObjTime> {
        self.objs
            .values()
            .flatten()
            .filter(|obj| !matches!(obj.kind, NoteKind::Invisible))
            .map(Reverse)
            .sorted()
            .next()
            .map(|Reverse(obj)| obj.offset)
    }

    /// Gets the time of last BGM object.
    ///
    /// You can't use this to find the length of music. Because this doesn't consider that the length of sound. And visible notes may ring after all BGMs.
    #[must_use]
    pub fn last_bgm_time(&self) -> Option<ObjTime> {
        self.bgms.last_key_value().map(|(time, _)| time).cloned()
    }

    /// Adds a new BGM volume change object to the notes.
    pub fn push_bgm_volume_change(
        &mut self,
        volume_obj: BgmVolumeObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.bgm_volume_changes.entry(volume_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(volume_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::BgmVolumeChangeEvent {
                        time: volume_obj.time,
                        older: existing,
                        newer: &volume_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        volume_obj.clone(),
                        volume_obj.time,
                        Channel::BgmVolume,
                    )
            }
        }
    }

    /// Adds a new KEY volume change object to the notes.
    pub fn push_key_volume_change(
        &mut self,
        volume_obj: KeyVolumeObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.key_volume_changes.entry(volume_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(volume_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::KeyVolumeChangeEvent {
                        time: volume_obj.time,
                        older: existing,
                        newer: &volume_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        volume_obj.clone(),
                        volume_obj.time,
                        Channel::KeyVolume,
                    )
            }
        }
    }

    /// Adds a new seek object to the notes.
    #[cfg(feature = "minor-command")]
    pub fn push_seek_event(
        &mut self,
        seek_obj: SeekObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.seek_events.entry(seek_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(seek_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::SeekMessageEvent {
                        time: seek_obj.time,
                        older: existing,
                        newer: &seek_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        seek_obj.clone(),
                        seek_obj.time,
                        Channel::Seek,
                    )
            }
        }
    }

    /// Adds a new text object to the notes.
    pub fn push_text_event(
        &mut self,
        text_obj: TextObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.text_events.entry(text_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(text_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::TextEvent {
                        time: text_obj.time,
                        older: existing,
                        newer: &text_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        text_obj.clone(),
                        text_obj.time,
                        Channel::Text,
                    )
            }
        }
    }

    /// Adds a new judge object to the notes.
    pub fn push_judge_event(
        &mut self,
        judge_obj: JudgeObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.judge_events.entry(judge_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(judge_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::JudgeEvent {
                        time: judge_obj.time,
                        older: existing,
                        newer: &judge_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        judge_obj.clone(),
                        judge_obj.time,
                        Channel::Judge,
                    )
            }
        }
    }

    /// Adds a new BGA keybound object to the notes.
    #[cfg(feature = "minor-command")]
    pub fn push_bga_keybound_event(
        &mut self,
        keybound_obj: BgaKeyboundObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.bga_keybound_events.entry(keybound_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(keybound_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::BgaKeyboundEvent {
                        time: keybound_obj.time,
                        older: existing,
                        newer: &keybound_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        keybound_obj.clone(),
                        keybound_obj.time,
                        Channel::BgaKeybound,
                    )
            }
        }
    }

    /// Adds a new option object to the notes.
    #[cfg(feature = "minor-command")]
    pub fn push_option_event(
        &mut self,
        option_obj: OptionObj,
        prompt_handler: &mut impl PromptHandler,
    ) -> Result<()> {
        match self.option_events.entry(option_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(option_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompt_handler
                    .handle_channel_duplication(ChannelDuplication::OptionEvent {
                        time: option_obj.time,
                        older: existing,
                        newer: &option_obj,
                    })
                    .apply_channel(
                        entry.get_mut(),
                        option_obj.clone(),
                        option_obj.time,
                        Channel::Option,
                    )
            }
        }
    }
}

impl<T: KeyLayoutMapper> Bms<T> {
    /// One-way converting ([`crate::bms::command::channel::PlayerSide`], [`crate::bms::command::channel::Key`]) with [`KeyLayoutConverter`].
    pub fn convert_key(&mut self, mut converter: impl KeyLayoutConverter) {
        for objs in self.notes.objs.values_mut() {
            for Obj {
                side, kind, key, ..
            } in objs.iter_mut()
            {
                let beat_map = T::new(*side, *kind, *key);
                let new_beat_map = converter.convert(beat_map);
                *side = new_beat_map.side();
                *key = new_beat_map.key();
            }
        }
    }
}
