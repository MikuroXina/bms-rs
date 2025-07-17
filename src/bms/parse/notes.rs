//! Note objects manager.

use itertools::Itertools;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
    ops::Bound,
};

use super::{ParseError, Result, header::Header, obj::Obj};
use crate::bms::{
    lex::{
        command::{self, Channel, Key, NoteKind, ObjId},
        token::Token,
    },
    parse::header::{ExRankDef, ExWavDef},
    time::{ObjTime, Track},
};

/// An object to change the BPM of the score.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BpmChangeObj {
    /// The time to begin the change of BPM.
    pub time: ObjTime,
    /// The BPM to be.
    pub bpm: f64,
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
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionLenChangeObj {
    /// The target track to change.
    pub track: Track,
    /// The length to be.
    pub length: f64,
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
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StopObj {
    /// Time to start the stop.
    pub time: ObjTime,
    /// Object duration how long stops scrolling of score.
    ///
    /// Note that the duration of stopping will not be changed by a current measure length but BPM.
    pub duration: u32,
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
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScrollingFactorObj {
    /// The time to begin the change of BPM.
    pub time: ObjTime,
    /// The scrolling factor to be.
    pub factor: f64,
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
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpacingFactorObj {
    /// The time to begin the change of BPM.
    pub time: ObjTime,
    /// The spacing factor to be.
    pub factor: f64,
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
#[derive(Debug, Default)]
pub struct Notes {
    // objects stored in obj is sorted, so it can be searched by bisection method
    objs: HashMap<ObjId, Vec<Obj>>,
    bgms: BTreeMap<ObjTime, Vec<ObjId>>,
    ids_by_key: HashMap<Key, BTreeMap<ObjTime, ObjId>>,
    bpm_changes: BTreeMap<ObjTime, BpmChangeObj>,
    section_len_changes: BTreeMap<Track, SectionLenChangeObj>,
    stops: BTreeMap<ObjTime, StopObj>,
    bga_changes: BTreeMap<ObjTime, BgaObj>,
    scrolling_factor_changes: BTreeMap<ObjTime, ScrollingFactorObj>,
    spacing_factor_changes: BTreeMap<ObjTime, SpacingFactorObj>,
    extended_messages: Vec<ExtendedMessageObj>,
    /// Storage for #EXRANK definitions
    pub exrank_defs: HashMap<ObjId, ExRankDef>,
    /// Storage for #EXWAV definitions
    pub exwav_defs: HashMap<ObjId, ExWavDef>,
    /// Storage for #CHANGEOPTION definitions
    pub change_options: HashMap<ObjId, String>,
    /// Storage for #TEXT definitions
    pub texts: HashMap<ObjId, String>,
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
        if self
            .bpm_changes
            .insert(bpm_change.time, bpm_change)
            .is_some()
        {
            eprintln!(
                "duplicate bpm change object detected at {:?}",
                bpm_change.time
            );
        }
    }

    /// Adds a new scrolling factor change object to the notes.
    pub fn push_scrolling_factor_change(&mut self, bpm_change: ScrollingFactorObj) {
        if self
            .scrolling_factor_changes
            .insert(bpm_change.time, bpm_change)
            .is_some()
        {
            eprintln!(
                "duplicate scrolling factor change object detected at {:?}",
                bpm_change.time
            );
        }
    }

    /// Adds a new spacing factor change object to the notes.
    pub fn push_spacing_factor_change(&mut self, bpm_change: SpacingFactorObj) {
        if self
            .spacing_factor_changes
            .insert(bpm_change.time, bpm_change)
            .is_some()
        {
            eprintln!(
                "duplicate spacing factor change object detected at {:?}",
                bpm_change.time
            );
        }
    }

    /// Adds a new section length change object to the notes.
    pub fn push_section_len_change(&mut self, section_len_change: SectionLenChangeObj) {
        if self
            .section_len_changes
            .insert(section_len_change.track, section_len_change)
            .is_some()
        {
            eprintln!(
                "duplicate section length change object detected at {:?}",
                section_len_change.track
            );
        }
    }

    /// Adds a new stop object to the notes.
    pub fn push_stop(&mut self, stop: StopObj) {
        self.stops
            .entry(stop.time)
            .and_modify(|existing| {
                existing.duration = existing.duration.saturating_add(stop.duration)
            })
            .or_insert(stop);
    }

    /// Adds a new bga change object to the notes.
    pub fn push_bga_change(&mut self, bga: BgaObj) {
        if self.bga_changes.insert(bga.time, bga).is_some() {
            eprintln!("duplicate bga change object detected at {:?}", bga.time);
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
                    let &bpm = header
                        .bpm_changes
                        .get(&obj)
                        .ok_or(ParseError::UndefinedObject(obj))?;
                    self.push_bpm_change(BpmChangeObj { time, bpm });
                }
            }
            Token::Message {
                track,
                channel: Channel::BpmChangeU8,
                message,
            } => {
                let denominator = message.len() as u32 / 2;
                for (i, (c1, c2)) in message.chars().tuples().enumerate() {
                    let bpm = c1.to_digit(16).unwrap() * 16 + c2.to_digit(16).unwrap();
                    if bpm == 0 {
                        continue;
                    }
                    let time = ObjTime::new(track.0, i as u32, denominator);
                    self.push_bpm_change(BpmChangeObj {
                        time,
                        bpm: bpm as f64,
                    });
                }
            }
            Token::Message {
                track,
                channel: Channel::Scroll,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let &factor = header
                        .scrolling_factor_changes
                        .get(&obj)
                        .ok_or(ParseError::UndefinedObject(obj))?;
                    self.push_scrolling_factor_change(ScrollingFactorObj { time, factor });
                }
            }
            Token::Message {
                track,
                channel: Channel::Speed,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let &factor = header
                        .spacing_factor_changes
                        .get(&obj)
                        .ok_or(ParseError::UndefinedObject(obj))?;
                    self.push_spacing_factor_change(SpacingFactorObj { time, factor });
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
                        .ok_or(ParseError::UndefinedObject(obj))?;
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
                let length = message.parse().expect("f64 as section length");
                assert!(0.0 < length, "section length must be greater than zero");
                self.push_section_len_change(SectionLenChangeObj { track, length });
            }
            Token::Message {
                track,
                channel: Channel::Stop,
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    let &duration = header
                        .stops
                        .get(&obj)
                        .ok_or(ParseError::UndefinedObject(obj))?;
                    self.push_stop(StopObj { time, duration })
                }
            }
            Token::Message {
                track,
                channel: channel @ (Channel::BgaBase | Channel::BgaPoor | Channel::BgaLayer),
                message,
            } => {
                for (time, obj) in ids_from_message(*track, message) {
                    if !header.bmp_files.contains_key(&obj) {
                        return Err(ParseError::UndefinedObject(obj));
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
                channel:
                    Channel::Note {
                        kind,
                        is_player1,
                        key,
                    },
                message,
            } => {
                for (offset, obj) in ids_from_message(*track, message) {
                    self.push_note(Obj {
                        offset,
                        kind: *kind,
                        is_player1: *is_player1,
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
                    .ok_or(ParseError::UndefinedObject(end_id))?;
                let Obj { offset, key, .. } = &end_note;
                let (_, &begin_id) =
                    self.ids_by_key[key].range(..offset).last().ok_or_else(|| {
                        ParseError::SyntaxError(format!(
                            "expected preceding object for #LNOBJ {:?}",
                            end_id
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
            Token::Email(_)
            | Token::Url(_)
            | Token::OctFp
            | Token::Option(_)
            | Token::PathWav(_)
            | Token::Maker(_)
            | Token::MidiFile(_)
            | Token::PoorBga(_)
            | Token::VideoFile(_)
            | Token::Artist(_)
            | Token::AtBga { .. }
            | Token::Banner(_)
            | Token::BackBmp(_)
            | Token::Base62
            | Token::Bga { .. }
            | Token::Bmp(_, _)
            | Token::Bpm(_)
            | Token::BpmChange(_, _)
            | Token::Case(_)
            | Token::Comment(_)
            | Token::Def
            | Token::Difficulty(_)
            | Token::Else
            | Token::ElseIf(_)
            | Token::EndIf
            | Token::EndRandom
            | Token::EndSwitch
            | Token::ExBmp(_, _, _)
            | Token::Genre(_)
            | Token::If(_)
            | Token::LnTypeRdm
            | Token::LnTypeMgq
            | Token::NotACommand(_)
            | Token::Player(_)
            | Token::PlayLevel(_)
            | Token::Random(_)
            | Token::Rank(_)
            | Token::Scroll(_, _)
            | Token::SetRandom(_)
            | Token::SetSwitch(_)
            | Token::Skip
            | Token::Speed(_, _)
            | Token::StageFile(_)
            | Token::Stop(_, _)
            | Token::SubArtist(_)
            | Token::SubTitle(_)
            | Token::Switch(_)
            | Token::Title(_)
            | Token::Total(_)
            | Token::VolWav(_)
            | Token::Wav(_, _) => {
                // These tokens don't need to be processed in Notes::parse, they should be handled in Header::parse
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
    pub fn resolution_for_pulses(&self) -> u32 {
        use num::Integer;

        let mut hyp_resolution = 1;
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
    let denominator = message.len() as u32 / 2;
    let mut chars = message.chars().tuples().enumerate();
    std::iter::from_fn(move || {
        let (i, c1, c2) = loop {
            let (i, (c1, c2)) = chars.next()?;
            if !(c1 == '0' && c2 == '0') {
                break (i, c1, c2);
            }
        };
        let obj = ObjId::from_chars([c1, c2]).expect("invalid object id");
        let time = ObjTime::new(track.0, i as u32, denominator);
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
        })
    }
}
