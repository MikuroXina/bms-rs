//! This module handles the tokens:
//!
//! - `#BPM n` - Initial BPM definition.
//! - `#BPM[01-ZZ] n` / `#EXBPM[01-ZZ] n` - BPM change definition.
//! - `#BASEBPM` - Reference speed for scroll speed. Obsolete.
//! - `#xxx08:` - BPM change channel.

use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::ParseWarningCollectior;
use super::{
    super::prompt::{DefDuplication, Prompter},
    ProcessContext, TokenProcessor, all_tokens, parse_hex_values, parse_obj_ids,
};
use crate::bms::ParseErrorWithRange;
use crate::{
    bms::{model::bpm::BpmObjects, parse::ParseWarning, prelude::*},
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

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> Result<Self::Output, ParseErrorWithRange> {
        let mut objects = BpmObjects::default();
        let mut buffered_warnings = Vec::new();
        let tokens_view = ctx.take_input();
        let mut syntactic_warnings: Vec<ParseWarningWithRange> = Vec::new();
        let prompter = ctx.prompter();
        all_tokens(tokens_view, &mut syntactic_warnings, |token| {
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
        {
            let mut wc = ctx.get_warning_collector();
            wc.collect(syntactic_warnings);
            wc.collect(buffered_warnings);
        }
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
    ) -> core::result::Result<(), ParseWarning> {
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
    ) -> core::result::Result<Vec<ParseWarningWithRange>, ParseWarning> {
        let mut warnings: Vec<ParseWarningWithRange> = Vec::new();
        if channel == Channel::BpmChange {
            let (pairs, w) = parse_obj_ids(track, message.clone(), &self.case_sensitive_obj_id);
            warnings.extend(w);
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
            let (pairs, w) = parse_hex_values(track, message);
            warnings.extend(w);
            for (time, value) in pairs {
                objects.push_bpm_change_u8(time, value, prompter)?;
            }
        }
        Ok(warnings)
    }
}
