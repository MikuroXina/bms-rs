//! Note objects manager.

use fraction::GenericFraction;
use itertools::Itertools;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
    ops::Bound,
    str::FromStr,
};

use super::{ParseWarning, Result, header::Header, obj::Obj};
use crate::{
    bms::Decimal,
    lex::{
        command::{self, Channel, Key, NoteKind, ObjId},
        token::Token,
    },
    parse::header::{ExRankDef, ExWavDef},
    time::{ObjTime, Track},
};

#[cfg(feature = "minor-command")]
use crate::lex::command::{Argb, StpEvent, SwBgaEvent, WavCmdEvent};

/// An object to change the BPM of the score.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BpmChangeObj {
    /// The time to begin the change of BPM.
    pub time: ObjTime,
    /// The BPM to be.
    pub bpm: Decimal,
}

impl PartialEq for BpmChangeObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for BpmChangeObj {}

impl PartialOrd for BpmChangeObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BpmChangeObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// An object to change its section length of the score.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionLenChangeObj {
    /// The target track to change.
    pub track: Track,
    /// The length to be.
    pub length: Decimal,
}

impl PartialEq for SectionLenChangeObj {
    fn eq(&self, other: &Self) -> bool {
        self.track == other.track
    }
}

impl Eq for SectionLenChangeObj {}

impl PartialOrd for SectionLenChangeObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SectionLenChangeObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.track.cmp(&other.track)
    }
}

/// An object to stop scrolling of score.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StopObj {
    /// Time to start the stop.
    pub time: ObjTime,
    /// Object duration how long stops scrolling of score.
    ///
    /// Note that the duration of stopping will not be changed by a current measure length but BPM.
    pub duration: Decimal,
}

impl PartialEq for StopObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for StopObj {}

impl PartialOrd for StopObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StopObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// An object to change the image for BGA (background animation).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BgaObj {
    /// Time to start to display the image.
    pub time: ObjTime,
    /// Identifier represents the image/video file registered in [`Header`].
    pub id: ObjId,
    /// Layer to display.
    pub layer: BgaLayer,
}

impl PartialEq for BgaObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for BgaObj {}

impl PartialOrd for BgaObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BgaObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// A layer where the image for BGA to be displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum BgaLayer {
    /// The lowest layer.
    Base,
    /// Layer which is displayed only if a player missed to play notes.
    Poor,
    /// An overlaying layer.
    Overlay,
}

/// An object to change scrolling factor of the score.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScrollingFactorObj {
    /// The time to begin the change of BPM.
    pub time: ObjTime,
    /// The scrolling factor to be.
    pub factor: Decimal,
}

impl PartialEq for ScrollingFactorObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for ScrollingFactorObj {}

impl PartialOrd for ScrollingFactorObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScrollingFactorObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// An object to change spacing factor between notes with linear interpolation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpacingFactorObj {
    /// The time to begin the change of BPM.
    pub time: ObjTime,
    /// The spacing factor to be.
    pub factor: Decimal,
}

impl PartialEq for SpacingFactorObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for SpacingFactorObj {}

impl PartialOrd for SpacingFactorObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SpacingFactorObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// An extended object on the score.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExtendedMessageObj {
    /// The track which the message is on.
    pub track: Track,
    /// The channel which the message is on.
    pub channel: Channel,
    /// The extended message.
    pub message: String,
}

impl PartialEq for ExtendedMessageObj {
    fn eq(&self, other: &Self) -> bool {
        self.track == other.track
    }
}

impl Eq for ExtendedMessageObj {}

impl PartialOrd for ExtendedMessageObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExtendedMessageObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.track.cmp(&other.track)
    }
}

