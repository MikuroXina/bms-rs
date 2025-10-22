//! This module handles the tokens:
//!
//! - `#MIDIFILE path` - MIDI file path for the BGM. Deprecated.
//! - `#CDDA track_no` - Track number of a CD-DA for the BGM, for Delight Delight Republication.
//! - `#MATERIALSWAV path` - Specifies the shared audio path. Obsolete.
//! - `#MATERIALSBMP path` - Specifies the shared image path. Obsolete.
//! - `#MATERIALS path` - Unknown. Obsolete.
#![cfg(feature = "minor-command")]

use std::{cell::RefCell, path::Path, rc::Rc, str::FromStr};

use num::BigUint;

use super::{TokenProcessor, TokenProcessorResult, all_tokens};
use crate::{
    bms::{model::Bms, prelude::*},
    parse::Result,
};

/// It processes external resources such as `#MIDIFILE`, `#CDDA` and so on.
pub struct ResourcesProcessor(pub Rc<RefCell<Bms>>);

impl TokenProcessor for ResourcesProcessor {
    fn process(&self, input: &mut &[&TokenWithRange<'_>]) -> TokenProcessorResult {
        all_tokens(input, |token| {
            Ok(match token {
                Token::Header { name, args } => self.on_header(name.as_ref(), args.as_ref()).err(),
                Token::Message { .. } | Token::NotACommand(_) => None,
            })
        })
    }
}

impl ResourcesProcessor {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "MIDIFILE" => {
                if args.is_empty() {
                    return Err(ParseWarning::SyntaxError("expected midi filename".into()));
                }
                self.0.borrow_mut().notes.midi_file = Some(Path::new(args).into());
            }
            "CDDA" => {
                let big_uint = BigUint::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                self.0.borrow_mut().others.cdda.push(big_uint)
            }
            "MATERIALSWAV" => {
                self.0
                    .borrow_mut()
                    .notes
                    .materials_wav
                    .push(Path::new(args).into());
            }
            "MATERIALSBMP" => {
                self.0
                    .borrow_mut()
                    .graphics
                    .materials_bmp
                    .push(Path::new(args).into());
            }
            "MATERIALS" => {
                self.0.borrow_mut().others.materials_path = Some(Path::new(args).into());
            }
            _ => {}
        }
        Ok(())
    }
}
