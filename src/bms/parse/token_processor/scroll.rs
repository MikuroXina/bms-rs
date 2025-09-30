use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{
    super::prompt::{DefDuplication, Prompter},
    ParseWarning, Result, TokenProcessor, ids_from_message,
};
use crate::bms::{model::Bms, prelude::*};

/// It processes `#SCROLLxx` definitions and objects on `Scroll` channel.
pub struct ScrollProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for ScrollProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name.starts_with("SCROLL") {
            let id = name.trim_start_matches("SCROLL");
            let factor =
                Decimal::from_fraction(GenericFraction::from_str(args).map_err(|_| {
                    ParseWarning::SyntaxError("expected decimal scroll factor".into())
                })?);
            let scroll_obj_id = ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id)?;
            if let Some(older) = self
                .0
                .borrow_mut()
                .scope_defines
                .scroll_defs
                .get_mut(&scroll_obj_id)
            {
                self.1
                    .handle_def_duplication(DefDuplication::ScrollingFactorChange {
                        id: scroll_obj_id,
                        older: older.clone(),
                        newer: factor.clone(),
                    })
                    .apply_def(older, factor, scroll_obj_id)?;
            } else {
                self.0
                    .borrow_mut()
                    .scope_defines
                    .scroll_defs
                    .insert(scroll_obj_id, factor);
            }
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if let Channel::Scroll = channel {
            for (time, obj) in ids_from_message(
                    track,
                    message,
                    self.0.borrow().header.case_sensitive_obj_id,
                    |w| self.1.warn(w),
                ) {
                let factor = self
                    .0
                    .borrow()
                    .scope_defines
                    .scroll_defs
                    .get(&obj)
                    .cloned()
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                self.0
                    .borrow_mut()
                    .arrangers
                    .push_scrolling_factor_change(ScrollingFactorObj { time, factor }, self.1)?;
            }
        }
        Ok(())
    }
}
