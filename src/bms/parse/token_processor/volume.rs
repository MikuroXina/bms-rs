use std::{cell::RefCell, rc::Rc};

use super::{super::prompt::Prompter, Result, TokenProcessor, hex_values_from_message};
use crate::bms::{model::Bms, prelude::*};

/// It processes `#VOLWAV` definitions and objects on `BgmVolume` and `KeyVolume` channels.
pub struct VolumeProcessor<'a, P>(pub Rc<RefCell<Bms>>, pub &'a P);

impl<P: Prompter> TokenProcessor for VolumeProcessor<'_, P> {
    fn on_header(&self, name: &str, args: &str) -> Result<()> {
        if name == "VOLWAV" {
            let volume = args
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let volume = Volume {
                relative_percent: volume,
            };
            self.0.borrow_mut().header.volume = volume;
        }
        Ok(())
    }

    fn on_message(&self, track: Track, channel: Channel, message: &str) -> Result<()> {
        match channel {
            Channel::BgmVolume => {
                for (time, volume_value) in
                    hex_values_from_message(track, message, |w| self.1.warn(w))
                {
                    self.0.borrow_mut().notes.push_bgm_volume_change(
                        BgmVolumeObj {
                            time,
                            volume: volume_value,
                        },
                        self.1,
                    )?;
                }
            }
            Channel::KeyVolume => {
                for (time, volume_value) in
                    hex_values_from_message(track, message, |w| self.1.warn(w))
                {
                    self.0.borrow_mut().notes.push_key_volume_change(
                        KeyVolumeObj {
                            time,
                            volume: volume_value,
                        },
                        self.1,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