/// The objects set for querying by lane or time.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Notes {
    // objects stored in obj is sorted, so it can be searched by bisection method
    /// All note objects, indexed by ObjId. #XXXYY:ZZ... (note placement)
    pub objs: HashMap<ObjId, Vec<Obj>>,
    /// BGM objects, indexed by time. #XXX01:ZZ... (BGM placement)
    pub bgms: BTreeMap<ObjTime, Vec<ObjId>>,
    /// Index for fast key lookup. Used for LN/landmine logic.
    pub ids_by_key: HashMap<Key, BTreeMap<ObjTime, ObjId>>,
    /// BPM change events, indexed by time. #BPM[01-ZZ] in message
    pub bpm_changes: BTreeMap<ObjTime, BpmChangeObj>,
    /// Section length change events, indexed by track. #SECLEN
    pub section_len_changes: BTreeMap<Track, SectionLenChangeObj>,
    /// Stop events, indexed by time. #STOP[01-ZZ] in message
    pub stops: BTreeMap<ObjTime, StopObj>,
    /// BGA change events, indexed by time. #BGA, #BGAPOOR, #BGALAYER
    pub bga_changes: BTreeMap<ObjTime, BgaObj>,
    /// Scrolling factor change events, indexed by time. #SCROLL in message
    pub scrolling_factor_changes: BTreeMap<ObjTime, ScrollingFactorObj>,
    /// Spacing factor change events, indexed by time. #SPEED in message
    pub spacing_factor_changes: BTreeMap<ObjTime, SpacingFactorObj>,
    /// Extended message events. #EXT
    pub extended_messages: Vec<ExtendedMessageObj>,
    /// Storage for #EXRANK definitions
    pub exrank_defs: HashMap<ObjId, ExRankDef>,
    /// Storage for #EXWAV definitions
    pub exwav_defs: HashMap<ObjId, ExWavDef>,
    /// Storage for #CHANGEOPTION definitions
    pub change_options: HashMap<ObjId, String>,
    /// Storage for #TEXT definitions
    pub texts: HashMap<ObjId, String>,
    /// bemaniaDX STP events, indexed by ObjTime. #STP
    #[cfg(feature = "minor-command")]
    pub stp_events: HashMap<ObjTime, StpEvent>,
    /// WAVCMD events, indexed by wav_index. #WAVCMD
    #[cfg(feature = "minor-command")]
    pub wavcmd_events: HashMap<ObjId, WavCmdEvent>,
    /// CDDA events, indexed by value. #CDDA
    #[cfg(feature = "minor-command")]
    pub cdda_events: HashMap<u64, u64>,
    /// SWBGA events, indexed by ObjId. #SWBGA
    #[cfg(feature = "minor-command")]
    pub swbga_events: HashMap<ObjId, SwBgaEvent>,
    /// ARGB definitions, indexed by ObjId. #ARGB
    #[cfg(feature = "minor-command")]
    pub argb_defs: HashMap<ObjId, Argb>,
    /// Seek events, indexed by ObjId. #SEEK
    #[cfg(feature = "minor-command")]
    pub seek_events: HashMap<ObjId, Decimal>,
}

impl Notes {
    /// Creates a new notes dictionary.
    pub fn new() -> Self {
        Default::default()
    }

    /// Converts into the notes sorted by time.
    pub fn into_all_notes(self) -> Vec<Obj> {
        self.objs.into_values().flatten().sorted().collect()
    }

    /// Returns the iterator having all of the notes sorted by time.
    pub fn all_notes(&self) -> impl Iterator<Item = &Obj> {
        self.objs.values().flatten().sorted()
    }

    /// Returns all the bgms in the score.
    pub fn bgms(&self) -> &BTreeMap<ObjTime, Vec<ObjId>> {
        &self.bgms
    }

    /// Returns the bpm change objects.
    pub fn bpm_changes(&self) -> &BTreeMap<ObjTime, BpmChangeObj> {
        &self.bpm_changes
    }

    /// Returns the scrolling factor change objects.
    pub fn scrolling_factor_changes(&self) -> &BTreeMap<ObjTime, ScrollingFactorObj> {
        &self.scrolling_factor_changes
    }

    /// Returns the spacing factor change objects.
    pub fn spacing_factor_changes(&self) -> &BTreeMap<ObjTime, SpacingFactorObj> {
        &self.spacing_factor_changes
    }

