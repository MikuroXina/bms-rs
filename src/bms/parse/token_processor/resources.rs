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

use super::{Result, TokenProcessor};
use crate::bms::{model::Bms, prelude::*};
use std::ops::ControlFlow;

/// It processes external resources such as `#MIDIFILE`, `#CDDA` and so on.
pub struct ResourcesProcessor(pub Rc<RefCell<Bms>>);

impl TokenProcessor for ResourcesProcessor {
    fn on_header(&self, name: &str, args: &str) -> ControlFlow<Result<()>> {
        match name.to_ascii_uppercase().as_str() {
            "MIDIFILE" => {
                if args.is_empty() {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected midi filename".into(),
                    )));
                }
                self.0.borrow_mut().notes.midi_file = Some(Path::new(args).into());
            }
            "CDDA" => {
                let big_uint = match BigUint::from_str(args) {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected integer".into(),
                        )));
                    }
                };
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
            _ => {
                return ControlFlow::Continue(());
            }
        }
        ControlFlow::Break(Ok(()))
    }

    fn on_message(&self, _: Track, _: Channel, _: &str) -> ControlFlow<Result<()>> {
        ControlFlow::Continue(())
    }
}
