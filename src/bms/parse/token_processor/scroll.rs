//! This module handles the tokens:
//!
//! - `#SCROLL[01-ZZ] n` - Scrolling speed factor definition. It changes scrolling speed while keeps BPM.
//! - `#xxxSC:` - Scrolling speed factor channel.

use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{
    super::prompt::{DefDuplication, Prompter},
    TokenProcessor, TokenProcessorOutput, all_tokens_with_range, parse_obj_ids,
};
use crate::{
    bms::{model::scroll::ScrollObjects, parse::ParseWarning, prelude::*},
    util::StrExtension,
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
    ) -> TokenProcessorOutput<Self::Output> {
        let mut objects = ScrollObjects::default();
        let mut extra_warnings: Vec<ParseWarningWithRange> = Vec::new();
        let TokenProcessorOutput {
            output: res,
            mut warnings,
        } = all_tokens_with_range(input, |token| match token.content() {
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
                    |w| {
                        extra_warnings.extend(w);
                        Ok(None)
                    },
                ),
            Token::NotACommand(_) => Ok(None),
        });
        warnings.extend(extra_warnings);
        TokenProcessorOutput {
            output: res.map(|_| objects),
            warnings,
        }
    }
}

impl ScrollProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut ScrollObjects,
    ) -> core::result::Result<(), ParseWarning> {
        if let Some(id) = name.strip_prefix_ignore_case("SCROLL") {
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
    ) -> core::result::Result<Vec<ParseWarningWithRange>, ParseWarning> {
        let mut warnings: Vec<ParseWarningWithRange> = Vec::new();
        if channel == Channel::Scroll {
            let (pairs, w) = parse_obj_ids(track, message, &self.case_sensitive_obj_id);
            warnings.extend(w);
            for (time, obj) in pairs {
                let factor = objects
                    .scroll_defs
                    .get(&obj)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                objects
                    .push_scrolling_factor_change(ScrollingFactorObj { time, factor }, prompter)?;
            }
        }
        Ok(warnings)
    }
}
