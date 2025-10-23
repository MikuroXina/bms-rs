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
    TokenProcessor, TokenProcessorResult, all_tokens_with_range, parse_obj_ids,
};
use crate::bms::{error::Result, model::wav::WavObjects, prelude::*};

/// It processes `#WAVxx` and `#LNOBJ` definitions and objects on `Bgm` and `Note` channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavProcessor<T> {
    case_sensitive_obj_id: Rc<RefCell<bool>>,
    _phantom: PhantomData<fn() -> T>,
}

impl<T: KeyLayoutMapper> WavProcessor<T> {
    pub fn new(case_sensitive_obj_id: &Rc<RefCell<bool>>) -> Self {
        Self {
            case_sensitive_obj_id: Rc::clone(case_sensitive_obj_id),
            _phantom: PhantomData,
        }
    }
}

impl<T: KeyLayoutMapper> TokenProcessor for WavProcessor<T> {
    type Output = WavObjects;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorResult<Self::Output> {
        let mut objects = WavObjects::default();
        all_tokens_with_range(input, prompter, |token| {
            Ok(match token.content() {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), prompter, &mut objects)
                    .err(),
                Token::Message {
                    track,
                    channel,
                    message,
                } => self
                    .on_message(
                        *track,
                        *channel,
                        message.as_ref().into_wrapper(token),
                        prompter,
                        &mut objects,
                    )
                    .err(),
                Token::NotACommand(_) => None,
            })
        })?;
        Ok(objects)
    }
}

impl<T: KeyLayoutMapper> WavProcessor<T> {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        prompter: &impl Prompter,
        objects: &mut WavObjects,
    ) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            wav if wav.starts_with("WAV") => {
                let id = &name["WAV".len()..];
                if args.is_empty() {
                    return Err(ParseWarning::SyntaxError(
                        "expected key audio filename".into(),
                    ));
                }
                let path = Path::new(args);
                let wav_obj_id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;
                if let Some(older) = objects.wav_files.get_mut(&wav_obj_id) {
                    prompter
                        .handle_def_duplication(DefDuplication::Wav {
                            id: wav_obj_id,
                            older,
                            newer: path,
                        })
                        .apply_def(older, path.into(), wav_obj_id)?;
                } else {
                    objects.wav_files.insert(wav_obj_id, path.into());
                }
            }
            #[cfg(feature = "minor-command")]
            ex_wav if ex_wav.starts_with("EXWAV") => {
                let id = &name["EXWAV".len()..];
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
                                .map_err(|_| {
                                    ParseWarning::SyntaxError("expected integer".into())
                                })?;
                            pan = Some(ExWavPan::try_from(pan_value).map_err(|_| {
                                ParseWarning::SyntaxError(
                                    "expected pan value but out of range [-10000, 10000]".into(),
                                )
                            })?);
                        }
                        b'v' => {
                            let volume_value: i64 = args
                                .next()
                                .ok_or_else(|| ParseWarning::SyntaxError("expected volume".into()))?
                                .parse()
                                .map_err(|_| {
                                    ParseWarning::SyntaxError("expected integer".into())
                                })?;
                            volume = Some(ExWavVolume::try_from(volume_value).map_err(|_| {
                                ParseWarning::SyntaxError(
                                    "expected volume value but out of range [-10000, 0]".into(),
                                )
                            })?);
                        }
                        b'f' => {
                            let frequency_value: u64 = args
                                .next()
                                .ok_or_else(|| {
                                    ParseWarning::SyntaxError("expected frequency".into())
                                })?
                                .parse()
                                .map_err(|_| {
                                    ParseWarning::SyntaxError("expected integer".into())
                                })?;
                            frequency =
                                Some(ExWavFrequency::try_from(frequency_value).map_err(|_| {
                                    ParseWarning::SyntaxError(
                                        "expected frequency value but out of range [100, 100000]"
                                            .into(),
                                    )
                                })?);
                        }
                        _ => return Err(ParseWarning::SyntaxError("expected p, v or f".into())),
                    }
                }
                let Some(file_name) = args.next() else {
                    return Err(ParseWarning::SyntaxError("expected filename".into()));
                };
                let id = ObjId::try_from(id, *self.case_sensitive_obj_id.borrow())?;
                let path = Path::new(file_name);
                let to_insert = ExWavDef {
                    id,
                    pan: pan.unwrap_or_default(),
                    volume: volume.unwrap_or_default(),
                    frequency,
                    path: path.into(),
                };
                if let Some(older) = objects.exwav_defs.get_mut(&id) {
                    prompter
                        .handle_def_duplication(DefDuplication::ExWav {
                            id,
                            older,
                            newer: &to_insert,
                        })
                        .apply_def(older, to_insert, id)?;
                } else {
                    objects.exwav_defs.insert(id, to_insert);
                }
            }
            "LNOBJ" => {
                let end_id = ObjId::try_from(args, *self.case_sensitive_obj_id.borrow())?;
                let mut end_note = objects
                    .notes
                    .pop_latest_of::<T>(end_id)
                    .ok_or(ParseWarning::UndefinedObject(end_id))?;
                let WavObj {
                    offset, channel_id, ..
                } = &end_note;
                let begin_idx = objects
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
                let mut begin_note = objects.notes.pop_by_idx(begin_idx).ok_or_else(|| {
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
                objects.notes.push_note(begin_note);

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
                objects.notes.push_note(end_note);
            }
            #[cfg(feature = "minor-command")]
            "WAVCMD" => {
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
                let wav_index = ObjId::try_from(args[1], *self.case_sensitive_obj_id.borrow())?;
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
                if let Some(older) = objects.wavcmd_events.get_mut(&key) {
                    prompter
                        .handle_def_duplication(DefDuplication::WavCmdEvent {
                            wav_index: key,
                            older,
                            newer: &ev,
                        })
                        .apply_def(older, ev, key)?;
                } else {
                    objects.wavcmd_events.insert(key, ev);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_message(
        &self,
        track: Track,
        channel: Channel,
        message: SourceRangeMixin<&str>,
        prompter: &impl Prompter,
        objects: &mut WavObjects,
    ) -> Result<()> {
        if channel == Channel::Bgm {
            for (time, obj) in parse_obj_ids(
                track,
                message.clone(),
                prompter,
                &self.case_sensitive_obj_id,
            ) {
                objects.notes.push_bgm::<T>(time, obj);
            }
        }
        if let Channel::Note { channel_id } = channel {
            for (offset, obj) in
                parse_obj_ids(track, message, prompter, &self.case_sensitive_obj_id)
            {
                objects.notes.push_note(WavObj {
                    offset,
                    channel_id,
                    wav_id: obj,
                });
            }
        }
        Ok(())
    }
}
