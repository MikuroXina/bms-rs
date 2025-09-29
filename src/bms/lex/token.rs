//! Definitions of the token in BMS format.

use std::borrow::Cow;

use num::BigUint;

use super::LexWarning;
use crate::bms::{
    command::{LnMode, channel::Channel, mixin::SourceRangeMixin, time::Track},
    prelude::{SourceRangeMixinExt, read_channel},
};

use super::{Result, cursor::Cursor};

/// A token content of BMS format.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum Token<'a> {
    /// `#BASE 62`. Declares that the score is using base-62 object id format. If this exists, the score is treated as case-sensitive.
    Base62,
    /// `#CASE [u32]`. Starts a case scope if the integer equals to the generated random number. If there's no `#SKIP` command in the scope, the parsing will **fallthrough** to the next `#CASE` or `#DEF`. See also [`Token::Switch`].
    Case(BigUint),
    /// `#DEF`. Starts a case scope if any `#CASE` had not matched to the generated random number. It must be placed in the end of the switch scope. See also [`Token::Switch`].
    Def,
    /// `#ELSEIF [u32]`. Starts an if scope when the preceding `#IF` had not matched to the generated random number. It must be in an if scope.
    Else,
    /// `#ELSEIF [u32]`. Starts an if scope when the integer equals to the generated random number. It must be in an if scope. If preceding `#IF` had matched to the generated, this scope don't start. Syntax sugar for:
    ///
    /// ```text
    /// #ELSE
    ///   #IF n
    ///   // ...
    ///   #ENDIF
    /// #ENDIF
    /// ```
    ElseIf(BigUint),
    /// `#ENDIF`. Closes the if scope. See [`Token::If`].
    EndIf,
    /// `#ENDRANDOM`. Closes the random scope. See [`Token::Random`].
    EndRandom,
    /// `#ENDSW`. Closes the random scope. See [`Token::Switch`].
    EndSwitch,
    /// `#IF [u32]`. Starts an if scope when the integer equals to the generated random number. This must be placed in a random scope. See also [`Token::Random`].
    If(BigUint),
    /// `#LNMODE [1:LN, 2:CN, 3:HCN]` Explicitly specify LN type for this chart.
    LnMode(LnMode),
    /// `#LNTYPE 1`. Declares the LN notation as the RDM type.
    LnTypeRdm,
    /// `#LNTYPE 2`. Declares the LN notation as the MGQ type.
    LnTypeMgq,
    /// Non-empty lines that not starts in `'#'` in bms file.
    NotACommand(&'a str),
    /// `#RANDOM [u32]`. Starts a random scope which can contain only `#IF`-`#ENDIF` scopes. The random scope must close with `#ENDRANDOM`. A random integer from 1 to the integer will be generated when parsing the score. Then if the integer of `#IF` equals to the random integer, the commands in an if scope will be parsed, otherwise all command in it will be ignored. Any command except `#IF` and `#ENDIF` must not be included in the scope, but some players allow it.
    Random(BigUint),
    /// `#SETRANDOM [u32]`. Starts a random scope but the integer will be used as the generated random number. It should be used only for tests.
    SetRandom(BigUint),
    /// `#SETSWITCH [u32]`. Starts a switch scope but the integer will be used as the generated random number. It should be used only for tests.
    SetSwitch(BigUint),
    /// `#SKIP`. Escapes the current switch scope. It is often used in the end of every case scope.
    Skip,
    /// `#SWITCH [u32]`. Starts a switch scope which can contain only `#CASE` or `#DEF` scopes. The switch scope must close with `#ENDSW`. A random integer from 1 to the integer will be generated when parsing the score. Then if the integer of `#CASE` equals to the random integer, the commands in a case scope will be parsed, otherwise all command in it will be ignored. Any command except `#CASE` and `#DEF` must not be included in the scope, but some players allow it.
    Switch(BigUint),
    /// Unknown Part. Includes all the line that not be parsed.
    UnknownCommand(&'a str),
}

/// A token with position information.
pub type TokenWithRange<'a> = SourceRangeMixin<Token<'a>>;

impl<'a> Token<'a> {
    pub(crate) fn parse(c: &mut Cursor<'a>) -> Result<TokenWithRange<'a>> {
        let channel_parser = read_channel;
        let (command_range, command) = c
            .next_token_with_range()
            .ok_or_else(|| c.make_err_expected_token("command"))?;

        let token = match command.to_uppercase().as_str() {
            // Part: Normal
            "#LNTYPE" => {
                if c.next_token() == Some("2") {
                    Self::LnTypeMgq
                } else {
                    Self::LnTypeRdm
                }
            }
            // Part: ControlFlow/Random
            "#RANDOM" => {
                let rand_max = c
                    .next_token()
                    .ok_or_else(|| c.make_err_expected_token("random max"))?
                    .parse()
                    .map_err(|_| c.make_err_expected_token("integer"))?;
                Self::Random(rand_max)
            }
            "#SETRANDOM" => {
                let rand_value = c
                    .next_token()
                    .ok_or_else(|| c.make_err_expected_token("random value"))?
                    .parse()
                    .map_err(|_| c.make_err_expected_token("integer"))?;
                Self::SetRandom(rand_value)
            }
            "#IF" => {
                let rand_target = c
                    .next_token()
                    .ok_or_else(|| c.make_err_expected_token("random target"))?
                    .parse()
                    .map_err(|_| c.make_err_expected_token("integer"))?;
                Self::If(rand_target)
            }
            "#ELSEIF" => {
                let rand_target = c
                    .next_token()
                    .ok_or_else(|| c.make_err_expected_token("random target"))?
                    .parse()
                    .map_err(|_| c.make_err_expected_token("integer"))?;
                Self::ElseIf(rand_target)
            }
            "#ELSE" => Self::Else,
            "#ENDIF" => Self::EndIf,
            "#ENDRANDOM" => Self::EndRandom,
            // Part: ControlFlow/Switch
            "#SWITCH" => {
                let switch_max = c
                    .next_token()
                    .ok_or_else(|| c.make_err_expected_token("switch max"))?
                    .parse()
                    .map_err(|_| c.make_err_expected_token("integer"))?;
                Self::Switch(switch_max)
            }
            "#SETSWITCH" => {
                let switch_value = c
                    .next_token()
                    .ok_or_else(|| c.make_err_expected_token("switch value"))?
                    .parse()
                    .map_err(|_| c.make_err_expected_token("integer"))?;
                Self::SetSwitch(switch_value)
            }
            "#CASE" => {
                let case_value = c
                    .next_token()
                    .ok_or_else(|| c.make_err_expected_token("switch case value"))?
                    .parse()
                    .map_err(|_| c.make_err_expected_token("integer"))?;
                Self::Case(case_value)
            }
            "#SKIP" => Self::Skip,
            "#DEF" => Self::Def, // See https://hitkey.bms.ms/cmds.htm#DEF
            "#ENDSW" => Self::EndSwitch, // See https://hitkey.bms.ms/cmds.htm#ENDSW
            // Part: Normal 2
            "#BASE" => {
                let base = c.next_line_remaining();
                if base != "62" {
                    return Err(LexWarning::OutOfBase62.into_wrapper_range(command_range));
                }
                Self::Base62
            }
            lnmode if lnmode.starts_with("#LNMODE") => {
                let mode = c
                    .next_token()
                    .ok_or_else(|| c.make_err_expected_token("lnmode value"))?;
                let mode: u8 = mode
                    .parse()
                    .map_err(|_| c.make_err_expected_token("integer 1-3"))?;
                let mode = match mode {
                    1 => LnMode::Ln,
                    2 => LnMode::Cn,
                    3 => LnMode::Hcn,
                    _ => return Err(c.make_err_expected_token("lnmode 1-3")),
                };
                Self::LnMode(mode)
            }
            message
                if message.starts_with('#')
                    && message.chars().nth(6) == Some(':')
                    && 8 <= message.len() =>
            {
                let message_line = c.next_line_entire().trim_start();
                let track = message_line[1..4]
                    .parse()
                    .map_err(|_| c.make_err_expected_token("[000-999]"))?;
                let channel = &message_line[4..6];
                let message = &message_line[7..];
                Self::Message {
                    track: Track(track),
                    channel: channel_parser(channel)
                        .ok_or_else(|| c.make_err_unknown_channel(channel.to_string()))?,
                    message: Cow::Borrowed(message),
                }
            }
            // Unknown command & Not a command
            command if command.starts_with('#') => Self::UnknownCommand(c.next_line_entire()),
            _not_command => Self::NotACommand(c.next_line_entire()),
        };

        // Calculate the full range of this token (from command start to current cursor position)
        let token_range = command_range.start..c.index();
        Ok(SourceRangeMixin::new(token, token_range))
    }

    pub(crate) fn make_id_uppercase(&mut self) {}

    /// Checks if a token is a control flow token.
    #[must_use]
    pub const fn is_control_flow_token(&self) -> bool {
        matches!(
            self,
            Token::Random(_)
                | Token::SetRandom(_)
                | Token::If(_)
                | Token::ElseIf(_)
                | Token::Else
                | Token::EndIf
                | Token::EndRandom
                | Token::Switch(_)
                | Token::SetSwitch(_)
                | Token::Case(_)
                | Token::Def
                | Token::Skip
                | Token::EndSwitch
        )
    }
}

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Base62 => write!(f, "#BASE 62"),
            Token::Case(value) => write!(f, "#CASE {value}"),
            #[cfg(feature = "minor-command")]
            Token::Cdda(value) => write!(f, "#CDDA {value}"),
            Token::Def => write!(f, "#DEF"),
            Token::Else => write!(f, "#ELSE"),
            Token::ElseIf(value) => write!(f, "#ELSEIF {value}"),
            Token::EndIf => write!(f, "#ENDIF"),
            Token::EndRandom => write!(f, "#ENDRANDOM"),
            Token::EndSwitch => write!(f, "#ENDSW"),
            Token::If(value) => write!(f, "#IF {value}"),
            Token::LnMode(mode) => write!(
                f,
                "#LNMODE {}",
                match mode {
                    LnMode::Ln => 1,
                    LnMode::Cn => 2,
                    LnMode::Hcn => 3,
                }
            ),
            Token::LnTypeRdm => write!(f, "#LNTYPE 1"),
            Token::LnTypeMgq => write!(f, "#LNTYPE 2"),
            #[cfg(feature = "minor-command")]
            Token::Materials(path) => write!(f, "#MATERIALS {}", path.display()),
            #[cfg(feature = "minor-command")]
            Token::MaterialsBmp(path) => write!(f, "#MATERIALSBMP {}", path.display()),
            #[cfg(feature = "minor-command")]
            Token::MaterialsWav(path) => write!(f, "#MATERIALSWAV {}", path.display()),
            #[cfg(feature = "minor-command")]
            Token::MidiFile(path) => write!(f, "#MIDIFILE {}", path.display()),
            Token::NotACommand(content) => write!(f, "{content}"),
            #[cfg(feature = "minor-command")]
            Token::OctFp => write!(f, "#OCT/FP"),
            Token::Random(value) => write!(f, "#RANDOM {value}"),
            Token::SetRandom(value) => write!(f, "#SETRANDOM {value}"),
            Token::SetSwitch(value) => write!(f, "#SETSWITCH {value}"),
            Token::Skip => write!(f, "#SKIP"),
            Token::Switch(value) => write!(f, "#SWITCH {value}"),
            Token::UnknownCommand(cmd) => write!(f, "{cmd}"),
            #[cfg(feature = "minor-command")]
            Token::WavCmd(ev) => {
                use crate::bms::command::minor_command::WavCmdParam;

                let param = match ev.param {
                    WavCmdParam::Pitch => "00",
                    WavCmdParam::Volume => "01",
                    WavCmdParam::Time => "02",
                };
                write!(f, "#WAVCMD {} {} {}", param, ev.wav_index, ev.value)
            }
        }
    }
}

