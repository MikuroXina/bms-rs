//! This module handles the tokens:
//!
//! - `#BASE 62` - Marks the BMS source as object ids must be treated as case-sensitive.
//! - `#LNTYPE 1` - Declares the long-notes are pair-wise placements. Deprecated.
//! - `#LNTYPE 2` - Declares the long-notes are continuous placements. Obsolete.
//! - `#LNMODE mode` - Long note judgement option for beatoraja.
//! - `#CHARSET charset` - Declares charset used in the BMS source. It doesn't have any meaning to this library.
//!
//! Also [`RepresentationProcessor`] bears the responsibility of the first processor to record raw command lines.
use std::{cell::RefCell, rc::Rc};

use super::{ParseWarning, Result, TokenProcessor};
use crate::bms::{model::Bms, prelude::*};

/// It processes representation of BMS source such as `#BASE`, `#LNMODE` and so on.
pub struct RepresentationProcessor(pub Rc<RefCell<Bms>>);

impl TokenProcessor for RepresentationProcessor {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if args.is_empty() {
            self.0
                .borrow_mut()
                .others
                .raw_command_lines
                .push(format!("#{name}"));
        } else {
            self.0
                .borrow_mut()
                .others
                .raw_command_lines
                .push(format!("#{name} {args}"));
        }
        match name.to_ascii_uppercase().as_str() {
            "BASE" => match args {
                "62" => {
                    self.0.borrow_mut().header.case_sensitive_obj_id = true;
                }
                "16" | "36" => {}
                _ => {
                    return Err(ParseWarning::OutOfBase62);
                }
            },
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
        Ok(())
    }

    fn on_message(&self, _: Track, _: Channel, _: &str) -> Result<()> {
        Ok(())
    }
}
