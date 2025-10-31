//! This module handles the tokens:
//!
//! - `#BPM n` - Initial BPM definition.
//! - `#BPM[01-ZZ] n` / `#EXBPM[01-ZZ] n` - BPM change definition.
//! - `#BASEBPM` - Reference speed for scroll speed. Obsolete.
//! - `#xxx08:` - BPM change channel.

use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{
    super::prompt::{DefDuplication, Prompter},
    TokenProcessor, all_tokens_with_range, parse_hex_values_with_warnings,
    parse_obj_ids_with_warnings,
};
use crate::{
    bms::{
        error::{ParseWarning, Result},
        model::bpm::BpmObjects,
        prelude::*,
    },
    util::StrExtension,
};

/// It processes `#BPM` and `#BPMxx` definitions and objects on `BpmChange` and `BpmChangeU8` channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BpmProcessor {
    case_sensitive_obj_id: Rc<RefCell<bool>>,
}

impl BpmProcessor {
    pub fn new(case_sensitive_obj_id: &Rc<RefCell<bool>>) -> Self {
        Self {
            case_sensitive_obj_id: Rc::clone(case_sensitive_obj_id),
        }
    }
}

impl TokenProcessor for BpmProcessor {
    type Output = BpmObjects;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> (Self::Output, Vec<ParseWarningWithRange>) {
        let mut objects = BpmObjects::default();
        let mut all_warnings = Vec::new();
        let (_, warnings) = all_tokens_with_range(input, prompter, |token| {
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
        (objects, all_warnings)
    }
}

impl BpmProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut BpmObjects,
    ) -> Result<()> {
        if name.eq_ignore_ascii_case("BPM") {
            let bpm = Decimal::from_fraction(
                GenericFraction::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected decimal BPM".into()))?,
            );
            objects.bpm = Some(bpm);
        }
        if let Some(id) = name
            .strip_prefix_ignore_case("BPM")
            .or_else(|| name.strip_prefix_ignore_case("EXBPM"))
        {
            let bpm_obj_id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;
            let bpm = Decimal::from_fraction(
                GenericFraction::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected decimal BPM".into()))?,
            );
            if let Some(older) = objects.bpm_defs.get_mut(&bpm_obj_id) {
                prompter
                    .handle_def_duplication(DefDuplication::BpmChange {
                        id: bpm_obj_id,
                        older: older.clone(),
                        newer: bpm.clone(),
                    })
                    .apply_def(older, bpm, bpm_obj_id)?;
            } else {
                objects.bpm_defs.insert(bpm_obj_id, bpm);
            }
        }
        if name.eq_ignore_ascii_case("BASEBPM") {
            let bpm = Decimal::from_fraction(
                GenericFraction::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected decimal BPM".into()))?,
            );
            objects.base_bpm = Some(bpm);
        }
        Ok(())
    }

    fn on_message(
        &self,
        track: Track,
        channel: Channel,
        message: SourceRangeMixin<&str>,
        prompter: &impl Prompter,
        objects: &mut BpmObjects,
    ) -> Vec<ParseWarningWithRange> {
        let mut warnings = Vec::new();
        if channel == Channel::BpmChange {
            let (obj_ids, parse_warnings) = parse_obj_ids_with_warnings(
                track,
                message.clone(),
                prompter,
                &self.case_sensitive_obj_id,
            );
            warnings.extend(parse_warnings);
            for (time, obj) in obj_ids {
                // Record used BPM change id for validity checks
                objects.bpm_change_ids_used.insert(obj);
                let bpm = objects.bpm_defs.get(&obj).cloned();
                match bpm {
                    Some(bpm) => {
                        if let Err(warning) =
                            objects.push_bpm_change(BpmChangeObj { time, bpm }, prompter)
                        {
                            warnings.push(warning.into_wrapper(&message));
                        }
                    }
                    None => {
                        warnings.push(ParseWarning::UndefinedObject(obj).into_wrapper(&message));
                    }
                }
            }
        }
        if channel == Channel::BpmChangeU8 {
            let (hex_values, parse_warnings) =
                parse_hex_values_with_warnings(track, message.clone(), prompter);
            warnings.extend(parse_warnings);
            for (time, value) in hex_values {
                if let Err(warning) = objects.push_bpm_change_u8(time, value, prompter) {
                    warnings.push(warning.into_wrapper(&message));
                }
            }
        }
        warnings
    }
}
