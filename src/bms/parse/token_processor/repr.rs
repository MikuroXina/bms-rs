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

use super::{ProcessContext, TokenProcessor};
use crate::bms::ParseErrorWithRange;
use crate::bms::{
    model::repr::BmsSourceRepresentation,
    parse::{ParseWarning, Result},
    prelude::*,
};

/// It processes representation of BMS source such as `#BASE`, `#LNMODE` and so on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationProcessor {
    case_sensitive_obj_id: Rc<RefCell<bool>>,
}

impl RepresentationProcessor {
    pub fn new(case_sensitive_obj_id: &Rc<RefCell<bool>>) -> Self {
        Self {
            case_sensitive_obj_id: Rc::clone(case_sensitive_obj_id),
        }
    }
}

impl TokenProcessor for RepresentationProcessor {
    type Output = BmsSourceRepresentation;

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> core::result::Result<Self::Output, ParseErrorWithRange> {
        let mut repr = BmsSourceRepresentation::default();
        ctx.all_tokens(|token, _prompter| match token.content() {
            Token::Header { name, args } => Ok(self
                .on_header(name.as_ref(), args.as_ref(), &mut repr)
                .err()
                .map(|warn| warn.into_wrapper(token))),
            Token::Message { .. } | Token::NotACommand(_) => Ok(None),
        })?;
        Ok(repr)
    }
}

impl RepresentationProcessor {
    fn on_header(&self, name: &str, args: &str, repr: &mut BmsSourceRepresentation) -> Result<()> {
        if args.is_empty() {
            repr.raw_command_lines.push(format!("#{name}"));
        } else {
            repr.raw_command_lines.push(format!("#{name} {args}"));
        }
        if name.eq_ignore_ascii_case("BASE") {
            if args != "62" {
                return Err(ParseWarning::OutOfBase62);
            }
            *self.case_sensitive_obj_id.borrow_mut() = true;
        }
        if name.eq_ignore_ascii_case("LNMODE") {
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
            repr.ln_mode = mode;
        }
        if name.eq_ignore_ascii_case("LNTYPE") {
            repr.ln_type = if args == "2" {
                LnType::Mgq
            } else {
                LnType::Rdm
            };
        }
        if name.eq_ignore_ascii_case("CHARSET") {
            repr.charset = Some(args.into());
        }
        Ok(())
    }
}
