//! This module handles the tokens:
//!
//! - `#SPEED[01-ZZ] n` - Spacing factor definition. It changes spacing among notes while keeps scrolling speed.
//! - `#xxxSP:` - Spacing factor channel.

use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{
    super::prompt::{DefDuplication, Prompter},
    TokenProcessor, TokenProcessorResult, all_tokens_with_range, parse_obj_ids,
};
use crate::{
    bms::{
        error::{ParseWarning, Result},
        model::speed::SpeedObjects,
        prelude::*,
    },
    util::StrExtension,
};

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

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorResult<Self::Output> {
        let mut objects = SpeedObjects::default();
        all_tokens_with_range(input, prompter, |token| {
            Ok(match token.content() {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), prompter, &mut objects)
                    .err(),
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
                    .err(),
                Token::NotACommand(_) => None,
            })
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
            let factor = Decimal::from_fraction(GenericFraction::from_str(args).map_err(|_| {
                ParseWarning::SyntaxError(format!("expected decimal but found: {args}"))
            })?);
            let speed_obj_id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;

            if let Some(older) = objects.speed_defs.get_mut(&speed_obj_id) {
                prompter
                    .handle_def_duplication(DefDuplication::SpeedFactorChange {
                        id: speed_obj_id,
                        older: older.clone(),
                        newer: factor.clone(),
                    })
                    .apply_def(older, factor, speed_obj_id)?;
            } else {
                objects.speed_defs.insert(speed_obj_id, factor);
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
    ) -> Result<()> {
        if channel == Channel::Speed {
            for (time, obj) in parse_obj_ids(track, message, prompter, &self.case_sensitive_obj_id)
            {
                let factor = objects
                    .speed_defs
                    .get(&obj)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                objects.push_speed_factor_change(SpeedObj { time, factor }, prompter)?;
            }
        }
        Ok(())
    }
}
