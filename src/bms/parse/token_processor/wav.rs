//! This module handles the tokens:
//!
//! - `#WAV[01-ZZ] path` - Sound file definition. Each one has own playback channel, so the sound of the same ID won't be played overlapping. ID range may be narrower by some BMS players.
//! - `#EXWAV[01-ZZ] [p/v/f pan volume frequency] path` - Sound file definition with effect. It defines a sound with applied some effects.
//! - `#LNOBJ wav_id` - It specifies the sound object `wav_id` as the end of a long note. Deprecated.
//! - `#WAVCMD command wav_id value` - It applies the effect to the sound object, for MacBeat.
//!   - `command` is `00`: Relative tone modification. Defaults to 60.
//!   - `command` is `01`: Relative volume percentage modification.
//!   - `command` is `02`: Changes playback time will be `value` of 0.5 milliseconds. 0 will do nothing.
//! - `#xxx01:` - BGM channel.
//! - `#xxx[11-1Z]:` - Player 1 visible channel.
//! - `#xxx[21-2Z]:` - Player 2 visible channel.
//! - `#xxx[31-3Z]:` - Player 1 invisible channel.
//! - `#xxx[41-4Z]:` - Player 2 invisible channel.
//! - `#xxx[51-5Z]:` - Player 1 long-note channel.
//! - `#xxx[61-6Z]:` - Player 2 long-note channel.
//! - `#xxx[D1-DZ]:` - Player 1 landmine channel with damage amount.
//! - `#xxx[E1-EZ]:` - Player 2 landmine channel with damage amount.
use std::{cell::RefCell, marker::PhantomData, path::Path, rc::Rc};

use super::{
    super::prompt::{DefDuplication, Prompter},
    ParseWarning, Result, TokenProcessor, ids_from_message,
};
use crate::bms::{model::Bms, prelude::*};
use std::ops::ControlFlow;

/// It processes `#WAVxx` and `#LNOBJ` definitions and objects on `Bgm` and `Note` channels.
pub struct WavProcessor<'a, P, T>(pub Rc<RefCell<Bms>>, pub &'a P, pub PhantomData<fn() -> T>);

