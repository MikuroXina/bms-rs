//! This module handles the tokens:
//!
//! - `#VOLWAV n` - Changes the score's volume at `n`%.
//! - `#xxx97:` - BGM volume change channel. It changes BGM notes volume at `[01-FF]`. Obsolete.
//! - `#xxx98:` - Key volume change channel. It changes key notes volume at `[01-FF]`. Obsolete.

use super::ParseWarningCollectior;
use super::{
    super::prompt::Prompter, ProcessContext, TokenProcessor, all_tokens, parse_hex_values,
};
use crate::bms::ParseErrorWithRange;
use crate::bms::{model::volume::VolumeObjects, prelude::*};

/// It processes `#VOLWAV` definitions and objects on `BgmVolume` and `KeyVolume` channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VolumeProcessor;

impl TokenProcessor for VolumeProcessor {
    type Output = VolumeObjects;

    fn process<'a, 't, P: Prompter>(
        &self,
        ctx: &mut ProcessContext<'a, 't, P>,
    ) -> Result<Self::Output, ParseErrorWithRange> {
        let mut objects = VolumeObjects::default();
        let mut buffered_warnings = Vec::new();
        let tokens_view = ctx.take_input();
        let mut syntactic_warnings: Vec<ParseWarningWithRange> = Vec::new();
        let prompter = ctx.prompter();
        all_tokens(tokens_view, &mut syntactic_warnings, |token| {
            match token.content() {
                Token::Header { name, args } => Ok(self
                    .on_header(name.as_ref(), args.as_ref(), &mut objects)
                    .err()),
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
                    .map_or_else(
                        |warn| Ok(Some(warn)),
                        |ws| {
                            buffered_warnings.extend(ws);
                            Ok(None)
                        },
                    ),
                Token::NotACommand(_) => Ok(None),
            }
        })?;
        {
            let mut wc = ctx.get_warning_collector();
            wc.collect(syntactic_warnings);
            wc.collect(buffered_warnings);
        }
        Ok(objects)
    }
}

impl VolumeProcessor {
    fn on_header(
        &self,
        name: &str,
        args: &str,
        volume: &mut VolumeObjects,
    ) -> core::result::Result<(), ParseWarning> {
        if name.eq_ignore_ascii_case("VOLWAV") {
            let volume_value = args
                .parse()
                .map_err(|_| ParseWarning::SyntaxError("expected integer".into()))?;
            let volume_obj = Volume {
                relative_percent: volume_value,
            };
            volume.volume = volume_obj;
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
    ) -> core::result::Result<Vec<ParseWarningWithRange>, ParseWarning> {
        let mut warnings: Vec<ParseWarningWithRange> = Vec::new();
        match channel {
            Channel::BgmVolume => {
                let (pairs, w) = parse_hex_values(track, message);
                warnings.extend(w);
                for (time, volume_value) in pairs {
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
                let (pairs, w) = parse_hex_values(track, message);
                warnings.extend(w);
                for (time, volume_value) in pairs {
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
        Ok(warnings)
    }
}
