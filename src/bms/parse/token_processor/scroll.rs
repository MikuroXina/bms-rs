//! This module handles the tokens:
//!
//! - `#SCROLL[01-ZZ] n` - Scrolling speed factor definition. It changes scrolling speed while keeps BPM.
//! - `#xxxSC:` - Scrolling speed factor channel.
use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{
    super::prompt::{DefDuplication, Prompter},
    ParseWarning, Result, TokenProcessor, ids_from_message,
};
use crate::bms::{model::Bms, prelude::*};
use std::ops::ControlFlow;

/// It processes `#SCROLLxx` definitions and objects on `Scroll` channel.
pub struct ScrollProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for ScrollProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> ControlFlow<Result<()>> {
        if name.to_ascii_uppercase().starts_with("SCROLL") {
            let id = &name["SCROLL".len()..];
            let factor = match GenericFraction::from_str(args) {
                Ok(frac) => Decimal::from_fraction(frac),
                Err(_) => {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected decimal scroll factor".into(),
                    )));
                }
            };
            let scroll_obj_id =
                match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                    Ok(v) => v,
                    Err(e) => return ControlFlow::Break(Err(e)),
                };
            if let Some(older) = self
                .0
                .borrow_mut()
                .scope_defines
                .scroll_defs
                .get_mut(&scroll_obj_id)
            {
                if let Err(e) = self
                    .1
                    .handle_def_duplication(DefDuplication::ScrollingFactorChange {
                        id: scroll_obj_id,
                        older: older.clone(),
                        newer: factor.clone(),
                    })
                    .apply_def(older, factor, scroll_obj_id)
                {
                    return ControlFlow::Break(Err(e));
                }
            } else {
                self.0
                    .borrow_mut()
                    .scope_defines
                    .scroll_defs
                    .insert(scroll_obj_id, factor);
            }
            return ControlFlow::Break(Ok(()));
        }
        ControlFlow::Continue(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> ControlFlow<Result<()>> {
        if channel == Channel::Scroll {
            let is_sensitive = self.0.borrow().header.case_sensitive_obj_id;
            for (time, obj) in ids_from_message(track, message, is_sensitive, |w| self.1.warn(w)) {
                let factor = match self.0.borrow().scope_defines.scroll_defs.get(&obj).cloned() {
                    Some(v) => v,
                    None => return ControlFlow::Break(Err(ParseWarning::UndefinedObject(obj))),
                };
                if let Err(e) = self
                    .0
                    .borrow_mut()
                    .arrangers
                    .push_scrolling_factor_change(ScrollingFactorObj { time, factor }, self.1)
                {
                    return ControlFlow::Break(Err(e));
                }
            }
            return ControlFlow::Break(Ok(()));
        }
        ControlFlow::Continue(())
    }
}
