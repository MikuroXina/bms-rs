use std::{cell::RefCell, rc::Rc, str::FromStr};

use fraction::GenericFraction;

use super::{super::prompt::Prompter, ParseWarning, Result, TokenProcessor, filter_message};
use crate::bms::{model::Bms, prelude::*};

/// It processes objects on `SectionLen` channel.
pub struct SectionLenProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for SectionLenProcessor<'_, P> {
    fn on_header(&self, _: &str, _: &str) -> Result<()> {
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if let Channel::SectionLen = channel {
            let message = filter_message(message);
            let message = message.as_ref();
            let length = Decimal::from(Decimal::from_fraction(
                GenericFraction::from_str(message).map_err(|_| {
                    ParseWarning::SyntaxError(format!("Invalid section length: {message}"))
                })?,
            ));
            if length <= Decimal::from(0u64) {
                return Err(ParseWarning::SyntaxError(
                    "section length must be greater than zero".to_string(),
                ));
            }
            self.0
                .borrow_mut()
                .arrangers
                .push_section_len_change(SectionLenChangeObj { track, length }, self.1)?;
        }
        Ok(())
    }
}
