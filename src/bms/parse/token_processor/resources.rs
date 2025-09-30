#[cfg(feature = "minor-command")]
use std::str::FromStr;
use std::{cell::RefCell, path::Path, rc::Rc};

#[cfg(feature = "minor-command")]
use num::BigUint;

use super::{Result, TokenProcessor};
use crate::bms::{model::Bms, prelude::*};

/// It processes external resources such as `#MIDIFILE`, `#CDDA` and so on.
pub struct ResourcesProcessor(pub Rc<RefCell<Bms>>);

impl TokenProcessor for ResourcesProcessor {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        match name {
            #[cfg(feature = "minor-command")]
            "MIDIFILE" => {
                if args.is_empty() {
                    return Err(ParseWarning::SyntaxError("expected midi filename".into()));
                }
                self.0.borrow_mut().notes.midi_file = Some(Path::new(args).into());
            }
            #[cfg(feature = "minor-command")]
            "CDDA" => {
                let big_uint = BigUint::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                self.0.borrow_mut().others.cdda.push(big_uint)
            }
            #[cfg(feature = "minor-command")]
            "MATERIALSWAV" => {
                self.0
                    .borrow_mut()
                    .notes
                    .materials_wav
                    .push(Path::new(args).into());
            }
            #[cfg(feature = "minor-command")]
            "MATERIALSBMP" => {
                self.0
                    .borrow_mut()
                    .graphics
                    .materials_bmp
                    .push(Path::new(args).into());
            }
            #[cfg(feature = "minor-command")]
            "MATERIALS" => {
                self.0.borrow_mut().others.materials_path = Some(Path::new(args).into());
            }
            _ => {}
        }
        Ok(())
    }

    fn on_message(&self, _: Track, _: Channel, _: &str) -> Result<()> {
        Ok(())
    }
}
