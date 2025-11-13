//! This module introduces struct [`JudgeObjects`], which manages internal setting values to score plays.

use std::collections::{BTreeMap, HashMap};

use crate::bms::{
    parse::{Result, prompt::ChannelDuplication},
    prelude::*,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// This aggregate manages internal setting values to score plays.
pub struct JudgeObjects {
    /// The judgement level of the score.
    pub rank: Option<JudgeLevel>,
    /// The total gauge percentage when all notes is got as PERFECT.
    pub total: Option<Decimal>,
    /// Storage for `#EXRANK` definitions
    pub exrank_defs: HashMap<ObjId, ExRankDef>,
    /// Judge events, indexed by time. `#xxxA0:`
    pub judge_events: BTreeMap<ObjTime, JudgeObj>,
}

impl JudgeObjects {
    /// Adds a new judge object to the notes.
    pub fn push_judge_event(
        &mut self,
        judge_obj: JudgeObj,
        prompter: &impl Prompter,
    ) -> Result<()> {
        match self.judge_events.entry(judge_obj.time) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(judge_obj);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get();

                prompter
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
}

/// A definition for `#EXRANK` command.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExRankDef {
    /// The object ID.
    pub id: ObjId,
    /// The judge level.
    pub judge_level: JudgeLevel,
}
