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
    TokenProcessor, TokenProcessorResult, all_tokens_with_range, parse_hex_values, parse_obj_ids,
};
use crate::bms::{
    error::{ParseWarning, Result},
    model::bpm::BpmObjects,
    prelude::*,
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
    ) -> TokenProcessorResult<Self::Output> {
        let mut objects = BpmObjects::default();
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

impl BpmProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut BpmObjects,
    ) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "BPM" => {
                let bpm = Decimal::from_fraction(
                    GenericFraction::from_str(args)
                        .map_err(|_| ParseWarning::SyntaxError("expected decimal BPM".into()))?,
                );
                objects.bpm = Some(bpm);
            }
            bpm if bpm.starts_with("BPM") || bpm.starts_with("EXBPM") => {
                let id = if bpm.starts_with("BPM") {
                    &name["BPM".len()..]
                } else {
                    &name["EXBPM".len()..]
                };
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

            "BASEBPM" => {
                let bpm = Decimal::from_fraction(
                    GenericFraction::from_str(args)
                        .map_err(|_| ParseWarning::SyntaxError("expected decimal BPM".into()))?,
                );
                objects.base_bpm = Some(bpm);
            }
            _ => {}
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
    ) -> Result<()> {
        if channel == Channel::BpmChange {
            for (time, obj) in parse_obj_ids(
                track,
                message.clone(),
                prompter,
                &self.case_sensitive_obj_id,
            ) {
                // Record used BPM change id for validity checks
                objects.bpm_change_ids_used.insert(obj);
                let bpm = objects
                    .bpm_defs
                    .get(&obj)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                objects.push_bpm_change(BpmChangeObj { time, bpm }, prompter)?;
            }
        }
        if channel == Channel::BpmChangeU8 {
            for (time, value) in parse_hex_values(track, message, prompter) {
                objects.push_bpm_change_u8(time, value, prompter)?;
            }
        }
        Ok(())
    }
}