impl<P: Prompter, T: KeyLayoutMapper> TokenProcessor for WavProcessor<'_, P, T> {
    fn on_header(&self, name: &str, args: &str) -> ControlFlow<Result<()>> {
        match name.to_ascii_uppercase().as_str() {
            wav if wav.starts_with("WAV") => {
                let id = &name["WAV".len()..];
                if args.is_empty() {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected key audio filename".into(),
                    )));
                }
                let path = Path::new(args);
                let wav_obj_id =
                    match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                        Ok(v) => v,
                        Err(e) => return ControlFlow::Break(Err(e)),
                    };
                if let Some(older) = self.0.borrow_mut().notes.wav_files.get_mut(&wav_obj_id) {
                    let res = self
                        .1
                        .handle_def_duplication(DefDuplication::Wav {
                            id: wav_obj_id,
                            older,
                            newer: path,
                        })
                        .apply_def(older, path.into(), wav_obj_id);
                    if let Err(e) = res {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    self.0
                        .borrow_mut()
                        .notes
                        .wav_files
                        .insert(wav_obj_id, path.into());
                }
                ControlFlow::Break(Ok(()))
            }
            #[cfg(feature = "minor-command")]
            ex_wav if ex_wav.starts_with("EXWAV") => {
                let id = &name["EXWAV".len()..];
                let mut args = args.split_whitespace();
                let Some(pvf_params) = args.next() else {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected parameters specified [pvf]".into(),
                    )));
                };
                let mut pan = None;
                let mut volume = None;
                let mut frequency = None;
                for param in pvf_params.bytes() {
                    match param {
                        b'p' => {
                            let pan_value_str = match args.next() {
                                Some(v) => v,
                                None => {
                                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                        "expected pan".into(),
                                    )));
                                }
                            };
                            let pan_value: i64 = match pan_value_str.parse() {
                                Ok(v) => v,
                                Err(_) => {
                                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                        "expected integer".into(),
                                    )));
                                }
                            };
                            let pan_conv = match ExWavPan::try_from(pan_value) {
                                Ok(v) => v,
                                Err(_) => {
                                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                        "expected pan value but out of range [-10000, 10000]"
                                            .into(),
                                    )));
                                }
                            };
                            pan = Some(pan_conv);
                        }
                        b'v' => {
                            let volume_value_str = match args.next() {
                                Some(v) => v,
                                None => {
                                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                        "expected volume".into(),
                                    )));
                                }
                            };
                            let volume_value: i64 = match volume_value_str.parse() {
                                Ok(v) => v,
                                Err(_) => {
                                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                        "expected integer".into(),
                                    )));
                                }
                            };
                            let vol_conv = match ExWavVolume::try_from(volume_value) {
                                Ok(v) => v,
                                Err(_) => {
                                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                        "expected volume value but out of range [-10000, 0]".into(),
                                    )));
                                }
                            };
                            volume = Some(vol_conv);
                        }
                        b'f' => {
                            let frequency_value_str = match args.next() {
                                Some(v) => v,
                                None => {
                                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                        "expected frequency".into(),
                                    )));
                                }
                            };
                            let frequency_value: u64 = match frequency_value_str.parse() {
                                Ok(v) => v,
                                Err(_) => {
                                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                        "expected integer".into(),
                                    )));
                                }
                            };
                            let freq_conv = match ExWavFrequency::try_from(frequency_value) {
                                Ok(v) => v,
                                Err(_) => {
                                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                        "expected frequency value but out of range [100, 100000]"
                                            .into(),
                                    )));
                                }
                            };
                            frequency = Some(freq_conv);
                        }
                        _ => {
                            return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                                "expected p, v or f".into(),
                            )));
                        }
                    }
                }
                let Some(file_name) = args.next() else {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected filename".into(),
                    )));
                };
                let id = match ObjId::try_from(id, self.0.borrow().header.case_sensitive_obj_id) {
                    Ok(v) => v,
                    Err(e) => return ControlFlow::Break(Err(e)),
                };
                let path = Path::new(file_name);
                let to_insert = ExWavDef {
                    id,
                    pan: pan.unwrap_or_default(),
                    volume: volume.unwrap_or_default(),
                    frequency,
                    path: path.into(),
                };
                if let Some(older) = self.0.borrow_mut().scope_defines.exwav_defs.get_mut(&id) {
                    let res = self
                        .1
                        .handle_def_duplication(DefDuplication::ExWav {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, id);
                    if let Err(e) = res {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    self.0
                        .borrow_mut()
                        .scope_defines
                        .exwav_defs
                        .insert(id, to_insert);
                }
                ControlFlow::Break(Ok(()))
            }
            "LNOBJ" => {
                let end_id =
                    match ObjId::try_from(args, self.0.borrow().header.case_sensitive_obj_id) {
                        Ok(v) => v,
                        Err(e) => return ControlFlow::Break(Err(e)),
                    };
                let mut end_note = match self.0.borrow_mut().notes.pop_latest_of::<T>(end_id) {
                    Some(v) => v,
                    None => return ControlFlow::Break(Err(ParseWarning::UndefinedObject(end_id))),
                };
                let WavObj {
                    offset, channel_id, ..
                } = &end_note;
                let begin_idx = match self
                    .0
                    .borrow()
                    .notes
                    .notes_in(..offset)
                    .rev()
                    .find(|(_, obj)| obj.channel_id == *channel_id)
                    .map(|(index, _)| index)
                {
                    Some(idx) => idx,
                    None => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(format!(
                            "expected preceding object for #LNOBJ {end_id:?}",
                        ))));
                    }
                };
                let mut begin_note = match self.0.borrow_mut().notes.pop_by_idx(begin_idx) {
                    Some(v) => v,
                    None => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(format!(
                            "Cannot find begin note for LNOBJ {end_id:?}"
                        ))));
                    }
                };

                let mut begin_note_tuple = match begin_note.channel_id.try_into_map::<T>() {
                    Some(map) => map.as_tuple(),
                    None => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(format!(
                            "channel of specified note for LNOBJ cannot become LN {end_id:?}"
                        ))));
                    }
                };
                begin_note_tuple.1 = NoteKind::Long;
                begin_note.channel_id = T::from_tuple(begin_note_tuple).to_channel_id();
                self.0.borrow_mut().notes.push_note(begin_note);

                let mut end_note_tuple = match end_note.channel_id.try_into_map::<T>() {
                    Some(map) => map.as_tuple(),
                    None => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(format!(
                            "channel of specified note for LNOBJ cannot become LN {end_id:?}"
                        ))));
                    }
                };
                end_note_tuple.1 = NoteKind::Long;
                end_note.channel_id = T::from_tuple(end_note_tuple).to_channel_id();
                self.0.borrow_mut().notes.push_note(end_note);
                ControlFlow::Break(Ok(()))
            }
            #[cfg(feature = "minor-command")]
            "WAVCMD" => {
                let args: Vec<_> = args.split_whitespace().collect();
                if args.len() != 3 {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected 3 arguments for #WAVCMD".into(),
                    )));
                }
                let param = match args[0] {
                    "00" => WavCmdParam::Pitch,
                    "01" => WavCmdParam::Volume,
                    "02" => WavCmdParam::Time,
                    _ => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "expected one of 00, 01, 02".into(),
                        )));
                    }
                };
                let wav_index =
                    match ObjId::try_from(args[1], self.0.borrow().header.case_sensitive_obj_id) {
                        Ok(v) => v,
                        Err(e) => return ControlFlow::Break(Err(e)),
                    };
                let value: u32 = match args[2].parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "wavcmd value u32".into(),
                        )));
                    }
                };
                // Validity check
                match param {
                    WavCmdParam::Pitch if !(0..=127).contains(&value) => {
                        return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                            "pitch must be in between 0 and 127".into(),
                        )));
                    }
                    WavCmdParam::Time => { /* 0 means original length, less than 50ms is unreliable */
                    }
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
                    let res = self
                        .1
                        .handle_def_duplication(DefDuplication::WavCmdEvent {
                            wav_index: key,
                            older,
                            newer: &ev,
                        })
                        .apply_def(older, ev, key);
                    if let Err(e) = res {
                        return ControlFlow::Break(Err(e));
                    }
                } else {
                    self.0
                        .borrow_mut()
                        .scope_defines
                        .wavcmd_events
                        .insert(key, ev);
                }
                ControlFlow::Break(Ok(()))
            }
            _ => ControlFlow::Continue(()),
        }
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> ControlFlow<Result<()>> {
        if channel == Channel::Bgm {
            let is_sensitive = self.0.borrow().header.case_sensitive_obj_id;
            for (time, obj) in ids_from_message(track, message, is_sensitive, |w| self.1.warn(w)) {
                self.0.borrow_mut().notes.push_bgm::<T>(time, obj);
            }
            return ControlFlow::Break(Ok(()));
        }
        if let Channel::Note { channel_id } = channel {
            let is_sensitive = self.0.borrow().header.case_sensitive_obj_id;
            for (offset, obj) in ids_from_message(track, message, is_sensitive, |w| self.1.warn(w))
            {
                self.0.borrow_mut().notes.push_note(WavObj {
                    offset,
                    channel_id,
                    wav_id: obj,
                });
            }
            return ControlFlow::Break(Ok(()));
        }
        ControlFlow::Continue(())
    }
}
