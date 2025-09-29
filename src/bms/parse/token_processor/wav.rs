use std::{cell::RefCell, path::Path, rc::Rc};

use super::{
    super::prompt::{DefDuplication, Prompter},
    ParseWarning, Result, TokenProcessor, ids_from_message,
};
use crate::bms::{model::Bms, prelude::*};

/// It processes `#WAVxx` and `#LNOBJ` definitions and objects on `Bgm` and `Note` channels.
pub struct WavProcessor<'a, P, T>(Rc<RefCell<Bms<T>>>, &'a P);

impl<P: Prompter, T: KeyLayoutMapper> TokenProcessor for WavProcessor<'_, P, T> {
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
        if name == "LNOBJ" {
            let end_id = ObjId::try_from(args).map_err(|id| {
                ParseWarning::SyntaxError(format!("expected object id but found: {id}"))
            })?;
            let mut end_note = self
                .0
                .borrow_mut()
                .notes
                .pop_latest_of(end_id)
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
