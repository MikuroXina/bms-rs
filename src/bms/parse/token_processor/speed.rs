//! This module handles the tokens:
//!
//! - `#SPEED[01-ZZ] n` - Spacing factor definition. It changes spacing among notes while keeps scrolling speed.
//! - `#xxxSP:` - Spacing factor channel.
use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{
    super::{
        Result,
        prompt::{DefDuplication, Prompter},
    },
    ParseWarning, TokenProcessor, TokenProcessorResult, all_tokens, ids_from_message,
};
use crate::bms::{model::Bms, prelude::*};

/// It processes `#SPEEDxx` definitions and objects on `Speed` channel.
pub struct SpeedProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for SpeedProcessor<'_, P> {
    fn process(&self, input: &mut &[&TokenWithRange<'_>]) -> TokenProcessorResult {
        all_tokens(input, |token| {
            Ok(match token {
                Token::Header { name, args } => self.on_header(name.as_ref(), args.as_ref()).err(),
                Token::Message {
                    track,
                    channel,
                    message,
                } => self.on_message(*track, *channel, message.as_ref()).err(),
                Token::NotACommand(_) => None,
            })
        })
    }
}

impl<P: Prompter> SpeedProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name.to_ascii_uppercase().starts_with("SPEED") {
            let id = &name["SPEED".len()..];
            let factor = Decimal::from_fraction(GenericFraction::from_str(args).map_err(|_| {
                ParseWarning::SyntaxError(format!("expected decimal but found: {args}"))
            })?);
            let speed_obj_id = ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id)?;

            if let Some(older) = self
                .0
                .borrow_mut()
                .scope_defines
                .speed_defs
                .get_mut(&speed_obj_id)
            {
                self.1
                    .handle_def_duplication(DefDuplication::SpeedFactorChange {
                        id: speed_obj_id,
                        older: older.clone(),
                        newer: factor.clone(),
                    })
                    .apply_def(older, factor, speed_obj_id)?;
            } else {
                self.0
                    .borrow_mut()
                    .scope_defines
                    .speed_defs
                    .insert(speed_obj_id, factor);
            }
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if channel == Channel::Speed {
            let is_sensitive = self.0.borrow().header.case_sensitive_obj_id;
            for (time, obj) in ids_from_message(track, message, is_sensitive, |w| self.1.warn(w)) {
                let factor = self
                    .0
                    .borrow()
                    .scope_defines
                    .speed_defs
                    .get(&obj)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                self.0
                    .borrow_mut()
                    .arrangers
                    .push_speed_factor_change(SpeedObj { time, factor }, self.1)?;
            }
        }
        Ok(())
    }
}
