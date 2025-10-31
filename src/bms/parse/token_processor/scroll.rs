//! This module handles the tokens:
//!
//! - `#SCROLL[01-ZZ] n` - Scrolling speed factor definition. It changes scrolling speed while keeps BPM.
//! - `#xxxSC:` - Scrolling speed factor channel.

use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{
    super::prompt::{DefDuplication, Prompter},
    TokenProcessor, all_tokens_with_range, parse_obj_ids_with_warnings,
};
use crate::{
    bms::{
        error::{ParseErrorWithRange, ParseWarning, Result},
        model::scroll::ScrollObjects,
        prelude::*,
    },
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
    ) -> (
        Self::Output,
        Vec<ParseWarningWithRange>,
        Vec<ParseErrorWithRange>,
    ) {
        let mut objects = ScrollObjects::default();
        let mut all_warnings = Vec::new();
        let (_, warnings, errors) = all_tokens_with_range(input, prompter, |token| {
            Ok(match token.content() {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), prompter, &mut objects)
                    .err(),
                Token::Message {
                    track,
                    channel,
                    message,
                } => {
                    let message_warnings = self.on_message(
                        *track,
                        *channel,
                        message.as_ref().into_wrapper(token),
                        prompter,
                        &mut objects,
                    );
                    all_warnings.extend(message_warnings);
                    None
                }
                Token::NotACommand(_) => None,
            })
        });
        all_warnings.extend(warnings);
        (objects, all_warnings, errors)
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
    ) -> Vec<ParseWarningWithRange> {
        let mut warnings = Vec::new();
        if channel == Channel::Scroll {
            let (obj_ids, parse_warnings) = parse_obj_ids_with_warnings(
                track,
                message.clone(),
                prompter,
                &self.case_sensitive_obj_id,
            );
            warnings.extend(parse_warnings);
            for (time, obj) in obj_ids {
                let factor = match objects.scroll_defs.get(&obj).cloned() {
                    Some(factor) => factor,
                    None => {
                        warnings.push(ParseWarning::UndefinedObject(obj).into_wrapper(&message));
                        continue;
                    }
                };
                if let Err(warning) = objects
                    .push_scrolling_factor_change(ScrollingFactorObj { time, factor }, prompter)
                {
                    warnings.push(warning.into_wrapper(&message));
                }
            }
        }
        warnings
    }
}
