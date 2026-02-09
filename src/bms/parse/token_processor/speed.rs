//! This module handles the tokens:
//!
//! - `#SPEED[01-ZZ] n` - Spacing factor definition. It changes spacing among notes while keeps scrolling speed.
//! - `#xxxSP:` - Spacing factor channel.

use std::{cell::RefCell, rc::Rc};

use super::{
    super::prompt::{DefDuplication, Prompter},
    ProcessContext, TokenProcessor, parse_obj_ids,
};
use crate::bms::ParseErrorWithRange;
use crate::{
    bms::{
        model::{StringValue, speed::SpeedObjects},
        parse::{ParseWarning, Result},
        prelude::*,
    },
    util::StrExtension,
};
use strict_num_extended::PositiveF64;

/// It processes `#SPEEDxx` definitions and objects on `Speed` channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeedProcessor {
    case_sensitive_obj_id: Rc<RefCell<bool>>,
}

impl SpeedProcessor {
    pub fn new(case_sensitive_obj_id: &Rc<RefCell<bool>>) -> Self {
        Self {
            case_sensitive_obj_id: Rc::clone(case_sensitive_obj_id),
        }
    }
}

impl TokenProcessor for SpeedProcessor {
    type Output = SpeedObjects;

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> core::result::Result<Self::Output, ParseErrorWithRange> {
        let mut objects = SpeedObjects::default();
        ctx.all_tokens(|token, prompter| match token.content() {
            Token::Header { name, args } => Ok(self
                .on_header(name.as_ref(), args.as_ref(), prompter, &mut objects)
                .err()
                .map(|warn| warn.into_wrapper(token))),
            Token::Message {
                track,
                channel,
                message,
            } => Ok(self
                .on_message(
                    *track,
                    *channel,
                    message.as_ref().into_wrapper(token),
                    prompter,
                    &mut objects,
                )
                .err()
                .map(|warn| warn.into_wrapper(token))),
            Token::NotACommand(_) => Ok(None),
        })?;
        Ok(objects)
    }
}

impl SpeedProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut SpeedObjects,
    ) -> Result<()> {
        if let Some(id) = name.strip_prefix_ignore_case("SPEED") {
            let string_value: StringValue<PositiveF64> = StringValue::new(args.to_string());
            let speed_obj_id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;

            if let Some(older) = objects.speed_defs.get_mut(&speed_obj_id) {
                prompter
                    .handle_def_duplication(DefDuplication::SpeedFactorChange {
                        id: speed_obj_id,
                        older: older.value(),
                        newer: string_value.value(),
                    })
                    .apply_def(older, string_value, speed_obj_id)?;
            } else {
                objects.speed_defs.insert(speed_obj_id, string_value);
            }
        }
        Ok(())
    }

    fn on_message(
        &self,
        track: Track,
        channel: Channel,
        message: SourceRangeMixin<&str>,
        prompter: &impl Prompter,
        objects: &mut SpeedObjects,
    ) -> Result<Vec<ParseWarningWithRange>> {
        let mut warnings: Vec<ParseWarningWithRange> = Vec::new();
        if channel == Channel::Speed {
            let (pairs, w) = parse_obj_ids(track, &message, &self.case_sensitive_obj_id);
            warnings.extend(w);
            for (time, obj) in pairs {
                let string_value = objects
                    .speed_defs
                    .get(&obj)
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                // Get factor, skip this object if parsing failed
                let Ok(factor) = string_value.value() else {
                    continue;
                };
                objects.push_speed_factor_change(
                    SpeedObj {
                        time,
                        factor: *factor,
                    },
                    prompter,
                )?;
            }
        }
        Ok(warnings)
    }
}
