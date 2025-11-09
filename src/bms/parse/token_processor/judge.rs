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
    super::prompt::Prompter, ProcessContext, TokenProcessor, all_tokens_with_range, parse_obj_ids,
};
use crate::bms::ParseErrorWithRange;
use crate::{
    bms::{model::judge::JudgeObjects, prelude::*},
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

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> Result<Self::Output, ParseErrorWithRange> {
        let mut objects = JudgeObjects::default();
        let prompter = ctx.prompter();
        let mut buffered_warnings = Vec::new();
        let tokens_view = *ctx.input;
        let mut iter_warnings = Vec::new();
        all_tokens_with_range(tokens_view, &mut iter_warnings, |token| {
            match token.content() {
                Token::Header { name, args } => Ok(self
                    .on_header(name.as_ref(), args.as_ref(), prompter, &mut objects)
                    .err()),
                Token::Message {
                    track,
                    channel,
                    message,
                } => self
                    .on_message(
                        *track,
                        *channel,
                        message.as_ref().into_wrapper(token),
                        prompter,
                        &mut objects,
                    )
                    .map_or_else(
                        |warn| Ok(Some(warn)),
                        |ws| {
                            buffered_warnings.extend(ws);
                            Ok(None)
                        },
                    ),
                Token::NotACommand(_) => Ok(None),
            }
        })?;
        *ctx.input = &[];
        ctx.reported.extend(buffered_warnings);
        ctx.reported.extend(iter_warnings);
        Ok(objects)
    }
}

impl JudgeProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut JudgeObjects,
    ) -> core::result::Result<(), ParseWarning> {
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
    ) -> core::result::Result<Vec<ParseWarningWithRange>, ParseWarning> {
        let mut warnings: Vec<ParseWarningWithRange> = Vec::new();
        if channel == Channel::Judge {
            let (pairs, w) = parse_obj_ids(track, message, &self.case_sensitive_obj_id);
            warnings.extend(w);
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
