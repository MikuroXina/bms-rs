//! Definitions of the token in BMS format.

use std::{borrow::Cow, path::Path, str::FromStr};

#[cfg(feature = "minor-command")]
use std::time::Duration;

use fraction::GenericFraction;
use num::BigUint;

use crate::bms::{
    Decimal,
    command::{
        Argb, JudgeLevel, LnModeType, ObjId, PlayerMode, PoorMode, Volume, channel::Channel,
        time::Track,
    },
};

#[cfg(feature = "minor-command")]
use crate::bms::command::{
    ExtChrEvent, WavCmdParam,
    minor_command::{ExWavFrequency, ExWavPan, ExWavVolume, StpEvent, SwBgaEvent, WavCmdEvent},
};

use super::{Result, cursor::Cursor};

/// A token of BMS format.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum Token<'a> {
    /// `#ARGB[A1-A4] [A],[R],[G],[B]` Extended transparent color definition.
    /// - A1: BGA BASE
    /// - A2: BGA LAYER
    /// - A3: BGA LAYER 2
    /// - A4: BGA POOR
    #[cfg(feature = "minor-command")]
    Argb(ObjId, Argb),
    /// `#ARTIST [string]`. Defines the artist name of the music.
    Artist(&'a str),
    /// `#@BGA[01-ZZ] [01-ZZ] [sx] [sy] [w] [h] [dx] [dy]`. Defines the image object from trimming the existing image object.
    #[cfg(feature = "minor-command")]
    AtBga {
        /// The id of the object to define.
        id: ObjId,
        /// The id of the object to be trimmed.
        source_bmp: ObjId,
        /// The top left point of the trim area in pixels.
        trim_top_left: (i16, i16),
        /// The size of the trim area in pixels.
        trim_size: (u16, u16),
        /// The top left point to be rendered in pixels.
        draw_point: (i16, i16),
    },
    /// `#BANNER [filename]`. Defines the banner image. This can be used on music select or result view. It should be 300x80.
    Banner(&'a Path),
    /// `#BACKBMP [filename]`. Defines the background image file of the play view. It should be 640x480. The effect will depend on the skin of the player.
    BackBmp(&'a Path),
    /// `#BASE 62`. Declares that the score is using base-62 object id format. If this exists, the score is treated as case-sensitive.
    Base62,
    /// `#BASEBPM [f64]` is the base BPM.
    /// It's not used in LunaticRave2, replaced by its Hi-Speed Settings.
    #[cfg(feature = "minor-command")]
    BaseBpm(Decimal),
    /// `#BGA[01-ZZ] [01-ZZ] [x1] [y1] [x2] [y2] [dx] [dy]`. Defines the image object from trimming the existing image object.
    #[cfg(feature = "minor-command")]
    Bga {
        /// The id of the object to define.
        id: ObjId,
        /// The id of the object to be trimmed.
        source_bmp: ObjId,
        /// The top left point of the trim area in pixels.
        trim_top_left: (i16, i16),
        /// The bottom right point of the trim area in pixels.
        trim_bottom_right: (i16, i16),
        /// The top left point to be rendered in pixels.
        draw_point: (i16, i16),
    },
    /// `#BMP[01-ZZ] [filename]`. Defines the background image/movie object. The file specified may be not only BMP format, and also PNG, AVI, MP4, MKV and others. Its size should be less than or equal to 256x256. The black (`#000000`) pixel in the image will be treated as transparent. When the id `00` is specified, this first field will be `None` and the image will be shown when the player get mistaken.
    Bmp(Option<ObjId>, &'a Path),
    /// `#BPM [f64]`. Defines the base Beats-Per-Minute of the score. Defaults to 130, but some players don't conform to it.
    Bpm(Decimal),
    /// `#BPM[01-ZZ] [f64]`. Defines the Beats-Per-Minute change object.
    BpmChange(ObjId, Decimal),
    /// `#CASE [u32]`. Starts a case scope if the integer equals to the generated random number. If there's no `#SKIP` command in the scope, the parsing will **fallthrough** to the next `#CASE` or `#DEF`. See also [`Token::Switch`].
    Case(BigUint),
    /// `#CDDA [u64]`.
    /// CD-DA can be used as BGM. In DDR, a config of `CD-Syncro` in `SYSTEM OPTION` is also applied.
    #[cfg(feature = "minor-command")]
    Cdda(BigUint),
    /// `#CHANGEOPTION[01-ZZ] [string]`. Defines the play option change object. Some players interpret and apply the preferences.
    ChangeOption(ObjId, &'a str),
    /// `#CHARFILE [filename]`.
    /// The character file similar to pop'n music. It's filextension is `.chp`.
    /// For now, `#CHARFILE` is a pomu2 proprietary extension. However, the next-generation version LunaticRave may support `#CHARFILE`.
    #[cfg(feature = "minor-command")]
    CharFile(&'a Path),
    /// `#CHARSET [string]` Charset declaration. Default is SHIFT-JIS.
    Charset(&'a str),
    /// `#COMMENT [string]`. Defines the text which is shown in the music select view. This may or may not be surrounded by double-quotes.
    Comment(&'a str),
    /// `#DEF`. Starts a case scope if any `#CASE` had not matched to the generated random number. It must be placed in the end of the switch scope. See also [`Token::Switch`].
    Def,
    /// `#DEFEXRANK [u64]` Extended judge rank definition, defined as n% of the original.
    /// 100 means NORMAL judge.
    /// Overrides `#RANK` definition.
    DefExRank(u64),
    /// `#DIFFICULTY [1-5]`. Defines the difficulty of the score. It can be used to sort the score having the same title.
    Difficulty(u8),
    /// `#DIVIDEPROP [string]` The resolution of Measure of BMS is specified.
    /// Deprecated.
    #[cfg(feature = "minor-command")]
    DivideProp(&'a str),
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
    /// `%EMAIL [string]`. The email address of this score file author.
    Email(&'a str),
    /// `#ENDIF`. Closes the if scope. See [Token::If].
    EndIf,
    /// `#ENDRANDOM`. Closes the random scope. See [Token::Random].
    EndRandom,
    /// `#ENDSW`. Closes the random scope. See [Token::Switch].
    EndSwitch,
    /// `#ExtChr SpriteNum BMPNum startX startY endX endY [offsetX offsetY [x y]]` BM98 extended character customization.
    #[cfg(feature = "minor-command")]
    ExtChr(ExtChrEvent),
    /// `#EXT #XXXYY:...`. Defines the extended message. `XXX` is the track, `YY` is the channel.
    ExtendedMessage {
        /// The track, or measure, must start from 1. But some player may allow the 0 measure (i.e. Lunatic Rave 2).
        track: Track,
        /// The channel commonly expresses what the lane be arranged the note to.
        channel: Channel,
        /// The message to the channel, but not only object ids.
        message: &'a str,
    },
    /// `#BMP[01-ZZ] [0-255],[0-255],[0-255],[0-255] [filename]`. Defines the background image/movie object with the color (alpha, red, green and blue) which will be treated as transparent.
    ExBmp(ObjId, Argb, &'a Path),
    /// `#EXRANK[01-ZZ] [0-3]`. Defines the judgement level change object.
    ExRank(ObjId, JudgeLevel),
    /// `#EXWAV[01-ZZ] [parameter order] [pan or volume or frequency; 1-3] [filename]`. Defines the key sound object with the effect of pan, volume and frequency.
    #[cfg(feature = "minor-command")]
    ExWav {
        /// The id of the object to define.
        id: ObjId,
        /// The pan decay of the sound. Also called volume balance.
        pan: ExWavPan,
        /// The volume decay of the sound.
        volume: ExWavVolume,
        /// The pitch frequency of the sound.
        frequency: Option<ExWavFrequency>,
        /// The relative file path of the sound.
        path: &'a Path,
    },
    /// `#GENRE [string]`. Defines the genre of the music.
    Genre(&'a str),
    /// `#IF [u32]`. Starts an if scope when the integer equals to the generated random number. This must be placed in a random scope. See also [`Token::Random`].
    If(BigUint),
    /// `#LNMODE [1:LN, 2:CN, 3:HCN]` Explicitly specify LN type for this chart.
    LnMode(LnModeType),
    /// `#LNOBJ [01-ZZ]`. Declares the object as the end of an LN. The preceding object of the declared will be treated as the beginning of an LN.
    LnObj(ObjId),
    /// `#LNTYPE 1`. Declares the LN notation as the RDM type.
    LnTypeRdm,
    /// `#LNTYPE 2`. Declares the LN notation as the MGQ type.
    LnTypeMgq,
    /// `#MAKER [string]`. Defines the author name of the score.
    Maker(&'a str),
    /// `#MATERIALSBMP [filename]` Material BMP extension.
    /// Deprecated.
    #[cfg(feature = "minor-command")]
    MaterialsBmp(&'a Path),
    /// `#MATERIALSWAV [filename]` Material WAV extension.
    /// Deprecated.
    #[cfg(feature = "minor-command")]
    MaterialsWav(&'a Path),
    /// `#XXXYY:ZZ...`. Defines the message which places the object onto the score. `XXX` is the track, `YY` is the channel, and `ZZ...` is the object id sequence.
    Message {
        /// The track, or measure, must start from 1. But some player may allow the 0 measure (i.e. Lunatic Rave 2).
        track: Track,
        /// The channel commonly expresses what the lane be arranged the note to.
        channel: Channel,
        /// The message to the channel.
        message: Cow<'a, str>,
    },
    /// `#MIDIFILE [filename]`. Defines the MIDI file as the BGM. *Deprecated*
    #[cfg(feature = "minor-command")]
    MidiFile(&'a Path),
    /// `#MOVIE [filename]` DXEmu extension, defines global video file.
    /// - Video starts from #000.
    /// - Priority rules:
    ///   - If #xxx04 is an image file (BMP, PNG, etc.), #MOVIE has priority.
    ///   - If both #xxx04 and #MOVIE are video files, #xxx04 has priority.
    /// - No loop, stays on last frame after playback.
    /// - Audio track in video is not played.
    Movie(&'a Path),
    /// Non-empty lines that not starts in `'#'` in bms file.
    NotACommand(&'a str),
    /// `#OCT/FP`. Declares the score as the octave mode.
    #[cfg(feature = "minor-command")]
    OctFp,
    /// `#OPTION [string]`. Defines the play option of the score. Some players interpret and apply the preferences.
    Option(&'a str),
    /// `#PATH_WAV [string]`. Defines the root path of [`Token::Wav`] paths. This should be used only for tests.
    PathWav(&'a Path),
    /// `#PLAYER [1-4]`. Defines the play style of the score.
    Player(PlayerMode),
    /// `#PLAYLEVEL [integer]`. Defines the difficulty level of the score. This can be used on music select view.
    PlayLevel(u8),
    /// `#POORBGA [0-2]`. Defines the display mode of the POOR BGA.
    PoorBga(PoorMode),
    /// `#PREVIEW [filename]` Preview audio file for music selection.
    Preview(&'a Path),
    /// `#RANDOM [u32]`. Starts a random scope which can contain only `#IF`-`#ENDIF` scopes. The random scope must close with `#ENDRANDOM`. A random integer from 1 to the integer will be generated when parsing the score. Then if the integer of `#IF` equals to the random integer, the commands in an if scope will be parsed, otherwise all command in it will be ignored. Any command except `#IF` and `#ENDIF` must not be included in the scope, but some players allow it.
    Random(BigUint),
    /// `#RANK [0-3]`. Defines the judgement level.
    Rank(JudgeLevel),
    /// `#SCROLL[01-ZZ] [f64]`. Defines the scroll speed change object. It changes relative falling speed of notes with keeping BPM. For example, if applying `2.0`, the scroll speed will become double.
    Scroll(ObjId, Decimal),
    /// `#SEEK[01-ZZ] [f64]` Video seek extension.
    #[cfg(feature = "minor-command")]
    Seek(ObjId, Decimal),
    /// `#SETRANDOM [u32]`. Starts a random scope but the integer will be used as the generated random number. It should be used only for tests.
    SetRandom(BigUint),
    /// `#SETSWITCH [u32]`. Starts a switch scope but the integer will be used as the generated random number. It should be used only for tests.
    SetSwitch(BigUint),
    /// `#SKIP`. Escapes the current switch scope. It is often used in the end of every case scope.
    Skip,
    /// `#SPEED[01-ZZ] [f64]`. Defines the spacing change object. It changes relative spacing of notes with linear interpolation. For example, if playing score between the objects `1.0` and `2.0`, the spaces of notes will increase at the certain rate until the `2.0` object.
    Speed(ObjId, Decimal),
    /// `#STAGEFILE [filename]`. Defines the splashscreen image. It should be 640x480.
    StageFile(&'a Path),
    /// `#STOP[01-ZZ] [0-4294967295]`. Defines the stop object. The scroll will stop the beats of the integer divided by 192. A beat length depends on the current BPM. If there are other objects on same time, the stop object must be evaluated at last.
    Stop(ObjId, Decimal),
    /// `#STP xxx.yyy zzzz` bemaniaDX STOP sequence.
    #[cfg(feature = "minor-command")]
    Stp(StpEvent),
    /// `#SUBARTIST [string]`. Defines the sub-artist name of the music.
    SubArtist(&'a str),
    /// `#SUBTITLE [string]`. Defines the subtitle of the music.
    SubTitle(&'a str),
    /// `#SWBGA[01-ZZ] fr:time:line:loop:a,r,g,b pattern` Key Bind Layer Animation.
    #[cfg(feature = "minor-command")]
    SwBga(ObjId, SwBgaEvent),
    /// `#SWITCH [u32]`. Starts a switch scope which can contain only `#CASE` or `#DEF` scopes. The switch scope must close with `#ENDSW`. A random integer from 1 to the integer will be generated when parsing the score. Then if the integer of `#CASE` equals to the random integer, the commands in a case scope will be parsed, otherwise all command in it will be ignored. Any command except `#CASE` and `#DEF` must not be included in the scope, but some players allow it.
    Switch(BigUint),
    /// `#TEXT[01-ZZ] string`. Defines the text object.
    Text(ObjId, &'a str),
    /// `#TITLE [string]`. Defines the title of the music.
    Title(&'a str),
    /// `#TOTAL [f64]`. Defines the total gauge percentage when all notes is got as PERFECT.
    Total(Decimal),
    /// Unknown Part. Includes all the line that not be parsed.
    UnknownCommand(&'a str),
    /// `%URL [string]`. The url of this score file.
    Url(&'a str),
    /// `#VIDEOCOLORS [u8]` Video color depth, default 16Bit.
    #[cfg(feature = "minor-command")]
    VideoColors(u8),
    /// `#VIDEODLY [f64]` Video delay extension.
    #[cfg(feature = "minor-command")]
    VideoDly(Decimal),
    /// `#VIDEOFILE [filename]` / `#MOVIE [filename]`. Defines the background movie file. The audio track in the movie file should not be played. The play should start from the track `000`.
    VideoFile(&'a Path),
    /// `#VIDEOF/S [f64]` Video file frame rate.
    #[cfg(feature = "minor-command")]
    VideoFs(Decimal),
    /// `#VOLWAV [0-255]`. Defines the relative volume percentage of the sound in the score.
    VolWav(Volume),
    /// `#WAV[01-ZZ] [filename]`. Defines the key sound object. When same id multiple objects ring at same time, it must be played only one. The file specified may be not only WAV format, and also OGG, MP3 and others.
    Wav(ObjId, &'a Path),
    /// `#WAVCMD [param] [wav-index] [value]` MacBeat extension, pseudo-MOD effect.
    #[cfg(feature = "minor-command")]
    WavCmd(WavCmdEvent),
}

impl<'a> Token<'a> {
    pub(crate) fn parse(
        c: &mut Cursor<'a>,
        channel_parser: impl Fn(&str) -> Option<Channel>,
    ) -> Result<Self> {
        loop {
            let command = c
                .next_token()
                .ok_or_else(|| c.make_err_expected_token("command"))?;

            break Ok(match command.to_uppercase().as_str() {
                // Part: Normal
                "#PLAYER" => Self::Player(PlayerMode::from(c)?),
                "#GENRE" => Self::Genre(c.next_line_remaining()),
                "#TITLE" => Self::Title(c.next_line_remaining()),
                "#SUBTITLE" => Self::SubTitle(c.next_line_remaining()),
                "#ARTIST" => Self::Artist(c.next_line_remaining()),
                "#SUBARTIST" => Self::SubArtist(c.next_line_remaining()),
                "#DIFFICULTY" => Self::Difficulty(
                    c.next_token()
                        .ok_or_else(|| c.make_err_expected_token("difficulty"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?,
                ),
                "#STAEGFILE" => {
                    let file_name = c.next_line_remaining();
                    if file_name.is_empty() {
                        return Err(c.make_err_expected_token("stage filename"));
                    }
                    Self::StageFile(Path::new(file_name))
                }
                "#BANNER" => {
                    let file_name = c.next_line_remaining();
                    if file_name.is_empty() {
                        return Err(c.make_err_expected_token("banner filename"));
                    }
                    Self::Banner(Path::new(file_name))
                }
                "#BACKBMP" => {
                    let file_name = c.next_line_remaining();
                    if file_name.is_empty() {
                        return Err(c.make_err_expected_token("backbmp filename"));
                    }
                    Self::BackBmp(Path::new(file_name))
                }
                "#TOTAL" => {
                    let s = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("gauge increase rate"))?;
                    let v = Decimal::from_fraction(
                        GenericFraction::from_str(s)
                            .map_err(|_| c.make_err_expected_token("decimal"))?,
                    );
                    Self::Total(v)
                }
                "#BPM" => {
                    let s = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("bpm"))?;
                    let v = Decimal::from_fraction(
                        GenericFraction::from_str(s)
                            .map_err(|_| c.make_err_expected_token("decimal"))?,
                    );
                    Self::Bpm(v)
                }
                "#PLAYLEVEL" => Self::PlayLevel(
                    c.next_token()
                        .ok_or_else(|| c.make_err_expected_token("play level"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?,
                ),
                "#RANK" => Self::Rank(JudgeLevel::try_read(c)?),
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
                "#STAGEFILE" => {
                    let file_name = c.next_line_remaining();
                    if file_name.is_empty() {
                        return Err(c.make_err_expected_token("splashscreen image filename"));
                    }
                    Self::StageFile(Path::new(file_name))
                }
                "#VOLWAV" => {
                    let volume = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("volume"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    Self::VolWav(Volume {
                        relative_percent: volume,
                    })
                }
                "#BASE" => {
                    let base = c.next_line_remaining();
                    if base != "62" {
                        eprintln!("unknown base declared: {base:?}");
                        continue;
                    }
                    Self::Base62
                }
                "#COMMENT" => {
                    let comment = c.next_line_remaining();
                    Self::Comment(comment)
                }
                "#EMAIL" | "%EMAIL" => Self::Email(c.next_line_remaining()),
                "#URL" | "%URL" => Self::Url(c.next_line_remaining()),
                #[cfg(feature = "minor-command")]
                "#OCT/FP" => Self::OctFp,
                "#OPTION" => Self::Option(c.next_line_remaining()),
                "#PATH_WAV" => {
                    let file_name = c.next_line_remaining();
                    if file_name.is_empty() {
                        return Err(c.make_err_expected_token("wav root path"));
                    }
                    Self::PathWav(Path::new(file_name))
                }
                "#MAKER" => Self::Maker(c.next_line_remaining()),
                #[cfg(feature = "minor-command")]
                "#MIDIFILE" => {
                    let file_name = c.next_line_remaining();
                    if file_name.is_empty() {
                        return Err(c.make_err_expected_token("midi filename"));
                    }
                    Self::MidiFile(Path::new(file_name))
                }
                "#POORBGA" => Self::PoorBga(PoorMode::from(c)?),
                "#VIDEOFILE" => {
                    let file_name = c.next_line_remaining();
                    if file_name.is_empty() {
                        return Err(c.make_err_expected_token("video filename"));
                    }
                    Self::VideoFile(Path::new(file_name))
                }
                // Part: Command with lane and arg
                // Place ahead of WAV to avoid being parsed as WAV.
                #[cfg(feature = "minor-command")]
                wavcmd if wavcmd.starts_with("#WAVCMD") => {
                    let param = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("wavcmd param (00/01/02)"))?;
                    let param = match param {
                        "00" => WavCmdParam::Pitch,
                        "01" => WavCmdParam::Volume,
                        "02" => WavCmdParam::Time,
                        _ => return Err(c.make_err_expected_token("wavcmd param 00/01/02")),
                    };
                    let wav_index = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("wavcmd wav-index"))?;
                    let wav_index = ObjId::try_load(wav_index, c)?;
                    let value = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("wavcmd value"))?;
                    let value: u32 = value
                        .parse()
                        .map_err(|_| c.make_err_expected_token("wavcmd value u32"))?;
                    // Validity check
                    match param {
                        WavCmdParam::Pitch if !(0..=127).contains(&value) => {
                            return Err(c.make_err_expected_token("pitch 0-127"));
                        }
                        WavCmdParam::Time => { /* 0 means original length, less than 50ms is unreliable */
                        }
                        _ => {}
                    }
                    Self::WavCmd(WavCmdEvent {
                        param,
                        wav_index,
                        value,
                    })
                }
                wav if wav.starts_with("#WAV") => {
                    let id = command.trim_start_matches("#WAV");
                    let str = c.next_line_remaining();
                    if str.is_empty() {
                        return Err(c.make_err_expected_token("key audio filename"));
                    }
                    let filename = Path::new(str);
                    Self::Wav(ObjId::try_load(id, c)?, filename)
                }
                bmp if bmp.starts_with("#BMP") => {
                    let id = command.trim_start_matches("#BMP");
                    let str = c.next_line_remaining();
                    if str.is_empty() {
                        return Err(c.make_err_expected_token("key audio filename"));
                    }
                    let filename = Path::new(str);
                    if id == "00" {
                        Self::Bmp(None, filename)
                    } else {
                        Self::Bmp(Some(ObjId::try_load(id, c)?), filename)
                    }
                }
                bpm if bpm.starts_with("#BPM") => {
                    let id = command.trim_start_matches("#BPM");
                    let s_bpm = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("bpm"))?;
                    let v = Decimal::from_fraction(
                        GenericFraction::from_str(s_bpm)
                            .map_err(|_| c.make_err_expected_token("decimal"))?,
                    );
                    Self::BpmChange(ObjId::try_load(id, c)?, v)
                }
                stop if stop.starts_with("#STOP") => {
                    let id = command.trim_start_matches("#STOP");
                    let s_stop = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("stop beats"))?;
                    let v = Decimal::from_fraction(
                        GenericFraction::from_str(s_stop)
                            .map_err(|_| c.make_err_expected_token("decimal"))?,
                    );
                    Self::Stop(ObjId::try_load(id, c)?, v)
                }
                scroll if scroll.starts_with("#SCROLL") => {
                    let id = command.trim_start_matches("#SCROLL");
                    let s_scroll = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("scroll"))?;
                    let v = Decimal::from_fraction(
                        GenericFraction::from_str(s_scroll)
                            .map_err(|_| c.make_err_expected_token("decimal"))?,
                    );
                    Self::Scroll(ObjId::try_load(id, c)?, v)
                }
                speed if speed.starts_with("#SPEED") => {
                    let id = command.trim_start_matches("#SPEED");
                    let s_speed = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("spacing factor"))?;
                    let v = Decimal::from_fraction(
                        GenericFraction::from_str(s_speed)
                            .map_err(|_| c.make_err_expected_token("decimal"))?,
                    );
                    Self::Speed(ObjId::try_load(id, c)?, v)
                }
                exbmp if exbmp.starts_with("#EXBMP") => {
                    let id = exbmp.trim_start_matches("#EXBMP");
                    let argb = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("argb"))?;
                    let filename = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("filename"))?;

                    let parts: Vec<&str> = argb.split(',').collect();
                    if parts.len() != 4 {
                        return Err(c.make_err_expected_token("expected 4 comma-separated values"));
                    }
                    let alpha = parts[0]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("invalid alpha value"))?;
                    let red = parts[1]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("invalid red value"))?;
                    let green = parts[2]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("invalid green value"))?;
                    let blue = parts[3]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("invalid blue value"))?;

                    Self::ExBmp(
                        ObjId::try_load(id, c)?,
                        Argb {
                            alpha,
                            red,
                            green,
                            blue,
                        },
                        Path::new(filename),
                    )
                }
                exrank if exrank.starts_with("#EXRANK") => {
                    let id = exrank.trim_start_matches("#EXRANK");
                    let judge_level = JudgeLevel::try_read(c)?;
                    Self::ExRank(ObjId::try_load(id, c)?, judge_level)
                }
                #[cfg(feature = "minor-command")]
                exwav if exwav.starts_with("#EXWAV") => {
                    let id = exwav.trim_start_matches("#EXWAV");
                    let pvf_params = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("param1"))?;
                    let mut pan = None;
                    let mut volume = None;
                    let mut frequency = None;
                    for param in pvf_params.bytes() {
                        match param {
                            b'p' => {
                                let pan_value: i64 = c
                                    .next_token()
                                    .ok_or_else(|| c.make_err_expected_token("pan"))?
                                    .parse()
                                    .map_err(|_| c.make_err_expected_token("integer"))?;
                                pan = Some(ExWavPan::try_from(pan_value).map_err(|_| {
                                    c.make_err_expected_token(
                                        "pan value out of range [-10000, 10000]",
                                    )
                                })?)
                            }
                            b'v' => {
                                let volume_value: i64 = c
                                    .next_token()
                                    .ok_or_else(|| c.make_err_expected_token("volume"))?
                                    .parse()
                                    .map_err(|_| c.make_err_expected_token("integer"))?;
                                volume =
                                    Some(ExWavVolume::try_from(volume_value).map_err(|_| {
                                        c.make_err_expected_token(
                                            "volume value out of range [-10000, 0]",
                                        )
                                    })?)
                            }
                            b'f' => {
                                let frequency_value: u64 = c
                                    .next_token()
                                    .ok_or_else(|| c.make_err_expected_token("frequency"))?
                                    .parse()
                                    .map_err(|_| c.make_err_expected_token("integer"))?;
                                frequency = Some(
                                    ExWavFrequency::try_from(frequency_value).map_err(|_| {
                                        c.make_err_expected_token(
                                            "frequency value out of range [100, 100000]",
                                        )
                                    })?,
                                )
                            }
                            _ => return Err(c.make_err_expected_token("expected p, v or f")),
                        }
                    }
                    let file_name = c.next_line_remaining();
                    if file_name.is_empty() {
                        return Err(c.make_err_expected_token("filename"));
                    }
                    Self::ExWav {
                        id: ObjId::try_load(id, c)?,
                        pan: pan.unwrap_or_default(),
                        volume: volume.unwrap_or_default(),
                        frequency,
                        path: Path::new(file_name),
                    }
                }
                text if text.starts_with("#TEXT") => {
                    let id = text.trim_start_matches("#TEXT");
                    let content = c.next_line_remaining();
                    Self::Text(ObjId::try_load(id, c)?, content)
                }
                #[cfg(feature = "minor-command")]
                atbga if atbga.starts_with("#@BGA") => {
                    let id = atbga.trim_start_matches("#@BGA");
                    let source_bmp = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("source bmp"))?;
                    let sx = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("sx"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    let sy = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("sy"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    let w = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("w"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    let h = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("h"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    let dx = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("dx"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    let dy = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("dy"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    Self::AtBga {
                        id: ObjId::try_load(id, c)?,
                        source_bmp: ObjId::try_load(source_bmp, c)?,
                        trim_top_left: (sx, sy),
                        trim_size: (w, h),
                        draw_point: (dx, dy),
                    }
                }
                #[cfg(feature = "minor-command")]
                bga if bga.starts_with("#BGA") && !bga.starts_with("#BGAPOOR") => {
                    let id = bga.trim_start_matches("#BGA");
                    // Cannot use next_line_remaining here because the remaining args
                    let source_bmp = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("source bmp"))?;
                    let x1 = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("x1"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    let y1 = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("y1"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    let x2 = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("x2"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    let y2 = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("y2"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    let dx = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("dx"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    let dy = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("dy"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer"))?;
                    Self::Bga {
                        id: ObjId::try_load(id, c)?,
                        source_bmp: ObjId::try_load(source_bmp, c)?,
                        trim_top_left: (x1, y1),
                        trim_bottom_right: (x2, y2),
                        draw_point: (dx, dy),
                    }
                }
                changeoption if changeoption.starts_with("#CHANGEOPTION") => {
                    let id = changeoption.trim_start_matches("#CHANGEOPTION");
                    let option = c.next_line_remaining();
                    Self::ChangeOption(ObjId::try_load(id, c)?, option)
                }
                lnobj if lnobj.starts_with("#LNOBJ") => {
                    let id = lnobj.trim_start_matches("#LNOBJ");
                    Self::LnObj(ObjId::try_load(id, c)?)
                }
                message
                    if message.starts_with('#')
                        && message.chars().nth(6) == Some(':')
                        && 8 <= message.len() =>
                {
                    let track = command[1..4]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("[000-999]"))?;
                    let channel = &command[4..6];

                    let message = &command[7..];
                    Self::Message {
                        track: Track(track),
                        channel: channel_parser(channel)
                            .ok_or_else(|| c.make_err_unknown_channel(channel.to_string()))?,
                        message: Cow::Borrowed(message),
                    }
                }
                // New command parsing
                #[cfg(feature = "minor-command")]
                extchr if extchr.to_uppercase().starts_with("#EXTCHR") => {
                    // Allow multiple spaces between parameters
                    let mut params = c.next_line_remaining().split_whitespace();
                    let sprite_num = params
                        .next()
                        .ok_or_else(|| c.make_err_expected_token("sprite_num"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("sprite_num i32"))?;
                    let bmp_num = params
                        .next()
                        .ok_or_else(|| c.make_err_expected_token("bmp_num"))?;
                    // BMPNum supports hexadecimal (e.g. 09/FF), also supports -1/-257, etc.
                    let bmp_num = if let Some(stripped) = bmp_num.strip_prefix("-") {
                        -stripped
                            .parse::<i32>()
                            .map_err(|_| c.make_err_expected_token("bmp_num i32"))?
                    } else if bmp_num.starts_with("0x")
                        || bmp_num.chars().all(|c| c.is_ascii_hexdigit())
                    {
                        i32::from_str_radix(bmp_num, 16)
                            .unwrap_or_else(|_| bmp_num.parse().unwrap_or(0))
                    } else {
                        bmp_num
                            .parse()
                            .map_err(|_| c.make_err_expected_token("bmp_num i32/hex"))?
                    };
                    let start_x = params
                        .next()
                        .ok_or_else(|| c.make_err_expected_token("start_x"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("start_x i32"))?;
                    let start_y = params
                        .next()
                        .ok_or_else(|| c.make_err_expected_token("start_y"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("start_y i32"))?;
                    let end_x = params
                        .next()
                        .ok_or_else(|| c.make_err_expected_token("end_x"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("end_x i32"))?;
                    let end_y = params
                        .next()
                        .ok_or_else(|| c.make_err_expected_token("end_y"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("end_y i32"))?;
                    // offsetX/offsetY are optional
                    let offset_x = params.next().and_then(|v| v.parse().ok());
                    let offset_y = params.next().and_then(|v| v.parse().ok());
                    // x/y are optional, only present if offset exists
                    let abs_x = params.next().and_then(|v| v.parse().ok());
                    let abs_y = params.next().and_then(|v| v.parse().ok());
                    Self::ExtChr(ExtChrEvent {
                        sprite_num,
                        bmp_num,
                        start_x,
                        start_y,
                        end_x,
                        end_y,
                        offset_x,
                        offset_y,
                        abs_x,
                        abs_y,
                    })
                }
                ext_message if ext_message.starts_with("#EXT") => {
                    let message = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("message definition"))?;
                    if !(message.starts_with('#')
                        && message.chars().nth(6) == Some(':')
                        && 8 <= message.len())
                    {
                        eprintln!("unknown #EXT format: {message:?}");
                        continue;
                    }

                    let track = message[1..4]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("[000-999]"))?;
                    let channel = &message[4..6];
                    let message = &message[7..];
                    Self::ExtendedMessage {
                        track: Track(track),
                        channel: channel_parser(channel)
                            .ok_or_else(|| c.make_err_unknown_channel(channel.to_string()))?,
                        message,
                    }
                }
                #[cfg(feature = "minor-command")]
                charfile if charfile.starts_with("#CHARFILE") => {
                    let path = c
                        .next_token()
                        .map(Path::new)
                        .ok_or_else(|| c.make_err_expected_token("charfile filename"))?;
                    Self::CharFile(path)
                }
                song if song.starts_with("#SONG") => {
                    let id = song.trim_start_matches("#SONG");
                    let content = c.next_line_remaining();
                    Self::Text(ObjId::try_load(id, c)?, content)
                }
                exbpm if exbpm.starts_with("#EXBPM") => {
                    let id = exbpm.trim_start_matches("#EXBPM");
                    let v = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("exbpm value"))?;
                    let v = Decimal::from_fraction(
                        GenericFraction::from_str(v)
                            .map_err(|_| c.make_err_expected_token("f64"))?,
                    );
                    Self::BpmChange(ObjId::try_load(id, c)?, v)
                }
                #[cfg(feature = "minor-command")]
                "#BASEBPM" => {
                    let v = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("basebpm value"))?;
                    let v = Decimal::from_fraction(
                        GenericFraction::<BigUint>::from_str(v)
                            .map_err(|_| c.make_err_expected_token("f64"))?,
                    );
                    Self::BaseBpm(v)
                }
                #[cfg(feature = "minor-command")]
                stp if stp.starts_with("#STP") => {
                    // Parse xxx.yyy zzzz
                    let xy = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("stp format [xxx.yyy] zzzz"))?;
                    let ms = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("stp format xxx.yyy [zzzz]"))?;
                    let ms: u32 = ms
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .parse()
                        .map_err(|_| c.make_err_expected_token("stp ms (u32)"))?;
                    let (measure, pos) = xy.split_once('.').unwrap_or((xy, "000"));
                    if measure.len() != 3 || pos.len() != 3 {
                        return Err(c.make_err_expected_token("stp measure/pos must be 3 digits"));
                    }
                    let measure: u16 = measure
                        .parse()
                        .map_err(|_| c.make_err_expected_token("stp measure u16"))?;
                    let pos: u16 = pos
                        .parse()
                        .map_err(|_| c.make_err_expected_token("stp pos u16"))?;
                    let time =
                        crate::bms::command::time::ObjTime::new(measure as u64, pos as u64, 1000);
                    let duration = Duration::from_millis(ms as u64);
                    Self::Stp(StpEvent { time, duration })
                }
                #[cfg(feature = "minor-command")]
                "#CDDA" => {
                    let v = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("cdda value"))?;
                    let v = BigUint::parse_bytes(v.as_bytes(), 10)
                        .ok_or_else(|| c.make_err_expected_token("BigUint"))?;
                    Self::Cdda(v)
                }
                #[cfg(feature = "minor-command")]
                swbga if swbga.starts_with("#SWBGA") => {
                    let id = swbga.trim_start_matches("#SWBGA");
                    // Parse fr:time:line:loop:a,r,g,b pattern
                    let params = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("swbga params"))?;
                    let mut parts = params.split(':');
                    let frame_rate = parts
                        .next()
                        .ok_or_else(|| c.make_err_expected_token("swbga frame_rate"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("swbga frame_rate u32"))?;
                    let total_time = parts
                        .next()
                        .ok_or_else(|| c.make_err_expected_token("swbga total_time"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("swbga total_time u32"))?;
                    let line = parts
                        .next()
                        .ok_or_else(|| c.make_err_expected_token("swbga line"))?
                        .parse()
                        .map_err(|_| c.make_err_expected_token("swbga line u8"))?;
                    let loop_mode = parts
                        .next()
                        .ok_or_else(|| c.make_err_expected_token("swbga loop"))?
                        .parse::<u8>()
                        .map_err(|_| c.make_err_expected_token("swbga loop 0/1"))?;
                    let loop_mode = match loop_mode {
                        0 => false,
                        1 => true,
                        _ => return Err(c.make_err_expected_token("swbga loop 0/1")),
                    };
                    let argb_str = parts
                        .next()
                        .ok_or_else(|| c.make_err_expected_token("swbga argb"))?;
                    let argb_parts: Vec<&str> = argb_str.split(',').collect();
                    if argb_parts.len() != 4 {
                        return Err(c.make_err_expected_token("swbga argb 4 values"));
                    }
                    let alpha = argb_parts[0]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("swbga argb alpha"))?;
                    let red = argb_parts[1]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("swbga argb red"))?;
                    let green = argb_parts[2]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("swbga argb green"))?;
                    let blue = argb_parts[3]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("swbga argb blue"))?;
                    let pattern = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("swbga pattern"))?
                        .to_string();
                    Self::SwBga(
                        ObjId::try_load(id, c)?,
                        SwBgaEvent {
                            frame_rate,
                            total_time,
                            line,
                            loop_mode,
                            argb: Argb {
                                alpha,
                                red,
                                green,
                                blue,
                            },
                            pattern,
                        },
                    )
                }
                #[cfg(feature = "minor-command")]
                argb if argb.starts_with("#ARGB") => {
                    let id = argb.trim_start_matches("#ARGB");
                    let argb_str = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("argb value"))?;
                    let parts: Vec<&str> = argb_str.split(',').collect();
                    if parts.len() != 4 {
                        return Err(c.make_err_expected_token("expected 4 comma-separated values"));
                    }
                    let alpha = parts[0]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("invalid alpha value"))?;
                    let red = parts[1]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("invalid red value"))?;
                    let green = parts[2]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("invalid green value"))?;
                    let blue = parts[3]
                        .parse()
                        .map_err(|_| c.make_err_expected_token("invalid blue value"))?;
                    Self::Argb(
                        ObjId::try_load(id, c)?,
                        Argb {
                            alpha,
                            red,
                            green,
                            blue,
                        },
                    )
                }
                #[cfg(feature = "minor-command")]
                videofs if videofs.starts_with("#VIDEOF/S") => {
                    let v = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("videofs value"))?;
                    let v = Decimal::from_fraction(
                        GenericFraction::<BigUint>::from_str(v)
                            .map_err(|_| c.make_err_expected_token("f64"))?,
                    );
                    Self::VideoFs(v)
                }
                #[cfg(feature = "minor-command")]
                videocolors if videocolors.starts_with("#VIDEOCOLORS") => {
                    let v = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("videocolors value"))?;
                    let v = v
                        .parse::<u8>()
                        .map_err(|_| c.make_err_expected_token("u8"))?;
                    Self::VideoColors(v)
                }
                #[cfg(feature = "minor-command")]
                videodly if videodly.starts_with("#VIDEODLY") => {
                    let v = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("videodly value"))?;
                    let v = Decimal::from_fraction(
                        GenericFraction::<BigUint>::from_str(v)
                            .map_err(|_| c.make_err_expected_token("f64"))?,
                    );
                    Self::VideoDly(v)
                }
                #[cfg(feature = "minor-command")]
                seek if seek.starts_with("#SEEK") => {
                    let id = seek.trim_start_matches("#SEEK");
                    let v = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("seek value"))?;
                    let v = Decimal::from_fraction(
                        GenericFraction::<BigUint>::from_str(v)
                            .map_err(|_| c.make_err_expected_token("f64"))?,
                    );
                    Self::Seek(ObjId::try_load(id, c)?, v)
                }
                #[cfg(feature = "minor-command")]
                materialswav if materialswav.starts_with("#MATERIALSWAV") => {
                    let path = c
                        .next_token()
                        .map(Path::new)
                        .ok_or_else(|| c.make_err_expected_token("materialswav filename"))?;
                    Self::MaterialsWav(path)
                }
                #[cfg(feature = "minor-command")]
                materialsbmp if materialsbmp.starts_with("#MATERIALSBMP") => {
                    let path = c
                        .next_token()
                        .map(Path::new)
                        .ok_or_else(|| c.make_err_expected_token("materialsbmp filename"))?;
                    Self::MaterialsBmp(path)
                }
                #[cfg(feature = "minor-command")]
                divideprop if divideprop.starts_with("#DIVIDEPROP") => {
                    let s = c.next_line_remaining();
                    Self::DivideProp(s)
                }
                charset if charset.starts_with("#CHARSET") => {
                    let s = c.next_line_remaining();
                    Self::Charset(s)
                }
                defexrank if defexrank.starts_with("#DEFEXRANK") => {
                    let value = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("defexrank value"))?;
                    let value = value
                        .parse()
                        .map_err(|_| c.make_err_expected_token("u64"))?;
                    Self::DefExRank(value)
                }
                preview if preview.starts_with("#PREVIEW") => {
                    let path = c
                        .next_token()
                        .map(Path::new)
                        .ok_or_else(|| c.make_err_expected_token("preview filename"))?;
                    Self::Preview(path)
                }
                lnmode if lnmode.starts_with("#LNMODE") => {
                    let mode = c
                        .next_token()
                        .ok_or_else(|| c.make_err_expected_token("lnmode value"))?;
                    let mode: u8 = mode
                        .parse()
                        .map_err(|_| c.make_err_expected_token("integer 1-3"))?;
                    let mode = match mode {
                        1 => LnModeType::Ln,
                        2 => LnModeType::Cn,
                        3 => LnModeType::Hcn,
                        _ => return Err(c.make_err_expected_token("lnmode 1-3")),
                    };
                    Self::LnMode(mode)
                }
                movie if movie.starts_with("#MOVIE") => {
                    let path = c
                        .next_token()
                        .map(Path::new)
                        .ok_or_else(|| c.make_err_expected_token("movie filename"))?;
                    Self::Movie(path)
                }
                // Unknown command & Not a command
                command if command.starts_with('#') => Self::UnknownCommand(c.next_line_entire()),
                _not_command => Self::NotACommand(c.next_line_entire()),
            });
        }
    }

    pub(crate) fn make_id_uppercase(&mut self) {
        use Token::*;
        match self {
            #[cfg(feature = "minor-command")]
            AtBga { id, source_bmp, .. } => {
                id.make_uppercase();
                source_bmp.make_uppercase();
            }
            #[cfg(feature = "minor-command")]
            Bga { id, source_bmp, .. } => {
                id.make_uppercase();
                source_bmp.make_uppercase();
            }
            Bmp(Some(id), _) => {
                id.make_uppercase();
            }
            BpmChange(id, _) => {
                id.make_uppercase();
            }
            ChangeOption(id, _) => {
                id.make_uppercase();
            }
            ExBmp(id, _, _) => {
                id.make_uppercase();
            }
            ExRank(id, _) => {
                id.make_uppercase();
            }
            #[cfg(feature = "minor-command")]
            ExWav { id, .. } => {
                id.make_uppercase();
            }
            LnObj(id) => {
                id.make_uppercase();
            }
            Message { message, .. } => {
                if message.chars().any(|ch| ch.is_ascii_lowercase()) {
                    message.to_mut().make_ascii_uppercase();
                }
            }
            Scroll(id, _) => {
                id.make_uppercase();
            }
            Speed(id, _) => {
                id.make_uppercase();
            }
            Stop(id, _) => {
                id.make_uppercase();
            }
            Text(id, _) => {
                id.make_uppercase();
            }
            Wav(id, _) => {
                id.make_uppercase();
            }
            _ => {}
        }
    }

    /// Checks if a token is a control flow token.
    pub fn is_control_flow_token(&self) -> bool {
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

#[cfg(test)]
mod tests {
    use crate::bms::command::channel::read_channel_beat;
    #[cfg(feature = "minor-command")]
    use crate::bms::command::time::Track;

    use super::*;

    fn parse_token(input: &str) -> Token {
        let mut cursor = Cursor::new(input);
        Token::parse(&mut cursor, read_channel_beat).unwrap()
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
        assert_eq!(pan.value(), 0);
        assert_eq!(volume.value(), 0);
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
    fn test_changeoption() {
        let Token::ChangeOption(id, opt) = parse_token("#CHANGEOPTION01 opt") else {
            panic!("Not ChangeOption");
        };
        assert_eq!(format!("{id:?}"), "ObjId(\"01\")");
        assert_eq!(opt, "opt");
    }

    #[test]
    fn test_lnobj() {
        let Token::LnObj(id) = parse_token("#LNOBJ01") else {
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
        assert_eq!(stp.time.track, Track(1));
        assert_eq!(stp.time.numerator, 500);
        assert_eq!(stp.time.denominator, 1000);
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
}
