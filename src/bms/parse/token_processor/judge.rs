//! This module handles the tokens:
//!
//! - `#RANK mode` - Judgement level option.
//! - `#EXRANK[01-ZZ] mode` - Judgement level change definition.
//! - `#xxxA0:` - Judgement level change channel.
//! - `#DEFEXRANK mode` - Custom judgement level option.
//! - `#TOTAL n` - Gauge increasing rate option. When the player played perfect, the gauge will increase the amount of `n`%.

use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{super::prompt::Prompter, Result, TokenProcessor, ids_from_message};
use crate::bms::{command::BaseType, model::Bms, prelude::*};

/// It processes `#RANK`` and `#EXRANKxx` definitions and objects on `Judge` channel.
pub struct JudgeProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for JudgeProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "RANK" => {
                self.0.borrow_mut().header.rank =
                    Some(JudgeLevel::try_from(args).map_err(|_| {
                        ParseWarning::SyntaxError(format!("expected integer but found: {args:?}"))
                    })?);
            }
            ex_rank if ex_rank.starts_with("EXRANK") => {
                let id = &name["EXRANK".len()..];
                let judge_level = JudgeLevel::try_from(args).map_err(|_| {
                    ParseWarning::SyntaxError(format!("expected integer but found: {args:?}"))
                })?;
                let mut id = ObjId::try_from(id)?;
                if !self.0.borrow().header.case_sensitive_obj_id {
                    id = id.fit_into_type(BaseType::Base36);
                }

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
            dex_ex_rank if dex_ex_rank.starts_with("DEFEXRANK") => {
                let value = args
                    .parse()
                    .map_err(|_| ParseWarning::SyntaxError("expected u64".into()))?;

                let judge_level = JudgeLevel::OtherInt(value);
                let id00 = ObjId::try_from("00")
                    .expect("00 must be valid ObjId")
                    .fit_into_type(BaseType::Base36);
                self.0.borrow_mut().scope_defines.exrank_defs.insert(
                    id00,
                    ExRankDef {
                        id: id00,
                        judge_level,
                    },
                );
            }
            "TOTAL" => {
                let total = Decimal::from_fraction(
                    GenericFraction::from_str(args)
                        .map_err(|_| ParseWarning::SyntaxError("expected decimal".into()))?,
                );
                self.0.borrow_mut().header.total = Some(total);
            }
            _ => {}
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if channel == Channel::Judge {
            let is_sensitive = self.0.borrow().header.case_sensitive_obj_id;
            for (time, judge_id) in
                ids_from_message(track, message, is_sensitive, |w| self.1.warn(w))
            {
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
        Ok(())
    }
}
