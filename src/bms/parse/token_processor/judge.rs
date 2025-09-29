use std::{cell::RefCell, rc::Rc};

use super::{super::prompt::Prompter, Result, TokenProcessor, ids_from_message};
use crate::bms::{model::Bms, prelude::*};

/// It processes `#TEXTxx` definition and objects on `Text` channel.
pub struct JudgeProcessor<'a, P, T>(Rc<RefCell<Bms<T>>>, &'a P);

impl<P: Prompter, T: KeyLayoutMapper> TokenProcessor for JudgeProcessor<'_, P, T> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name == "RANK" {
            self.0.borrow_mut().header.rank = Some(JudgeLevel::try_from(args).map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but found: {args:?}"))
            })?);
        }
        if name.starts_with("EXRANK") {
            let id = name.trim_start_matches("EXRANK");
            let judge_level = JudgeLevel::try_from(args).map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but found: {args:?}"))
            })?;
            let id = ObjId::try_from(id).map_err(|id| {
                ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
            })?;

            let to_insert = ExRankDef { id, judge_level };
            if let Some(older) = self.0.borrow_mut().scope_defines.exrank_defs.get_mut(&id) {
                self.1
                    .handle_def_duplication(DefDuplication::ExRank {
                        id,
                        older,
                        newer: &to_insert,
                    })
                    .apply_def(older, to_insert, id)?;
            } else {
                self.0
                    .borrow_mut()
                    .scope_defines
                    .exrank_defs
                    .insert(id, to_insert);
            }
        }
        if name.starts_with("DEFEXRANK") {
            let value = args
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected u64".into()))?;

            let judge_level = JudgeLevel::OtherInt(value);
            self.0.borrow_mut().scope_defines.exrank_defs.insert(
                ObjId::try_from([b'0', b'0'])
                    .map_err(|_| ParseWarning::SyntaxError("Invalid ObjId [0, 0]".to_string()))?,
                ExRankDef {
                    id: ObjId::try_from([b'0', b'0']).map_err(|_| {
                        ParseWarning::SyntaxError("Invalid ObjId [0, 0]".to_string())
                    })?,
                    judge_level,
                },
            );
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        match channel {
            Channel::Judge => {
                for (time, judge_id) in ids_from_message(track, message, |w| self.1.warn(w)) {
                    let exrank_def = self
                        .0
                        .borrow()
                        .scope_defines
                        .exrank_defs
                        .get(&judge_id)
                        .cloned()
                        .ok_or(ParseWarning::UndefinedObject(judge_id))?;
                    self.0.borrow_mut().notes.push_judge_event(
                        JudgeObj {
                            time,
                            judge_level: exrank_def.judge_level,
                        },
                        self.1,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
