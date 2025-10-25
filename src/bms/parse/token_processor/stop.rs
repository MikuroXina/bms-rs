//! This module handles the tokens:
//!
//! - `#STOP[01-ZZ] n` - Stop definition. It stops the scroll as `n` of 192nd note.
//! - `#xxx09:` - Stop channel.
//! - `#STP xxx.yyy time` - It stops `time` milliseconds at section `xxx` and its position (`yyy` / 1000).

use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{
    super::prompt::{DefDuplication, Prompter},
    TokenProcessor, TokenProcessorResult, all_tokens_with_range, parse_obj_ids,
};
use crate::bms::{
    error::{ParseWarning, Result},
    model::stop::StopObjects,
    prelude::*,
};

/// It processes `#STOPxx` definitions and objects on `Stop` channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopProcessor {
    case_sensitive_obj_id: Rc<RefCell<bool>>,
}

impl StopProcessor {
    pub fn new(case_sensitive_obj_id: &Rc<RefCell<bool>>) -> Self {
        Self {
            case_sensitive_obj_id: Rc::clone(case_sensitive_obj_id),
        }
    }
}

impl TokenProcessor for StopProcessor {
    type Output = StopObjects;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorResult<Self::Output> {
        let mut objects = StopObjects::default();
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

impl StopProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut StopObjects,
    ) -> Result<()> {
        if name.to_ascii_uppercase().starts_with("STOP") {
            let id = &name["STOP".len()..];
            let len =
                Decimal::from_fraction(GenericFraction::from_str(args).map_err(|_| {
                    ParseWarning::SyntaxError("expected decimal stop length".into())
                })?);

            let stop_obj_id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;

            if let Some(older) = objects.stop_defs.get_mut(&stop_obj_id) {
                prompter
                    .handle_def_duplication(DefDuplication::Stop {
                        id: stop_obj_id,
                        older: older.clone(),
                        newer: len.clone(),
                    })
                    .apply_def(older, len, stop_obj_id)?;
            } else {
                objects.stop_defs.insert(stop_obj_id, len);
            }
        }

        if name.to_ascii_uppercase().starts_with("STP") {
            // Parse xxx.yyy zzzz
            use std::{num::NonZeroU64, time::Duration};
            let args: Vec<_> = args.split_whitespace().collect();
            if args.len() != 3 {
                return Err(ParseWarning::SyntaxError(
                    "stp measure/pos must be 3 digits".into(),
                ));
            }

            let (measure, pos) = args[0].split_once('.').unwrap_or((args[0], "000"));
            let measure: u16 = measure
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected measure u16".into()))?;
            let pos: u16 = pos
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected pos u16".into()))?;
            let ms: u64 = args[2]
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected pos u64".into()))?;
            let time = ObjTime::new(
                measure as u64,
                pos as u64,
                NonZeroU64::new(1000).expect("1000 should be a valid NonZeroU64"),
            );
            let duration = Duration::from_millis(ms);

            // Store by ObjTime as key, handle duplication with prompt handler
            let ev = StpEvent { time, duration };
            if let Some(older) = objects.stp_events.get_mut(&time) {
                use crate::parse::prompt::ChannelDuplication;

                prompter
                    .handle_channel_duplication(ChannelDuplication::StpEvent {
                        time,
                        older,
                        newer: &ev,
                    })
                    .apply_channel(older, ev, time, Channel::Stop)?;
            } else {
                objects.stp_events.insert(time, ev);
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
        objects: &mut StopObjects,
    ) -> Result<()> {
        if channel == Channel::Stop {
            for (time, obj) in parse_obj_ids(track, message, prompter, &self.case_sensitive_obj_id)
            {
                // Record used STOP id for validity checks
                objects.stop_ids_used.insert(obj);
                let duration = objects
                    .stop_defs
                    .get(&obj)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                objects.push_stop(StopObj { time, duration });
            }
        }
        Ok(())
    }
}
