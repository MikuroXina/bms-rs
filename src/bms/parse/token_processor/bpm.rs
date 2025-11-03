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
    TokenProcessor, TokenProcessorOutput, all_tokens_with_range, parse_hex_values, parse_obj_ids,
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
    ) -> TokenProcessorOutput<Self::Output> {
        let mut objects = BpmObjects::default();
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
            Ok(()) => TokenProcessorOutput {
                output: Ok(objects),
                warnings,
            },
            Err(e) => TokenProcessorOutput {
                output: Err(e),
                warnings,
            },
        }
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
    ) -> Result<Vec<ParseWarningWithRange>> {
        let mut warnings: Vec<ParseWarningWithRange> = Vec::new();
        if channel == Channel::BpmChange {
            let (pairs, mut w) = parse_obj_ids(track, message.clone(), &self.case_sensitive_obj_id);
            warnings.append(&mut w);
            for (time, obj) in pairs {
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
            let (pairs, mut w) = parse_hex_values(track, message);
            warnings.append(&mut w);
            for (time, value) in pairs {
                objects.push_bpm_change_u8(time, value, prompter)?;
            }
        }
        Ok(warnings)
    }
}