    /// Returns the section len change objects.
    pub fn section_len_changes(&self) -> &BTreeMap<Track, SectionLenChangeObj> {
        &self.section_len_changes
    }

    /// Returns the scroll stop objects.
    pub fn stops(&self) -> &BTreeMap<ObjTime, StopObj> {
        &self.stops
    }

    /// Returns the bga change objects.
    pub fn bga_changes(&self) -> &BTreeMap<ObjTime, BgaObj> {
        &self.bga_changes
    }

    /// Finds next object on the key `Key` from the time `ObjTime`.
    pub fn next_obj_by_key(&self, key: Key, time: ObjTime) -> Option<&Obj> {
        self.ids_by_key
            .get(&key)?
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
        self.objs.entry(note.obj).or_default().push(note.clone());
        self.ids_by_key
            .entry(note.key)
            .or_default()
            .insert(note.offset, note.obj);
    }

    /// Removes the latest note from the notes.
    pub fn remove_latest_note(&mut self, id: ObjId) -> Option<Obj> {
        self.objs.entry(id).or_default().pop().inspect(|removed| {
            self.ids_by_key
                .get_mut(&removed.key)
                .unwrap()
                .remove(&removed.offset)
                .unwrap();
        })
    }

    /// Removes the note from the notes.
    pub fn remove_note(&mut self, id: ObjId) -> Vec<Obj> {
        self.objs.remove(&id).map_or(vec![], |removed| {
            for item in &removed {
                self.ids_by_key
                    .get_mut(&item.key)
                    .unwrap()
                    .remove(&item.offset)
                    .unwrap();
            }
            removed
        })
    }

    /// Adds a new BPM change object to the notes.
    pub fn push_bpm_change(&mut self, bpm_change: BpmChangeObj) {
        if let Some(existing) = self.bpm_changes.insert(bpm_change.time, bpm_change) {
            eprintln!(
                "duplicate bpm change object detected at {:?}",
                existing.time
            );
        }
    }

    /// Adds a new scrolling factor change object to the notes.
    pub fn push_scrolling_factor_change(&mut self, bpm_change: ScrollingFactorObj) {
        if let Some(existing) = self
            .scrolling_factor_changes
            .insert(bpm_change.time, bpm_change.clone())
        {
            eprintln!(
                "duplicate scrolling factor change object detected at {:?}",
                existing.time
            );
        }
    }

    /// Adds a new spacing factor change object to the notes.
    pub fn push_spacing_factor_change(&mut self, bpm_change: SpacingFactorObj) {
        if let Some(existing) = self
            .spacing_factor_changes
            .insert(bpm_change.time, bpm_change.clone())
        {
            eprintln!(
                "duplicate spacing factor change object detected at {:?}",
                existing.time
            );
        }
    }

    /// Adds a new section length change object to the notes.
    pub fn push_section_len_change(&mut self, section_len_change: SectionLenChangeObj) {
        if let Some(existing) = self
            .section_len_changes
            .insert(section_len_change.track, section_len_change.clone())
        {
            eprintln!(
                "duplicate section length change object detected at {:?}",
                existing.track
            );
        }
    }

    /// Adds a new stop object to the notes.
    pub fn push_stop(&mut self, stop: StopObj) {
        self.stops
            .entry(stop.time)
            .and_modify(|existing| {
                existing.duration = existing.duration.clone() + stop.duration.clone();
            })
            .or_insert(stop.clone());
    }

    /// Adds a new bga change object to the notes.
    pub fn push_bga_change(&mut self, bga: BgaObj) {
        if let Some(existing) = self.bga_changes.insert(bga.time, bga) {
            eprintln!(
                "duplicate bga change object detected at {:?}",
                existing.time
            );
        }
    }

    /// Adds the new extended message object to the notes.
    pub fn push_extended_message(&mut self, message: ExtendedMessageObj) {
        self.extended_messages.push(message);
    }

