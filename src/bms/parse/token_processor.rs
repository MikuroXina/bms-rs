use std::{cell::RefCell, path::Path, rc::Rc};

use fraction::GenericFraction;

use super::{
    ParseWarning, Result, hex_values_from_message, ids_from_message,
    prompt::{DefDuplication, Prompter},
};
use crate::bms::{model::Bms, prelude::*};

/// A processor of tokens in the BMS. An implementation takes control only one feature about definitions and placements such as `WAVxx` definition and its sound object.
///
/// There are some invariants on calling:
///
/// - Once `on_message` is called, `one_header` must not be invoked after that.
/// - The effects of called `on_message` must be same regardless order of calls.
pub trait TokenProcessor {
    /// Processes a header command consists of `#{name} {args}`.
    fn on_header(&self, name: &str, args: &str) -> Result<()>;
    /// Processes a message command consists of `#{track}{channel}:{message}`.
    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()>;
}

/// It processes `#WAVxx` definitions and objects on `Bgm` and `Note` channels.
pub struct WavProducer<'a, P>(Rc<RefCell<Bms>>, &'a P);

impl<P: Prompter> TokenProcessor for WavProducer<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name.to_uppercase().starts_with("WAV") {
            let id = name.trim_start_matches("WAV");
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError(
                    "expected key audio filename".into(),
                ));
            }
            let path = Path::new(args);
            let wav_obj_id = ObjId::try_from(id).map_err(|id| {
                ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
            })?;
            let mut bms = self.0.borrow_mut();
            if let Some(older) = bms.notes.wav_files.get_mut(&wav_obj_id) {
                self.1
                    .handle_def_duplication(DefDuplication::Wav {
                        id: wav_obj_id,
                        older,
                        newer: path,
                    })
                    .apply_def(older, path.into(), wav_obj_id)?;
            } else {
                bms.notes.wav_files.insert(wav_obj_id, path.into());
            }
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if let Channel::Bgm = channel {
            for (time, obj) in ids_from_message(track, message, |w| self.1.warn(w)) {
                self.0.borrow_mut().notes.push_bgm(time, obj);
            }
        }
        if let Channel::Note { channel_id } = channel {
            for (offset, obj) in ids_from_message(track, message, |w| self.1.warn(w)) {
                self.0.borrow_mut().notes.push_note(WavObj {
                    offset,
                    channel_id,
                    wav_id: obj,
                });
            }
        }
        Ok(())
    }
}

/// It processes `#BPM` and `#BPMxx` definitions and objects on `BpmChange` and `BpmChangeU8` channels.
pub struct BpmProcessor<'a, P>(Rc<RefCell<Bms>>, &'a P);

impl<P: Prompter> TokenProcessor for BpmProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        use std::str::FromStr;
        if name == "BPM" {
            let bpm = Decimal::from_fraction(
                GenericFraction::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected decimal BPM".into()))?,
            );
            self.0.borrow_mut().arrangers.bpm = Some(bpm);
        } else if name.starts_with("BPM") || name.starts_with("EXBPM") {
            let id = if name.starts_with("BPM") {
                name.trim_start_matches("BPM")
            } else {
                name.trim_start_matches("EXBPM")
            };
            let bpm_obj_id = ObjId::try_from(id).map_err(|id| {
                ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
            })?;
            let bpm = Decimal::from_fraction(
                GenericFraction::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected decimal BPM".into()))?,
            );
            let scope_defines = &mut self.0.borrow_mut().scope_defines;
            if let Some(older) = scope_defines.bpm_defs.get_mut(&bpm_obj_id) {
                self.1
                    .handle_def_duplication(DefDuplication::BpmChange {
                        id: bpm_obj_id,
                        older: older.clone(),
                        newer: bpm.clone(),
                    })
                    .apply_def(older, bpm.clone(), bpm_obj_id)?;
            } else {
                scope_defines.bpm_defs.insert(bpm_obj_id, bpm.clone());
            }
        }
        #[cfg(feature = "minor-command")]
        if name == "#BASEBPM" {
            let bpm = Decimal::from_fraction(
                GenericFraction::from_str(args)
                    .map_err(|_| ParseWarning::SyntaxError("expected decimal BPM".into()))?,
            );
            self.0.borrow_mut().arrangers.base_bpm = Some(bpm);
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if let Channel::BpmChange = channel {
            for (time, obj) in ids_from_message(track, message, |w| self.1.warn(w)) {
                // Record used BPM change id for validity checks
                self.0
                    .borrow_mut()
                    .arrangers
                    .bpm_change_ids_used
                    .insert(obj);
                let bpm = self
                    .0
                    .borrow_mut()
                    .scope_defines
                    .bpm_defs
                    .get(&obj)
                    .ok_or(ParseWarning::UndefinedObject(obj))?;
                self.0.borrow_mut().arrangers.push_bpm_change(
                    BpmChangeObj {
                        time,
                        bpm: bpm.clone(),
                    },
                    self.1,
                )?;
            }
        }
        if let Channel::BpmChangeU8 = channel {
            for (time, value) in hex_values_from_message(track, message, |w| self.1.warn(w)) {
                self.0.borrow_mut().arrangers.push_bpm_change(
                    BpmChangeObj {
                        time,
                        bpm: Decimal::from(value),
                    },
                    self.1,
                )?;
            }
        }
        Ok(())
    }
}
