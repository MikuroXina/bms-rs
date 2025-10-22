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

use super::{ParseWarning, TokenProcessor, TokenProcessorResult, all_tokens};
use crate::{
    bms::{model::Bms, prelude::*},
    parse::Result,
};

/// It processes representation of BMS source such as `#BASE`, `#LNMODE` and so on.
pub struct RepresentationProcessor(pub Rc<RefCell<Bms>>);

impl TokenProcessor for RepresentationProcessor {
    fn process(&self, input: &mut &[&TokenWithRange<'_>]) -> TokenProcessorResult {
        all_tokens(input, |token| {
            Ok(match token {
                Token::Header { name, args } => self.on_header(name.as_ref(), args.as_ref()).err(),
                Token::Message { .. } | Token::NotACommand(_) => None,
            })
        })
    }
}

impl RepresentationProcessor {
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
            "BASE" => {
                if args != "62" {
                    return Err(ParseWarning::OutOfBase62);
                }
                self.0.borrow_mut().header.case_sensitive_obj_id = true;
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
        Ok(())
    }
}
