use std::{cell::RefCell, path::Path, rc::Rc};

use super::{
    ParseWarning, Result, ids_from_message,
    prompt::{ChannelDuplication, DefDuplication, DuplicationWorkaround, TrackDuplication},
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

/// An interface to prompt about handling conflicts on the BMS file.
pub trait Prompter {
    /// Determines a [`DuplicationWorkaround`] for [`DefDuplication`].
    fn handle_def_duplication(&self, duplication: DefDuplication) -> DuplicationWorkaround;
    /// Determines a [`DuplicationWorkaround`] for [`TrackDuplication`].
    fn handle_track_duplication(&self, duplication: TrackDuplication) -> DuplicationWorkaround;
    /// Determines a [`DuplicationWorkaround`] for [`ChannelDuplication`].
    fn handle_channel_duplication(&self, duplication: ChannelDuplication) -> DuplicationWorkaround;
    /// Shows the user a [`ParseWarning`].
    fn warn(&self, warning: ParseWarning);
}

/// It processes `#WAVxx` definitions and objects on `Bgm` and `Note` channels.
pub struct WavProducer<'a, P>(Rc<RefCell<Bms>>, &'a P);

impl<P: Prompter> TokenProcessor for WavProducer<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name.to_uppercase().starts_with("WAV") {
            let id = name.trim_start_matches("WAV");
            if args.is_empty() {
                return Err(ParseWarning::SyntaxError("key audio filename".into()));
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