fn fmt_message(
    f: &mut std::fmt::Formatter<'_>,
    track: Track,
    channel: Channel,
    message: &str,
) -> std::fmt::Result {
    // Convert channel back to string representation
    match channel {
        Channel::BgaBase => {
            write!(f, "#{:03}04:{}", track.0, message)
        }
        Channel::BgaLayer => {
            write!(f, "#{:03}07:{}", track.0, message)
        }
        Channel::BgaPoor => {
            write!(f, "#{:03}06:{}", track.0, message)
        }
        Channel::Bgm => {
            write!(f, "#{:03}01:{}", track.0, message)
        }
        Channel::BpmChangeU8 => {
            write!(f, "#{:03}03:{}", track.0, message)
        }
        Channel::BpmChange => {
            write!(f, "#{:03}08:{}", track.0, message)
        }
        #[cfg(feature = "minor-command")]
        Channel::ChangeOption => {
            write!(f, "#{:03}A6:{}", track.0, message)
        }
        Channel::Note { channel_id } => {
            write!(f, "#{:03}{}:{}", track.0, channel_id, message)
        }
        Channel::SectionLen => {
            write!(f, "#{:03}02:{}", track.0, message)
        }
        Channel::Stop => {
            write!(f, "#{:03}09:{}", track.0, message)
        }
        Channel::Scroll => {
            write!(f, "#{:03}SC:{}", track.0, message)
        }
        Channel::Speed => {
            write!(f, "#{:03}SP:{}", track.0, message)
        }
        #[cfg(feature = "minor-command")]
        Channel::Seek => {
            write!(f, "#{:03}05:{}", track.0, message)
        }
        Channel::BgaLayer2 => {
            write!(f, "#{:03}0A:{}", track.0, message)
        }
        #[cfg(feature = "minor-command")]
        Channel::BgaBaseOpacity => {
            write!(f, "#{:03}0B:{}", track.0, message)
        }
        #[cfg(feature = "minor-command")]
        Channel::BgaLayerOpacity => {
            write!(f, "#{:03}0C:{}", track.0, message)
        }
        #[cfg(feature = "minor-command")]
        Channel::BgaLayer2Opacity => {
            write!(f, "#{:03}0D:{}", track.0, message)
        }
        #[cfg(feature = "minor-command")]
        Channel::BgaPoorOpacity => {
            write!(f, "#{:03}0E:{}", track.0, message)
        }
        Channel::BgmVolume => {
            write!(f, "#{:03}97:{}", track.0, message)
        }
        Channel::KeyVolume => {
            write!(f, "#{:03}98:{}", track.0, message)
        }
        Channel::Judge => {
            write!(f, "#{:03}A0:{}", track.0, message)
        }
        #[cfg(feature = "minor-command")]
        Channel::BgaBaseArgb => {
            write!(f, "#{:03}A1:{}", track.0, message)
        }
        #[cfg(feature = "minor-command")]
        Channel::BgaLayerArgb => {
            write!(f, "#{:03}A2:{}", track.0, message)
        }
        #[cfg(feature = "minor-command")]
        Channel::BgaLayer2Argb => {
            write!(f, "#{:03}A3:{}", track.0, message)
        }
        #[cfg(feature = "minor-command")]
        Channel::BgaPoorArgb => {
            write!(f, "#{:03}A4:{}", track.0, message)
        }
        #[cfg(feature = "minor-command")]
        Channel::BgaKeybound => {
            write!(f, "#{:03}A5:{}", track.0, message)
        }
        #[cfg(feature = "minor-command")]
        Channel::Option => {
            write!(f, "#{:03}A6:{}", track.0, message)
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "minor-command")]
    use crate::bms::command::time::Track;

    use super::*;

    fn parse_token(input: &'_ str) -> Token<'_> {
        let mut cursor = Cursor::new(input);
        Token::parse(&mut cursor).unwrap().into_content()
    }

    #[test]
    fn test_exbmp() {
        let Token::ExBmp(id, argb, path) = parse_token("#EXBMP01 255,0,0,0 exbmp.png") else {
            panic!("Not ExBmp");
        };
        assert_eq!(format!("{id:?}"), "ObjId(\"01\")");
        assert_eq!(argb.alpha, 255);
        assert_eq!(argb.red, 0);
        assert_eq!(argb.green, 0);
        assert_eq!(argb.blue, 0);
        assert_eq!(path, Path::new("exbmp.png"));
    }

    #[test]
    fn test_exrank() {
        let Token::ExRank(id, level) = parse_token("#EXRANK01 2") else {
            panic!("Not ExRank");
        };
        assert_eq!(format!("{id:?}"), "ObjId(\"01\")");
        assert_eq!(level, JudgeLevel::Normal);
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_exwav() {
        let Token::ExWav {
            id,
            pan,
            volume,
            frequency,
            path: file,
        } = parse_token("#EXWAV01 pvf 10000 0 48000 ex.wav")
        else {
            panic!("Not ExWav");
        };
        assert_eq!(format!("{id:?}"), "ObjId(\"01\")");
        assert_eq!(pan.value(), 10000);
        assert_eq!(volume.value(), 0);
        assert_eq!(frequency.map(|f| f.value()), Some(48000));
        assert_eq!(file, Path::new("ex.wav"));
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_exwav_2() {
        let Token::ExWav {
            id,
            pan,
            volume,
            frequency,
            path: file,
        } = parse_token("#EXWAV01 vpf 0 10000 48000 ex.wav")
        else {
            panic!("Not ExWav");
        };
        assert_eq!(format!("{id:?}"), "ObjId(\"01\")");
        assert_eq!(pan.value(), 10000);
        assert_eq!(volume.value(), 0);
        assert_eq!(frequency.map(|f| f.value()), Some(48000));
        assert_eq!(file, Path::new("ex.wav"));
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_exwav_default() {
        let Token::ExWav {
            id,
            pan,
            volume,
            frequency,
            path: file,
        } = parse_token("#EXWAV01 f 48000 ex.wav")
        else {
            panic!("Not ExWav");
        };
        assert_eq!(format!("{id:?}"), "ObjId(\"01\")");
        assert_eq!(pan, ExWavPan::default());
        assert_eq!(volume, ExWavVolume::default());
        assert_eq!(frequency.map(|f| f.value()), Some(48000));
        assert_eq!(file, Path::new("ex.wav"));
    }

    #[test]
    fn test_text() {
        let Token::Text(id, text) = parse_token("#TEXT01 hello world") else {
            panic!("Not Text");
        };
        assert_eq!(format!("{id:?}"), "ObjId(\"01\")");
        assert_eq!(text, "hello world");
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_atbga() {
        let Token::AtBga {
            id,
            source_bmp,
            trim_top_left,
            trim_size,
            draw_point,
        } = parse_token("#@BGA01 02 1 2 3 4 5 6")
        else {
            panic!("Not AtBga");
        };
        assert_eq!(format!("{id:?}"), "ObjId(\"01\")");
        assert_eq!(format!("{source_bmp:?}"), "ObjId(\"02\")");
        assert_eq!(trim_top_left, (1, 2));
        assert_eq!(trim_size, (3, 4));
        assert_eq!(draw_point, (5, 6));
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_bga() {
        let Token::Bga {
            id,
            source_bmp,
            trim_top_left,
            trim_bottom_right,
            draw_point,
        } = parse_token("#BGA01 02 1 2 3 4 5 6")
        else {
            panic!("Not Bga");
        };
        assert_eq!(format!("{id:?}"), "ObjId(\"01\")");
        assert_eq!(format!("{source_bmp:?}"), "ObjId(\"02\")");
        assert_eq!(trim_top_left, (1, 2));
        assert_eq!(trim_bottom_right, (3, 4));
        assert_eq!(draw_point, (5, 6));
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_changeoption() {
        let Token::ChangeOption(id, opt) = parse_token("#CHANGEOPTION01 opt") else {
            panic!("Not ChangeOption");
        };
        assert_eq!(format!("{id:?}"), "ObjId(\"01\")");
        assert_eq!(opt, "opt");
    }

    #[test]
    fn test_lnobj() {
        let Token::LnObj(id) = parse_token("#LNOBJ 01") else {
            panic!("Not LnObj");
        };
        assert_eq!(format!("{id:?}"), "ObjId(\"01\")");
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_stpseq() {
        let Token::Stp(stp) = parse_token("#STP 001.500 1500") else {
            panic!("Not StpSeq");
        };
        assert_eq!(stp.time.track(), Track(1));
        assert_eq!(stp.time.numerator(), 1);
        assert_eq!(stp.time.denominator().get(), 2); // After GCD(500, 1000)
        assert_eq!(stp.duration.as_millis(), 1500);
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_wavcmd_pitch() {
        let Token::WavCmd(ev) = parse_token("#WAVCMD 00 0E 61") else {
            panic!("Not WavCmd");
        };
        assert_eq!(ev.param, WavCmdParam::Pitch);
        assert_eq!(ev.wav_index, ObjId::try_from("0E").unwrap());
        assert_eq!(ev.value, 61);
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_wavcmd_volume() {
        let Token::WavCmd(ev) = parse_token("#WAVCMD 01 0E 50") else {
            panic!("Not WavCmd");
        };
        assert_eq!(ev.param, WavCmdParam::Volume);
        assert_eq!(ev.wav_index, ObjId::try_from("0E").unwrap());
        assert_eq!(ev.value, 50);
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_wavcmd_time() {
        let Token::WavCmd(ev) = parse_token("#WAVCMD 02 0E 100") else {
            panic!("Not WavCmd");
        };
        assert_eq!(ev.param, WavCmdParam::Time);
        assert_eq!(ev.wav_index, ObjId::try_from("0E").unwrap());
        assert_eq!(ev.value, 100);
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_swbga() {
        let Token::SwBga(id, ev) = parse_token("#SWBGA01 100:400:16:0:255,255,255,255 01020304")
        else {
            panic!("Not SwBga");
        };
        assert_eq!(id, ObjId::try_from("01").unwrap());
        assert_eq!(ev.frame_rate, 100);
        assert_eq!(ev.total_time, 400);
        assert_eq!(ev.line, 16);
        assert!(!ev.loop_mode);
        assert_eq!(
            ev.argb,
            Argb {
                alpha: 255,
                red: 255,
                green: 255,
                blue: 255
            }
        );
        assert_eq!(ev.pattern, "01020304");
    }

    #[test]
    fn test_movie() {
        let Token::Movie(path) = parse_token("#MOVIE video.mp4") else {
            panic!("Not Movie");
        };
        assert_eq!(path, Path::new("video.mp4"));
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_materials() {
        let Token::Materials(path) = parse_token("#MATERIALS /path/to/materials") else {
            panic!("Not Materials");
        };
        assert_eq!(path, Path::new("/path/to/materials"));
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_extchr_basic() {
        let token = parse_token("#ExtChr 512 09 30 0 99 9");
        let Token::ExtChr(ev) = token else {
            panic!("Not ExtChr");
        };
        assert_eq!(ev.sprite_num, 512);
        assert_eq!(ev.bmp_num, 9);
        assert_eq!(ev.start_x, 30);
        assert_eq!(ev.start_y, 0);
        assert_eq!(ev.end_x, 99);
        assert_eq!(ev.end_y, 9);
        assert_eq!(ev.offset_x, None);
        assert_eq!(ev.offset_y, None);
        assert_eq!(ev.abs_x, None);
        assert_eq!(ev.abs_y, None);
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_extchr_offset() {
        let token = parse_token("#ExtChr 516 0 38 1 62 9 -2 -2");
        let Token::ExtChr(ev) = token else {
            panic!("Not ExtChr: {token:?}");
        };
        assert_eq!(ev.offset_x, Some(-2));
        assert_eq!(ev.offset_y, Some(-2));
        assert_eq!(ev.abs_x, None);
        assert_eq!(ev.abs_y, None);
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_extchr_abs() {
        let token = parse_token("#ExtChr 513 0 38 1 62 9 -2 -2 0 0");
        let Token::ExtChr(ev) = token else {
            panic!("Not ExtChr: {token:?}");
        };
        assert_eq!(ev.offset_x, Some(-2));
        assert_eq!(ev.offset_y, Some(-2));
        assert_eq!(ev.abs_x, Some(0));
        assert_eq!(ev.abs_y, Some(0));
    }

    #[test]
    fn test_display_roundtrip() {
        // Test basic commands
        let test_cases = vec![
            "#ARTIST Test Artist",
            "#TITLE Test Title",
            "#GENRE Test Genre",
            "#MAKER Test Maker",
            "#COMMENT Test Comment",
            "#PLAYLEVEL 5",
            "#RANK 2",
            "#TOTAL 100",
            "#PLAYER 1",
            "#DIFFICULTY 3",
            "#BASE 62",
            "#LNTYPE 1",
            "#LNTYPE 2",
            "#VOLWAV 100",
            "#BANNER banner.png",
            "#BACKBMP back.png",
            "#STAGEFILE stage.png",
            "#PATH_WAV /path/to/wav",
            "#VIDEOFILE video.mp4",
            "#MOVIE movie.mp4",
            "#PREVIEW preview.wav",
            "%EMAIL test@example.com",
            "%URL http://example.com",
            "#CHARSET UTF-8",
            "#DEFEXRANK 100",
            "#LNMODE 1",
            "#LNMODE 2",
            "#LNMODE 3",
            "#POORBGA 0",
            "#POORBGA 1",
            "#POORBGA 2",
        ];

        for input in test_cases {
            let token = parse_token(input);
            let output = format!("{}", token);
            assert_eq!(input, output, "Failed roundtrip for: {}", input);
        }
    }

    #[test]
    fn test_display_with_id_commands() {
        // Test commands with object IDs
        let test_cases = vec![
            "#WAV01 test.wav",
            "#BMP01 test.bmp",
            "#BMP00 poor.bmp",
            "#STOP01 48",
            "#TEXT01 Hello World",
            "#LNOBJ 01",
        ];

        for input in test_cases {
            let token = parse_token(input);
            let output = format!("{}", token);
            assert_eq!(input, output, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_display_control_flow() {
        // Test control flow commands
        let test_cases = vec![
            "#RANDOM 5",
            "#SETRANDOM 3",
            "#IF 2",
            "#ELSEIF 4",
            "#ELSE",
            "#ENDIF",
            "#ENDRANDOM",
            "#SWITCH 3",
            "#SETSWITCH 1",
            "#CASE 2",
            "#DEF",
            "#SKIP",
            "#ENDSW",
        ];

        for input in test_cases {
            let token = parse_token(input);
            let output = format!("{}", token);
            assert_eq!(input, output, "Failed roundtrip for: {}", input);
        }
    }

    #[test]
    fn test_display_message() {
        // Test message commands
        let test_cases = vec![
            "#00101:01020304",
            "#00204:01020304",
            "#00308:01020304",
            "#004SC:01020304",
            "#005SP:01020304",
        ];

        for input in test_cases {
            let token = parse_token(input);
            let output = format!("{}", token);
            assert_eq!(input, output, "Failed roundtrip for: {}", input);
        }
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_display_minor_commands() {
        // Test minor commands
        let test_cases = vec![
            "#OCT/FP",
            "#OPTION Test Option",
            "#MIDIFILE test.mid",
            "#CHARFILE test.chp",
            "#MATERIALS /path/to/materials",
            "#MATERIALSBMP materials.bmp",
            "#MATERIALSWAV materials.wav",
            "#DIVIDEPROP 192",
            "#CDDA 12345",
            "#VIDEOCOLORS 16",
        ];

        for input in test_cases {
            let token = parse_token(input);
            let output = format!("{}", token);
            assert_eq!(input, output, "Failed roundtrip for: {}", input);
        }
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_display_complex_commands() {
        // Test complex commands with multiple parameters
        let test_cases = vec![
            "#EXBMP01 255,0,0,0 exbmp.png",
            "#EXRANK01 2",
            "#EXWAV01 pvf 10000 -1000 48000 ex.wav",
            "#@BGA01 02 1 2 3 4 5 6",
            "#BGA01 02 1 2 3 4 5 6",
            "#CHANGEOPTION01 opt",
            "#ARGB01 255,255,255,255",
            "#STP 001.500 1500",
            "#WAVCMD 00 0E 61",
            "#SWBGA01 100:400:16:0:255,255,255,255 01020304",
            "#ExtChr 512 9 30 0 99 9",
            "#ExtChr 516 0 38 1 62 9 -2 -2",
            "#ExtChr 513 0 38 1 62 9 -2 -2 0 0",
        ];

        for input in test_cases {
            let token = parse_token(input);
            let output = format!("{}", token);
            assert_eq!(input, output, "Failed for: {}", input);
        }
    }

    #[test]
    fn test_display_roundtrip_decimal() {
        // Test basic commands with decimal values
        let test_tokens = vec![Token::Bpm(Decimal::from(120.5))];

        for token in test_tokens {
            let output = format!("{}", token);
            let parsed_token = parse_token(&output);
            assert_eq!(token, parsed_token, "Failed roundtrip for: {}", output);
        }
    }

    #[test]
    fn test_display_with_id_commands_decimal() {
        // Test commands with object IDs and decimal values
        let test_tokens = vec![
            Token::BpmChange(
                ObjId::try_from("01").unwrap(),
                Decimal::from_fraction(GenericFraction::from_str("150.0").unwrap()),
            ),
            Token::Scroll(
                ObjId::try_from("01").unwrap(),
                Decimal::from_fraction(GenericFraction::from_str("2.0").unwrap()),
            ),
            Token::Speed(
                ObjId::try_from("01").unwrap(),
                Decimal::from_fraction(GenericFraction::from_str("1.5").unwrap()),
            ),
        ];

        for token in test_tokens {
            let output = format!("{}", token);
            let parsed_token = parse_token(&output);
            assert_eq!(token, parsed_token, "Failed for: {}", output);
        }
    }

    #[test]
    #[cfg(feature = "minor-command")]
    fn test_display_minor_commands_decimal() {
        // Test minor commands with decimal values
        let test_tokens = vec![
            Token::BaseBpm(Decimal::from(120.0)),
            Token::VideoDly(Decimal::from(1.5)),
            Token::VideoFs(Decimal::from(30.0)),
        ];

        for token in test_tokens {
            let output = format!("{}", token);
            let parsed_token = parse_token(&output);
            assert_eq!(token, parsed_token, "Failed roundtrip for: {}", output);
        }
    }
}
