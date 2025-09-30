use std::{cell::RefCell, marker::PhantomData, path::Path, rc::Rc};

use super::{
    super::prompt::{DefDuplication, Prompter},
    ParseWarning, Result, TokenProcessor, ids_from_message,
};
use crate::bms::{model::Bms, prelude::*};

/// It processes `#WAVxx` and `#LNOBJ` definitions and objects on `Bgm` and `Note` channels.
pub struct WavProcessor<'a, P, T>(pub Rc<RefCell<Bms>>, pub &'a P, pub PhantomData<fn() -> T>);

impl<P: Prompter, T: KeyLayoutMapper> TokenProcessor for WavProcessor<'_, P, T> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name.starts_with("WAV") {
            let id = name.trim_start_matches("WAV");
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError(
                    "expected key audio filename".into(),
                ));
            }
            let path = Path::new(args);
            let wav_obj_id = ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id)?;
            if let Some(older) = self.0.borrow_mut().notes.wav_files.get_mut(&wav_obj_id) {
                self.1
                    .handle_def_duplication(DefDuplication::Wav {
                        id: wav_obj_id,
                        older,
                        newer: path,
                    })
                    .apply_def(older, path.into(), wav_obj_id)?;
            } else {
                self.0
                    .borrow_mut()
                    .notes
                    .wav_files
                    .insert(wav_obj_id, path.into());
            }
        }
        #[cfg(feature = "minor-command")]
        if name.starts_with("EXWAV") {
            let id = name.trim_start_matches("EXWAV");
            let mut args = args.split_whitespace();
            let Some(pvf_params) = args.next() else {
                return Err(ParseWarning::SyntaxError(
                    "expected parameters specified [pvf]".into(),
                ));
            };
            let mut pan = None;
            let mut volume = None;
            let mut frequency = None;
            for param in pvf_params.bytes() {
                match param {
                    b'p' => {
                        let pan_value: i64 = args
                            .next()
                            .ok_or_else(|| ParseWarning::SyntaxError("expected pan".into()))?
                            .parse()
                            .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                        pan = Some(ExWavPan::try_from(pan_value).map_err(|_| {
                            ParseWarning::SyntaxError(
                                "expected pan value out of range [-10000, 10000]".into(),
                            )
                        })?);
                    }
                    b'v' => {
                        let volume_value: i64 = args
                            .next()
                            .ok_or_else(|| ParseWarning::SyntaxError("expected volume".into()))?
                            .parse()
                            .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                        volume = Some(ExWavVolume::try_from(volume_value).map_err(|_| {
                            ParseWarning::SyntaxError(
                                "expected volume value out of range [-10000, 0]".into(),
                            )
                        })?);
                    }
                    b'f' => {
                        let frequency_value: u64 = args
                            .next()
                            .ok_or_else(|| ParseWarning::SyntaxError("expected frequency".into()))?
                            .parse()
                            .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
                        frequency =
                            Some(ExWavFrequency::try_from(frequency_value).map_err(|_| {
                                ParseWarning::SyntaxError(
                                    "expected frequency value out of range [100, 100000]".into(),
                                )
                            })?);
                    }
                    _ => return Err(ParseWarning::SyntaxError("expected p, v or f".into())),
                }
            }
            let Some(file_name) = args.next() else {
                return Err(ParseWarning::SyntaxError("expected filename".into()));
            };
            let id = ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id)?;
            let path = Path::new(file_name);
            let to_insert = ExWavDef {
                id,
                pan: pan.unwrap_or_default(),
                volume: volume.unwrap_or_default(),
                frequency,
                path: path.into(),
            };
            if let Some(older) = self.0.borrow_mut().scope_defines.exwav_defs.get_mut(&id) {
                self.1
                    .handle_def_duplication(DefDuplication::ExWav {
                        id,
                        older,
                        newer: &to_insert,
                    })
                    .apply_def(older, to_insert, id)?;
            } else {
                self.0
                    .borrow_mut()
                    .scope_defines
                    .exwav_defs
                    .insert(id, to_insert);
            }
        }
        if name == "LNOBJ" {
            let end_id = ObjId::try_from(args, self.0.borrow().header.case_sensitive_obj_id)?;
            let mut end_note = self
                .0
                .borrow_mut()
                .notes
                .pop_latest_of::<T>(end_id)
                .ok_or(ParseWarning::UndefinedObject(end_id))?;
            let WavObj {
                offset, channel_id, ..
            } = &end_note;
            let begin_idx = self
                .0
                .borrow()
                .notes
                .notes_in(..offset)
                .rev()
                .find(|(_, obj)| obj.channel_id == *channel_id)
                .ok_or_else(|| {
                    ParseWarning::SyntaxError(format!(
                        "expected preceding object for #LNOBJ {end_id:?}",
                    ))
                })
                .map(|(index, _)| index)?;
            let mut begin_note =
                self.0
                    .borrow_mut()
                    .notes
                    .pop_by_idx(begin_idx)
                    .ok_or_else(|| {
                        ParseWarning::SyntaxError(format!(
                            "Cannot find begin note for LNOBJ {end_id:?}"
                        ))
                    })?;

            let mut begin_note_tuple = begin_note
                .channel_id
                .try_into_map::<T>()
                .ok_or_else(|| {
                    ParseWarning::SyntaxError(format!(
                        "channel of specified note for LNOBJ cannot become LN {end_id:?}"
                    ))
                })?
                .as_tuple();
            begin_note_tuple.1 = NoteKind::Long;
            begin_note.channel_id = T::from_tuple(begin_note_tuple).to_channel_id();
            self.0.borrow_mut().notes.push_note(begin_note);

            let mut end_note_tuple = end_note
                .channel_id
                .try_into_map::<T>()
                .ok_or_else(|| {
                    ParseWarning::SyntaxError(format!(
                        "channel of specified note for LNOBJ cannot become LN {end_id:?}"
                    ))
                })?
                .as_tuple();
            end_note_tuple.1 = NoteKind::Long;
            end_note.channel_id = T::from_tuple(end_note_tuple).to_channel_id();
            self.0.borrow_mut().notes.push_note(end_note);
        }
        #[cfg(feature = "minor-command")]
        if name == "WAVCMD" {
            let args: Vec<_> = args.split_whitespace().collect();
            if args.len() != 3 {
                return Err(ParseWarning::SyntaxError(
                    "expected 3 arguments for #WAVCMD".into(),
                ));
            }
            let param = match args[0] {
                "00" => WavCmdParam::Pitch,
                "01" => WavCmdParam::Volume,
                "02" => WavCmdParam::Time,
                _ => {
                    return Err(ParseWarning::SyntaxError(
                        "expected one of 00, 01, 02".into(),
                    ));
                }
            };
            let wav_index = ObjId::try_from(args[1], self.0.borrow().header.case_sensitive_obj_id)?;
            let value: u32 = args[2]
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("wavcmd value u32".into()))?;
            // Validity check
            match param {
                WavCmdParam::Pitch if !(0..=127).contains(&value) => {
                    return Err(ParseWarning::SyntaxError(
                        "pitch must be in between 0 and 127".into(),
                    ));
                }
                WavCmdParam::Time => { /* 0 means original length, less than 50ms is unreliable */ }
                _ => {}
            }
            let ev = WavCmdEvent {
                param,
                wav_index,
                value,
            };

            // Store by wav_index as key, handle duplication with prompt handler
            let key = ev.wav_index;
            if let Some(older) = self
                .0
                .borrow_mut()
                .scope_defines
                .wavcmd_events
                .get_mut(&key)
            {
                self.1
                    .handle_def_duplication(DefDuplication::WavCmdEvent {
                        wav_index: key,
                        older,
                        newer: &ev,
                    })
                    .apply_def(older, ev, key)?;
            } else {
                self.0
                    .borrow_mut()
                    .scope_defines
                    .wavcmd_events
                    .insert(key, ev);
            }
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        if let Channel::Bgm = channel {
            for (time, obj) in ids_from_message(
                track,
                message,
                self.0.borrow().header.case_sensitive_obj_id,
                |w| self.1.warn(w),
            ) {
                self.0.borrow_mut().notes.push_bgm::<T>(time, obj);
            }
        }
        if let Channel::Note { channel_id } = channel {
            for (offset, obj) in ids_from_message(
                track,
                message,
                self.0.borrow().header.case_sensitive_obj_id,
                |w| self.1.warn(w),
            ) {
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