    pub(crate) fn parse(&mut self, token: &Token, header: &Header) -> Result<()> {
        match token {
            Token::Message {
                track,
                channel: Channel::BpmChange,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let bpm = header
                        .bpm_changes
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    self.push_bpm_change(BpmChangeObj {
                        time,
                        bpm: bpm.clone(),
                    });
                }
            }
            Token::Message {
                track,
                channel: Channel::BpmChangeU8,
                message,
            } => {
                let denominator = message.len() as u64 / 2;
                for (i, (c1, c2)) in message.chars().tuples().enumerate() {
                    let bpm = c1.to_digit(16).unwrap() * 16 + c2.to_digit(16).unwrap();
                    if bpm == 0 {
                        continue;
                    }
                    let time = ObjTime::new(track.0, i as u64, denominator);
                    self.push_bpm_change(BpmChangeObj {
                        time,
                        bpm: Decimal::from(bpm),
                    });
                }
            }
            Token::Message {
                track,
                channel: Channel::Scroll,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let factor = header
                        .scrolling_factor_changes
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    self.push_scrolling_factor_change(ScrollingFactorObj {
                        time,
                        factor: factor.clone(),
                    });
                }
            }
            Token::Message {
                track,
                channel: Channel::Speed,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let factor = header
                        .spacing_factor_changes
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    self.push_spacing_factor_change(SpacingFactorObj {
                        time,
                        factor: factor.clone(),
                    });
                }
            }
            Token::Message {
                track,
                channel: Channel::ChangeOption,
                message,
            } => {
                for (_time, obj) in ids_from_message(*track, message) {
                    let _option = header
                        .change_options
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    // Here we can add logic to handle ChangeOption
                    // Currently just ignored because change_options are already stored in header
                }
            }
            Token::Message {
                track,
                channel: Channel::SectionLen,
                message,
            } => {
                let track = Track(track.0);
                let length = Decimal::from(Decimal::from_fraction(
                    GenericFraction::from_str(message).expect("f64 as section length"),
                ));
                assert!(
                    length > Decimal::from(0u64),
                    "section length must be greater than zero"
                );
                self.push_section_len_change(SectionLenChangeObj { track, length });
            }
            Token::Message {
                track,
                channel: Channel::Stop,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let duration = header
                        .stops
                        .get(&obj)
                        .ok_or(ParseWarning::UndefinedObject(obj))?;
                    self.push_stop(StopObj {
                        time,
                        duration: duration.clone(),
                    })
                }
            }
            Token::Message {
                track,
                channel: channel @ (Channel::BgaBase | Channel::BgaPoor | Channel::BgaLayer),
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    if !header.bmp_files.contains_key(&obj) {
                        return Err(ParseWarning::UndefinedObject(obj));
                    }
                    let layer = match channel {
                        Channel::BgaBase => BgaLayer::Base,
                        Channel::BgaPoor => BgaLayer::Poor,
                        Channel::BgaLayer => BgaLayer::Overlay,
                        _ => unreachable!(),
                    };
                    self.push_bga_change(BgaObj {
                        time,
                        id: obj,
                        layer,
                    });
                }
            }
            Token::Message {
                track,
                channel: Channel::Bgm,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    self.bgms.entry(time).or_default().push(obj)
                }
            }
            Token::Message {
                track,
                channel: Channel::Note { kind, side, key },
                message,
            } => {
                for (offset, obj) in ids_from_message(*track, message) {
                    self.push_note(Obj {
                        offset,
                        kind: *kind,
                        side: *side,
                        key: *key,
                        obj,
                    });
                }
            }
            Token::ExtendedMessage {
                track,
                channel,
                message,
            } => {
                let track = Track(track.0);
                self.push_extended_message(ExtendedMessageObj {
                    track,
                    channel: channel.clone(),
                    message: (*message).to_owned(),
                });
            }
            &Token::LnObj(end_id) => {
                let mut end_note = self
                    .remove_latest_note(end_id)
                    .ok_or(ParseWarning::UndefinedObject(end_id))?;
                let Obj { offset, key, .. } = &end_note;
                let (_, &begin_id) =
                    self.ids_by_key[key].range(..offset).last().ok_or_else(|| {
                        ParseWarning::SyntaxError(format!(
                            "expected preceding object for #LNOBJ {end_id:?}",
                        ))
                    })?;
                let mut begin_note = self.remove_latest_note(begin_id).unwrap();
                begin_note.kind = NoteKind::Long;
                end_note.kind = NoteKind::Long;
                self.push_note(begin_note);
                self.push_note(end_note);
            }
            Token::ExRank(id, judge_level) => {
                self.exrank_defs.insert(
                    *id,
                    ExRankDef {
                        id: *id,
                        judge_level: *judge_level,
                    },
                );
            }
            Token::ExWav {
                id,
                pan,
                volume,
                frequency,
                path,
            } => {
                self.exwav_defs.insert(
                    *id,
                    ExWavDef {
                        id: *id,
                        pan: *pan,
                        volume: *volume,
                        frequency: *frequency,
                        path: path.into(),
                    },
                );
            }
            Token::ChangeOption(id, option) => {
                self.change_options.insert(*id, (*option).to_string());
            }
            Token::Text(id, text) => {
                self.texts.insert(*id, (*text).to_string());
            }
            #[cfg(feature = "minor-command")]
            Token::Stp(ev) => {
                // Store by ObjTime as key, report error if duplicated
                let key = ev.time;
                if self.stp_events.contains_key(&key) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated STP event at time {key:?}"
                    )));
                }
                self.stp_events.insert(key, *ev);
            }
            #[cfg(feature = "minor-command")]
            Token::WavCmd(ev) => {
                // Store by wav_index as key, report error if duplicated
                let key = ev.wav_index;
                if self.wavcmd_events.contains_key(&key) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated WAVCMD event for wav_index {key:?}",
                    )));
                }
                self.wavcmd_events.insert(key, *ev);
            }
            #[cfg(feature = "minor-command")]
            Token::SwBga(id, ev) => {
                if self.swbga_events.contains_key(id) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated SWBGA event for id {id:?}",
                    )));
                }
                self.swbga_events.insert(*id, ev.clone());
            }
            #[cfg(feature = "minor-command")]
            Token::Argb(id, argb) => {
                if self.argb_defs.contains_key(id) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated ARGB definition for id {id:?}",
                    )));
                }
                self.argb_defs.insert(*id, *argb);
            }
            #[cfg(feature = "minor-command")]
            Token::Seek(id, v) => {
                if self.seek_events.contains_key(id) {
                    return Err(super::ParseWarning::SyntaxError(format!(
                        "Duplicated Seek event for id {id:?}",
                    )));
                }
                self.seek_events.insert(*id, v.clone());
            }
            // Control flow
            Token::Random(_)
            | Token::SetRandom(_)
            | Token::If(_)
            | Token::ElseIf(_)
            | Token::Else
            | Token::EndIf
            | Token::EndRandom
            | Token::Switch(_)
            | Token::SetSwitch(_)
            | Token::Case(_)
            | Token::Def
            | Token::Skip
            | Token::EndSwitch => {
                unreachable!()
            }
            Token::Email(_)
            | Token::Url(_)
            | Token::Option(_)
            | Token::PathWav(_)
            | Token::Maker(_)
            | Token::PoorBga(_)
            | Token::VideoFile(_)
            | Token::Artist(_)
            | Token::Banner(_)
            | Token::BackBmp(_)
            | Token::Base62
            | Token::Bmp(_, _)
            | Token::Bpm(_)
            | Token::BpmChange(_, _)
            | Token::Comment(_)
            | Token::Difficulty(_)
            | Token::ExBmp(_, _, _)
            | Token::Genre(_)
            | Token::LnTypeRdm
            | Token::LnTypeMgq
            | Token::Player(_)
            | Token::PlayLevel(_)
            | Token::Rank(_)
            | Token::Scroll(_, _)
            | Token::Speed(_, _)
            | Token::StageFile(_)
            | Token::Stop(_, _)
            | Token::SubArtist(_)
            | Token::SubTitle(_)
            | Token::Title(_)
            | Token::Total(_)
            | Token::VolWav(_)
            | Token::Wav(_, _) => {
                // These tokens don't need to be processed in Notes::parse, they should be handled in Header::parse
            }
            Token::Charset(_)
            | Token::DefExRank(_)
            | Token::Preview(_)
            | Token::LnMode(_)
            | Token::Movie(_) => {
                // These tokens are not stored in Notes, just ignore
            }
            #[cfg(feature = "minor-command")]
            Token::CharFile(_)
            | Token::BaseBpm(_)
            | Token::AtBga { .. }
            | Token::Bga { .. }
            | Token::OctFp
            | Token::MidiFile(_)
            | Token::ExtChr(_)
            | Token::MaterialsWav(_)
            | Token::MaterialsBmp(_)
            | Token::DivideProp(_)
            | Token::Cdda(_)
            | Token::VideoFs(_)
            | Token::VideoColors(_)
            | Token::VideoDly(_) => {
                // These tokens are not stored in Notes, just ignore
            }
            Token::UnknownCommand(_) | Token::NotACommand(_) => {
                // this token should be handled outside.
            }
        }
        Ok(())
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
    pub fn last_bgm_time(&self) -> Option<ObjTime> {
        self.bgms.last_key_value().map(|(time, _)| time).cloned()
    }

    /// Gets the time of last any object including visible, BGM, BPM change, section length change and so on.
    ///
    /// You can't use this to find the length of music. Because this doesn't consider that the length of sound.
    pub fn last_obj_time(&self) -> Option<ObjTime> {
        let obj_last = self
            .objs
            .values()
            .flatten()
            .map(Reverse)
            .sorted()
            .next()
            .map(|Reverse(obj)| obj.offset);
        let bpm_last = self.bpm_changes.last_key_value().map(|(&time, _)| time);
        let section_len_last =
            self.section_len_changes
                .last_key_value()
                .map(|(&time, _)| ObjTime {
                    track: time,
                    numerator: 0,
                    denominator: 4,
                });
        let stop_last = self.stops.last_key_value().map(|(&time, _)| time);
        let bga_last = self.bga_changes.last_key_value().map(|(&time, _)| time);
        [obj_last, bpm_last, section_len_last, stop_last, bga_last]
            .into_iter()
            .max()
            .flatten()
    }

    /// Calculates a required resolution to convert the notes time into pulses, which split one quarter note evenly.
    pub fn resolution_for_pulses(&self) -> u64 {
        use num::Integer;

        let mut hyp_resolution = 1u64;
        for obj in self.objs.values().flatten() {
            hyp_resolution = hyp_resolution.lcm(&obj.offset.denominator);
        }
        for bpm_change in self.bpm_changes.values() {
            hyp_resolution = hyp_resolution.lcm(&bpm_change.time.denominator);
        }
        hyp_resolution
    }
}

