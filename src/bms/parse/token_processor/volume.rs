//! This module handles the tokens:
//!
//! - `#VOLWAV n` - Changes the score's volume at `n`%.
//! - `#xxx97:` - BGM volume change channel. It changes BGM notes volume at `[01-FF]`. Obsolete.
//! - `#xxx98:` - Key volume change channel. It changes key notes volume at `[01-FF]`. Obsolete.
use std::{cell::RefCell, rc::Rc};

use super::{super::prompt::Prompter, Result, TokenProcessor, hex_values_from_message};
use crate::bms::{model::Bms, prelude::*};
use std::ops::ControlFlow;

/// It processes `#VOLWAV` definitions and objects on `BgmVolume` and `KeyVolume` channels.
pub struct VolumeProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for VolumeProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> ControlFlow<Result<()>> {
        if name.to_ascii_uppercase().as_str() == "VOLWAV" {
            let volume = match args.parse() {
                Ok(v) => v,
                Err(_) => {
                    return ControlFlow::Break(Err(ParseWarning::SyntaxError(
                        "expected integer".into(),
                    )));
                }
            };
            let volume = Volume {
                relative_percent: volume,
            };
            self.0.borrow_mut().header.volume = volume;
            return ControlFlow::Break(Ok(()));
        }
        ControlFlow::Continue(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> ControlFlow<Result<()>> {
        match channel {
            Channel::BgmVolume => {
                for (time, volume_value) in
                    hex_values_from_message(track, message, |w| self.1.warn(w))
                {
                    if let Err(e) = self.0.borrow_mut().notes.push_bgm_volume_change(
                        BgmVolumeObj {
                            time,
                            volume: volume_value,
                        },
                        self.1,
                    ) {
                        return ControlFlow::Break(Err(e));
                    }
                }
                return ControlFlow::Break(Ok(()));
            }
            Channel::KeyVolume => {
                for (time, volume_value) in
                    hex_values_from_message(track, message, |w| self.1.warn(w))
                {
                    if let Err(e) = self.0.borrow_mut().notes.push_key_volume_change(
                        KeyVolumeObj {
                            time,
                            volume: volume_value,
                        },
                        self.1,
                    ) {
                        return ControlFlow::Break(Err(e));
                    }
                }
                return ControlFlow::Break(Ok(()));
            }
            _ => {}
        }
        ControlFlow::Continue(())
    }
}
