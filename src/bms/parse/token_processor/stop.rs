//! This module handles the tokens:
//!
//! - `#STOP[01-ZZ] n` - Stop definition. It stops the scroll as `n` of 192nd note.
//! - `#xxx09:` - Stop channel.
//! - `#STP xxx.yyy time` - It stops `time` milliseconds at section `xxx` and its position (`yyy` / 1000).
use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{
    super::prompt::{DefDuplication, Prompter},
    ParseWarning, Result, TokenProcessor, ids_from_message,
};
use crate::bms::{model::Bms, prelude::*};
use std::ops::ControlFlow;

/// It processes `#STOPxx` definitions and objects on `Stop` channel.
pub struct StopProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for StopProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> ControlFlow<Result<()>> {
        if name.to_ascii_uppercase().starts_with("STOP") {
            let id = &name["STOP".len()..];
            let len = match GenericFraction::from_str(args) {
                Ok(frac) => Decimal::from_fraction(frac),
                Err(_) => {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected decimal stop length".into(),
                    )));
                }
            };
            let stop_obj_id =
                match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                    Ok(v) => v,
                    Err(e) => return ControlFlow::Break(Err(e)),
                };

            if let Some(older) = self
                .0
                .borrow_mut()
                .scope_defines
                .stop_defs
                .get_mut(&stop_obj_id)
            {
                if let Err(e) = self
                    .1
                    .handle_def_duplication(DefDuplication::Stop {
                        id: stop_obj_id,
                        older: older.clone(),
                        newer: len.clone(),
                    })
                    .apply_def(older, len, stop_obj_id)
                {
                    return ControlFlow::Break(Err(e));
                }
            } else {
                self.0
                    .borrow_mut()
                    .scope_defines
                    .stop_defs
                    .insert(stop_obj_id, len);
            }
            return ControlFlow::Break(Ok(()));
        }
        #[cfg(feature = "minor-command")]
        if name.to_ascii_uppercase().starts_with("STP") {
            // Parse xxx.yyy zzzz
            use std::{num::NonZeroU64, time::Duration};
            let args: Vec<_> = args.split_whitespace().collect();
            if args.len() != 3 {
                return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                    "stp measure/pos must be 3 digits".into(),
                )));
            }

            let (measure, pos) = args[0].split_once('.').unwrap_or((args[0], "000"));
            let measure: u16 = match measure.parse() {
                Ok(v) => v,
                Err(_) => {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected measure u16".into(),
                    )));
                }
            };
            let pos: u16 = match pos.parse() {
                Ok(v) => v,
                Err(_) => {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected pos u16".into(),
                    )));
                }
            };
            let ms: u64 = match args[2].parse() {
                Ok(v) => v,
                Err(_) => {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected pos u64".into(),
                    )));
                }
            };
            let time = ObjTime::new(
                measure as u64,
                pos as u64,
                NonZeroU64::new(1000).expect("1000 should be a valid NonZeroU64"),
            );
            let duration = Duration::from_millis(ms);

            // Store by ObjTime as key, handle duplication with prompt handler
            let ev = StpEvent { time, duration };
            if let Some(older) = self.0.borrow_mut().arrangers.stp_events.get_mut(&time) {
                use crate::parse::prompt::ChannelDuplication;

                if let Err(e) = self
                    .1
                    .handle_channel_duplication(ChannelDuplication::StpEvent {
                        time,
                        older,
                        newer: &ev,
                    })
                    .apply_channel(older, ev, time, Channel::Stop)
                {
                    return ControlFlow::Break(Err(e));
                }
            } else {
                self.0.borrow_mut().arrangers.stp_events.insert(time, ev);
            }
            return ControlFlow::Break(Ok(()));
        }
        ControlFlow::Continue(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> ControlFlow<Result<()>> {
        if channel == Channel::Stop {
            let is_sensitive = self.0.borrow().header.case_sensitive_obj_id;
            for (time, obj) in ids_from_message(track, message, is_sensitive, |w| self.1.warn(w)) {
                // Record used STOP id for validity checks
                self.0.borrow_mut().arrangers.stop_ids_used.insert(obj);
                let duration = match self.0.borrow().scope_defines.stop_defs.get(&obj).cloned() {
                    Some(v) => v,
                    None => return ControlFlow::Break(Err(ParseWarning::UndefinedObject(obj))),
                };
                self.0
                    .borrow_mut()
                    .arrangers
                    .push_stop(StopObj { time, duration });
            }
            return ControlFlow::Break(Ok(()));
        }
        ControlFlow::Continue(())
    }
}