fn ids_from_message(
    track: command::Track,
    message: &'_ str,
) -> impl Iterator<Item = (ObjTime, ObjId)> + '_ {
    let denominator = message.len() as u64 / 2;
    let mut chars = message.chars().tuples().enumerate();
    std::iter::from_fn(move || {
        let (i, c1, c2) = loop {
            let (i, (c1, c2)) = chars.next()?;
            if !(c1 == '0' && c2 == '0') {
                break (i, c1, c2);
            }
        };
        let obj = ObjId::try_from([c1, c2]).expect("invalid object id");
        let time = ObjTime::new(track.0, i as u64, denominator);
        Some((time, obj))
    })
}

#[cfg(feature = "serde")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// A pack of [`Notes`] for serialize/deserialize.
pub struct NotesPack {
    /// Note objects, ring the sound.
    pub objs: Vec<Obj>,
    /// BPM change events.
    pub bpm_changes: Vec<BpmChangeObj>,
    /// Section length change events.
    pub section_len_changes: Vec<SectionLenChangeObj>,
    /// Stop events.
    pub stops: Vec<StopObj>,
    /// BGA change events.
    pub bga_changes: Vec<BgaObj>,
    /// Scrolling factor change events.
    pub scrolling_factor_changes: Vec<ScrollingFactorObj>,
    /// Spacing factor change events.
    pub spacing_factor_changes: Vec<SpacingFactorObj>,
    /// Extended message events.
    pub extended_messages: Vec<ExtendedMessageObj>,
}

