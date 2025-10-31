//! This module handles the tokens:
//!
//! - `#VOLWAV n` - Changes the score's volume at `n`%.
//! - `#xxx97:` - BGM volume change channel. It changes BGM notes volume at `[01-FF]`. Obsolete.
//! - `#xxx98:` - Key volume change channel. It changes key notes volume at `[01-FF]`. Obsolete.

use super::{
    super::prompt::Prompter, TokenProcessor, all_tokens_with_range, parse_hex_values_with_warnings,
};
use crate::bms::{
    error::{ControlFlowWarningWithRange, Result},
    model::volume::VolumeObjects,
    prelude::*,
};

/// It processes `#VOLWAV` definitions and objects on `BgmVolume` and `KeyVolume` channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VolumeProcessor;

impl TokenProcessor for VolumeProcessor {
    type Output = VolumeObjects;

    fn process<P: Prompter>(
        &self,
        input: &mut &[&TokenWithRange<'_>],
        prompter: &P,
    ) -> (
        Self::Output,
        Vec<ParseWarningWithRange>,
        Vec<ControlFlowWarningWithRange>,
    ) {
        let mut objects = VolumeObjects::default();
        let mut all_warnings = Vec::new();
        let (_, warnings, errors) = all_tokens_with_range(input, prompter, |token| {
            Ok(match token.content() {
                Token::Header { name, args } => self
                    .on_header(name.as_ref(), args.as_ref(), &mut objects)
                    .err(),
                Token::Message {
                    track,
                    channel,
                    message,
                } => {
                    let message_warnings = self.on_message(
                        *track,
                        *channel,
                        message.as_ref().into_wrapper(token),
                        prompter,
                        &mut objects,
                    );
                    all_warnings.extend(message_warnings);
                    None
                }
                Token::NotACommand(_) => None,
            })
        });
        all_warnings.extend(warnings);
        (objects, all_warnings, errors)
    }
}

impl VolumeProcessor {
    fn on_header(&self, name: &str, args: &str, objects: &mut VolumeObjects) -> Result<()> {
        if name.eq_ignore_ascii_case("VOLWAV") {
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
    ) -> Vec<ParseWarningWithRange> {
        let mut warnings = Vec::new();
        match channel {
            Channel::BgmVolume => {
                let (hex_values, parse_warnings) =
                    parse_hex_values_with_warnings(track, message.clone(), prompter);
                warnings.extend(parse_warnings);
                for (time, volume_value) in hex_values {
                    if let Err(warning) = objects.push_bgm_volume_change(
                        BgmVolumeObj {
                            time,
                            volume: volume_value,
                        },
                        prompter,
                    ) {
                        warnings.push(warning.into_wrapper(&message));
                    }
                }
            }
            Channel::KeyVolume => {
                let (hex_values, parse_warnings) =
                    parse_hex_values_with_warnings(track, message.clone(), prompter);
                warnings.extend(parse_warnings);
                for (time, volume_value) in hex_values {
                    if let Err(warning) = objects.push_key_volume_change(
                        KeyVolumeObj {
                            time,
                            volume: volume_value,
                        },
                        prompter,
                    ) {
                        warnings.push(warning.into_wrapper(&message));
                    }
                }
            }
            _ => {}
        }
        warnings
    }
}
