//! This module handles the tokens:
//!
//! - `#SCROLL[01-ZZ] n` - Scrolling speed factor definition. It changes scrolling speed while keeps BPM.
//! - `#xxxSC:` - Scrolling speed factor channel.

use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{
    super::prompt::{DefDuplication, Prompter},
    TokenProcessor, TokenProcessorResult, all_tokens_with_range, parse_obj_ids,
};
use crate::bms::{
    error::{ParseWarning, Result},
    model::scroll::ScrollObjects,
    prelude::*,
};

/// It processes `#SCROLLxx` definitions and objects on `Scroll` channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollProcessor {
    case_sensitive_obj_id: Rc<RefCell<bool>>,
}

impl ScrollProcessor {
    pub fn new(case_sensitive_obj_id: &Rc<RefCell<bool>>) -> Self {
        Self {
            case_sensitive_obj_id: Rc::clone(case_sensitive_obj_id),
        }
    }
}

impl TokenProcessor for ScrollProcessor {
    type Output = ScrollObjects;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorResult<Self::Output> {
        let mut objects = ScrollObjects::default();
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

impl ScrollProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut ScrollObjects,
    ) -> Result<()> {
        if name.to_ascii_uppercase().starts_with("SCROLL") {
            let id = &name["SCROLL".len()..];
            let factor =
                Decimal::from_fraction(GenericFraction::from_str(args).map_err(|_| {
                    ParseWarning::SyntaxError("expected decimal scroll factor".into())
                })?);
            let scroll_obj_id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;
            if let Some(older) = objects.scroll_defs.get_mut(&scroll_obj_id) {
                prompter
                    .handle_def_duplication(DefDuplication::ScrollingFactorChange {
                        id: scroll_obj_id,
                        older: older.clone(),
                        newer: factor.clone(),
                    })
                    .apply_def(older, factor, scroll_obj_id)?;
            } else {
                objects.scroll_defs.insert(scroll_obj_id, factor);
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
        objects: &mut ScrollObjects,
    ) -> Result<()> {
        if channel == Channel::Scroll {
            for (time, obj) in parse_obj_ids(track, message, prompter, &self.case_sensitive_obj_id)
            {
                let factor = objects
                    .scroll_defs
                    .get(&obj)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                objects
                    .push_scrolling_factor_change(ScrollingFactorObj { time, factor }, prompter)?;
            }
        }
        Ok(())
    }
}
