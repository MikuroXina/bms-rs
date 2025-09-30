use std::{cell::RefCell, rc::Rc};

use super::{super::prompt::Prompter, ParseWarning, Result, TokenProcessor};
use crate::bms::{model::Bms, prelude::*};

/// It processes representation of BMS source such as `#BASE`, `#LNMODE` and so on.
pub struct RepresentationProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for RepresentationProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        match name {
            "BASE" => {
                if args != "62" {
                    return Err(ParseWarning::OutOfBase62);
                }
                // TODO: mark object ids case-sensitive
            }
            "LNMODE" => {
                let mode: u8 = args.parse().map_err(|_| {
                    ParseWarning::SyntaxError("expected integer between 1 and 3".into())
                })?;
                let mode = match mode {
                    1 => LnMode::Ln,
                    2 => LnMode::Cn,
                    3 => LnMode::Hcn,
                    _ => {
                        return Err(ParseWarning::SyntaxError(
                            "expected long note mode between 1 and 3".into(),
                        ));
                    }
                };
                self.0.borrow_mut().header.ln_mode = mode;
            }
            "LNTYPE" => {
                self.0.borrow_mut().header.ln_type = if args == "2" {
                    LnType::Mgq
                } else {
                    LnType::Rdm
                };
            }
            "CHARSET" => {
                // `#CHARSET` doesn't have a meaning because this library accepts only UTF-8.
            }
            _ => {}
        }
        todo!()
    }

    fn on_message(&self, _: Track, _: Channel, _: &str) -> Result<()> {
        Ok(())
    }
}