#[cfg(feature = "serde")]
impl serde::Serialize for Notes {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        NotesPack {
            objs: self.all_notes().cloned().collect(),
            bpm_changes: self.bpm_changes.values().cloned().collect(),
            section_len_changes: self.section_len_changes.values().cloned().collect(),
            stops: self.stops.values().cloned().collect(),
            bga_changes: self.bga_changes.values().cloned().collect(),
            scrolling_factor_changes: self.scrolling_factor_changes.values().cloned().collect(),
            spacing_factor_changes: self.spacing_factor_changes.values().cloned().collect(),
            extended_messages: self.extended_messages.clone(),
        }
        .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Notes {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let pack = NotesPack::deserialize(deserializer)?;
        let mut objs = HashMap::<ObjId, Vec<Obj>>::new();
        let mut bgms: BTreeMap<ObjTime, Vec<ObjId>> = BTreeMap::new();
        let mut ids_by_key: HashMap<Key, BTreeMap<ObjTime, ObjId>> = HashMap::new();
        for obj in pack.objs {
            if matches!(obj.kind, NoteKind::Invisible) {
                bgms.entry(obj.offset)
                    .and_modify(|ids| ids.push(obj.obj))
                    .or_default();
            }
            ids_by_key
                .entry(obj.key)
                .and_modify(|id_map| {
                    id_map.insert(obj.offset, obj.obj);
                })
                .or_default();
            objs.entry(obj.obj).or_default().push(obj);
        }
        let mut bpm_changes = BTreeMap::new();
        for bpm_change in pack.bpm_changes {
            bpm_changes.insert(bpm_change.time, bpm_change);
        }
        let mut section_len_changes = BTreeMap::new();
        for section_len_change in pack.section_len_changes {
            section_len_changes.insert(section_len_change.track, section_len_change);
        }
        let mut stops = BTreeMap::new();
        for stop in pack.stops {
            stops.insert(stop.time, stop);
        }
        let mut bga_changes = BTreeMap::new();
        for bga_change in pack.bga_changes {
            bga_changes.insert(bga_change.time, bga_change);
        }
        let mut scrolling_factor_changes = BTreeMap::new();
        for scrolling_change in pack.scrolling_factor_changes {
            scrolling_factor_changes.insert(scrolling_change.time, scrolling_change);
        }
        let mut spacing_factor_changes = BTreeMap::new();
        for spacing_change in pack.spacing_factor_changes {
            spacing_factor_changes.insert(spacing_change.time, spacing_change);
        }
        let mut extended_messages = vec![];
        for extended_message in pack.extended_messages {
            extended_messages.push(extended_message);
        }
        Ok(Notes {
            objs,
            bgms,
            ids_by_key,
            bpm_changes,
            section_len_changes,
            stops,
            bga_changes,
            scrolling_factor_changes,
            spacing_factor_changes,
            extended_messages,
            exrank_defs: HashMap::new(),
            exwav_defs: HashMap::new(),
            change_options: HashMap::new(),
            texts: HashMap::new(),
            #[cfg(feature = "minor-command")]
            stp_events: Default::default(),
            #[cfg(feature = "minor-command")]
            wavcmd_events: Default::default(),
            #[cfg(feature = "minor-command")]
            cdda_events: Default::default(),
            #[cfg(feature = "minor-command")]
            swbga_events: Default::default(),
            #[cfg(feature = "minor-command")]
            argb_defs: Default::default(),
            #[cfg(feature = "minor-command")]
            seek_events: Default::default(),
        })
    }
}
