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
    ParseWarning, Result, TokenProcessor, hex_values_from_message, ids_from_message,
};
use crate::bms::{model::Bms, prelude::*};
use std::ops::ControlFlow;

/// It processes `#BPM` and `#BPMxx` definitions and objects on `BpmChange` and `BpmChangeU8` channels.
pub struct BpmProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for BpmProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> ControlFlow<Result<()>> {
        match name.to_ascii_uppercase().as_str() {
            "BPM" => {
                let frac = match GenericFraction::from_str(args) {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected decimal BPM".into(),
                        )));
                    }
                };
                let bpm = Decimal::from_fraction(frac);
                self.0.borrow_mut().arrangers.bpm = Some(bpm);
            }
            bpm if bpm.starts_with("BPM") || bpm.starts_with("EXBPM") => {
                let id = if bpm.starts_with("BPM") {
                    &name["BPM".len()..]
                } else {
                    &name["EXBPM".len()..]
                };
                let bpm_obj_id =
                    match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                        Ok(v) => v,
                        Err(e) => return ControlFlow::Break(Err(e)),
                    };
                let frac = match GenericFraction::from_str(args) {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected decimal BPM".into(),
                        )));
                    }
                };
                let bpm = Decimal::from_fraction(frac);
                let scope_defines = &mut self.0.borrow_mut().scope_defines;
                if let Some(older) = scope_defines.bpm_defs.get_mut(&bpm_obj_id) {
                    if let Err(e) = self
                        .1
                        .handle_def_duplication(DefDuplication::BpmChange {
                            id: bpm_obj_id,
                            older: older.clone(),
                            newer: bpm.clone(),
                        })
                        .apply_def(older, bpm, bpm_obj_id)
                    {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    scope_defines.bpm_defs.insert(bpm_obj_id, bpm);
                }
            }
            #[cfg(feature = "minor-command")]
            "BASEBPM" => {
                let frac = match GenericFraction::from_str(args) {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected decimal BPM".into(),
                        )));
                    }
                };
                let bpm = Decimal::from_fraction(frac);
                self.0.borrow_mut().arrangers.base_bpm = Some(bpm);
            }
            _ => {
                return ControlFlow::Continue(());
            }
        }
        ControlFlow::Break(Ok(()))
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> ControlFlow<Result<()>> {
        if channel == Channel::BpmChange {
            let is_sensitive = self.0.borrow().header.case_sensitive_obj_id;
            for (time, obj) in ids_from_message(track, message, is_sensitive, |w| self.1.warn(w)) {
                // Record used BPM change id for validity checks
                self.0
                    .borrow_mut()
                    .arrangers
                    .bpm_change_ids_used
                    .insert(obj);
                let bpm = match self.0.borrow().scope_defines.bpm_defs.get(&obj).cloned() {
                    Some(v) => v,
                    None => return ControlFlow::Break(Err(ParseWarning::UndefinedObject(obj))),
                };
                if let Err(e) = self
                    .0
                    .borrow_mut()
                    .arrangers
                    .push_bpm_change(BpmChangeObj { time, bpm }, self.1)
                {
                    return ControlFlow::Break(Err(e));
                }
            }
            return ControlFlow::Break(Ok(()));
        }
        if channel == Channel::BpmChangeU8 {
            for (time, value) in hex_values_from_message(track, message, |w| self.1.warn(w)) {
                if let Err(e) = self
                    .0
                    .borrow_mut()
                    .arrangers
                    .push_bpm_change_u8(time, value, self.1)
                {
                    return ControlFlow::Break(Err(e));
                }
            }
            return ControlFlow::Break(Ok(()));
        }
        ControlFlow::Continue(())
    }
}
