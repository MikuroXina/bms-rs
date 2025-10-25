//! This module handles the tokens:
//!
//! - `#VOLWAV n` - Changes the score's volume at `n`%.
//! - `#xxx97:` - BGM volume change channel. It changes BGM notes volume at `[01-FF]`. Obsolete.
//! - `#xxx98:` - Key volume change channel. It changes key notes volume at `[01-FF]`. Obsolete.

use super::{
    super::prompt::Prompter, TokenProcessor, TokenProcessorResult, all_tokens_with_range,
    parse_hex_values,
};
use crate::bms::{error::Result, model::volume::VolumeObjects, prelude::*};

/// It processes `#VOLWAV` definitions and objects on `BgmVolume` and `KeyVolume` channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VolumeProcessor;

impl TokenProcessor for VolumeProcessor {
    type Output = VolumeObjects;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> TokenProcessorResult<Self::Output> {
        let mut objects = VolumeObjects::default();
        all_tokens_with_range(input, prompter, |token| {
            Ok(match token.content() {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), &mut objects)
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

impl VolumeProcessor {
    fn on_header(&self, name: &str, args: &str, objects: &mut VolumeObjects) -> Result<()> {
        if name.to_ascii_uppercase().as_str() == "VOLWAV" {
            let volume = args
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let volume = Volume {
                relative_percent: volume,
            };
            objects.volume = volume;
        }
        Ok(())
    }

    fn on_message(
        &self,
        track: Track,
        channel: Channel,
        message: SourceRangeMixin<&str>,
        prompter: &impl Prompter,
        objects: &mut VolumeObjects,
    ) -> Result<()> {
        match channel {
            Channel::BgmVolume => {
                for (time, volume_value) in parse_hex_values(track, message, prompter) {
                    objects.push_bgm_volume_change(
                        BgmVolumeObj {
                            time,
                            volume: volume_value,
                        },
                        prompter,
                    )?;
                }
            }
            Channel::KeyVolume => {
                for (time, volume_value) in parse_hex_values(track, message, prompter) {
                    objects.push_key_volume_change(
                        KeyVolumeObj {
                            time,
                            volume: volume_value,
                        },
                        prompter,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
