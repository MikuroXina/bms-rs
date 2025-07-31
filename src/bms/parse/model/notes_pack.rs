//! NotesPack def.
#![allow(unused)]

use std::collections::{BTreeMap, HashMap};

use crate::bms::command::*;

use super::Notes;
use super::obj::*;

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
