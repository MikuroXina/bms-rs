//! This module handles the tokens:
//!
//! - `#RANK mode` - Judgement level option.
//! - `#EXRANK[01-ZZ] mode` - Judgement level change definition.
//! - `#xxxA0:` - Judgement level change channel.
//! - `#DEFEXRANK mode` - Custom judgement level option.
//! - `#TOTAL n` - Gauge increasing rate option. When the player played perfect, the gauge will increase the amount of `n`%.

use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{
    super::prompt::Prompter, TokenProcessor, TokenProcessorOutput, all_tokens_with_range,
    parse_obj_ids,
};
use crate::{
    bms::{error::Result, model::judge::JudgeObjects, prelude::*},
    util::StrExtension,
};

/// It processes `#RANK`` and `#EXRANKxx` definitions and objects on `Judge` channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeProcessor {
    case_sensitive_obj_id: Rc<RefCell<bool>>,
}

impl JudgeProcessor {
    pub fn new(case_sensitive_obj_id: &Rc<RefCell<bool>>) -> Self {
        Self {
            case_sensitive_obj_id: Rc::clone(case_sensitive_obj_id),
        }
    }
}

impl TokenProcessor for JudgeProcessor {
    type Output = JudgeObjects;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorOutput<Self::Output> {
        let mut objects = JudgeObjects::default();
        let mut extra_warnings: Vec<ParseWarningWithRange> = Vec::new();
        let (res, mut warnings) = all_tokens_with_range(input, |token| match token.content() {
            Token::Header { name, args } => Ok(self
                .on_header(name.as_ref(), args.as_ref(), prompter, &mut objects)
                .err()),
            Token::Message {
                track,
                channel,
                message,
            } => match self.on_message(
                *track,
                *channel,
                message.as_ref().into_wrapper(token),
                prompter,
                &mut objects,
            ) {
                Ok(w) => {
                    extra_warnings.extend(w);
                    Ok(None)
                }
                Err(warn) => Ok(Some(warn)),
            },
            Token::NotACommand(_) => Ok(None),
        });
        warnings.extend(extra_warnings);
        match res {
            Ok(()) => (Ok(objects), warnings),
            Err(e) => (Err(e), warnings),
        }
    }
}

impl JudgeProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut JudgeObjects,
    ) -> Result<()> {
        if name.eq_ignore_ascii_case("RANK") {
            objects.rank = Some(JudgeLevel::try_from(args).map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but found: {args:?}"))
            })?);
        }
        if let Some(id) = name.strip_prefix_ignore_case("EXRANK") {
            let judge_level = JudgeLevel::try_from(args).map_err(|_| {
                ParseWarning::SyntaxError(format!("expected integer but found: {args:?}"))
            })?;
            let id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;

            let to_insert = ExRankDef { id, judge_level };
            if let Some(older) = objects.exrank_defs.get_mut(&id) {
                prompter
                    .handle_def_duplication(DefDuplication::ExRank {
                        id,
                        older,
                        newer: &to_insert,
                    })
                    .apply_def(older, to_insert, id)?;
            } else {
                objects.exrank_defs.insert(id, to_insert);
            }
        }
        if name.eq_ignore_ascii_case("DEFEXRANK") {
            let value = args
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected u64".into()))?;

            let judge_level = JudgeLevel::OtherInt(value);
            objects.exrank_defs.insert(
                ObjId::try_from("00", false).expect("00 must be valid ObjId"),
                ExRankDef {
                    id: ObjId::try_from("00", false).expect("00 must be valid ObjId"),
                    judge_level,
                },
            );
        }
        if name.eq_ignore_ascii_case("TOTAL") {
            let total = Decimal::from_fraction(
                GenericFraction::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected decimal".into()))?,
            );
            objects.total = Some(total);
        }
        Ok(())
    }

    fn on_message(
        &self,
        track: Track,
        channel: Channel,
        message: SourceRangeMixin<&str>,
        prompter: &impl Prompter,
        objects: &mut JudgeObjects,
    ) -> Result<Vec<ParseWarningWithRange>> {
        let mut warnings: Vec<ParseWarningWithRange> = Vec::new();
        if channel == Channel::Judge {
            let (pairs, mut w) = parse_obj_ids(track, message, &self.case_sensitive_obj_id);
            warnings.append(&mut w);
            for (time, judge_id) in pairs {
                let exrank_def = objects
                    .exrank_defs
                    .get(&judge_id)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(judge_id))?;
                objects.push_judge_event(
                    JudgeObj {
                        time,
                        judge_level: exrank_def.judge_level,
                    },
                    prompter,
                )?;
            }
        }
        Ok(warnings)
    }
}
