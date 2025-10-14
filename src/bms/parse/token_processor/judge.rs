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
use crate::bms::{model::Bms, prelude::*};
use std::ops::ControlFlow;

/// It processes `#RANK`` and `#EXRANKxx` definitions and objects on `Judge` channel.
pub struct JudgeProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for JudgeProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> ControlFlow<Result<()>> {
        match name.to_ascii_uppercase().as_str() {
            "RANK" => {
                let rank = match JudgeLevel::try_from(args) {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(format!(
                            "expected integer but found: {args:?}"
                        ))));
                    }
                };
                self.0.borrow_mut().header.rank = Some(rank);
                return ControlFlow::Break(Ok(()));
            }
            ex_rank if ex_rank.starts_with("EXRANK") => {
                let id = &name["EXRANK".len()..];
                let judge_level = match JudgeLevel::try_from(args) {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(format!(
                            "expected integer but found: {args:?}"
                        ))));
                    }
                };
                let id = match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                    Ok(v) => v,
                    Err(e) => return ControlFlow::Break(Err(e)),
                };

                let to_insert = ExRankDef { id, judge_level };
                if let Some(older) = self.0.borrow_mut().scope_defines.exrank_defs.get_mut(&id) {
                    if let Err(e) = self
                        .1
                        .handle_def_duplication(DefDuplication::ExRank {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, id)
                    {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    self.0
                        .borrow_mut()
                        .scope_defines
                        .exrank_defs
                        .insert(id, to_insert);
                }
                return ControlFlow::Break(Ok(()));
            }
            dex_ex_rank if dex_ex_rank.starts_with("DEFEXRANK") => {
                let value = match args.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected u64".into(),
                        )));
                    }
                };

                let judge_level = JudgeLevel::OtherInt(value);
                self.0.borrow_mut().scope_defines.exrank_defs.insert(
                    ObjId::try_from("00", false).expect("00 must be valid ObjId"),
                    ExRankDef {
                        id: ObjId::try_from("00", false).expect("00 must be valid ObjId"),
                        judge_level,
                    },
                );
                return ControlFlow::Break(Ok(()));
            }
            "TOTAL" => {
                let frac = match GenericFraction::from_str(args) {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected decimal".into(),
                        )));
                    }
                };
                let total = Decimal::from_fraction(frac);
                self.0.borrow_mut().header.total = Some(total);
                return ControlFlow::Break(Ok(()));
            }
            _ => {
                return ControlFlow::Continue(());
            }
        }
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> ControlFlow<Result<()>> {
        if channel == Channel::Judge {
            let is_sensitive = self.0.borrow().header.case_sensitive_obj_id;
            for (time, judge_id) in
                ids_from_message(track, message, is_sensitive, |w| self.1.warn(w))
            {
                let exrank_def = match self
                    .0
                    .borrow()
                    .scope_defines
                    .exrank_defs
                    .get(&judge_id)
                    .cloned()
                {
                    Some(v) => v,
                    None => {
                        return ControlFlow::Break(Err(ParseWarning::UndefinedObject(judge_id)));
                    }
                };
                if let Err(e) = self.0.borrow_mut().notes.push_judge_event(
                    JudgeObj {
                        time,
                        judge_level: exrank_def.judge_level,
                    },
                    self.1,
                ) {
                    return ControlFlow::Break(Err(e));
                }
            }
            return ControlFlow::Break(Ok(()));
        }
        ControlFlow::Continue(())
    }
}
