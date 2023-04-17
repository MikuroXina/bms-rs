//! Header information from parsed BMS file.

use std::{collections::HashMap, fmt::Debug, path::PathBuf};

use super::{ParseError, Result};
use crate::lex::{command::*, token::Token};

/// A notation type about LN in the score. But you don't have to take care of how the notes are actually placed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LnType {
    /// The RDM type.
    Rdm,
    /// The MGQ type.
    Mgq,
}

impl Default for LnType {
    fn default() -> Self {
        Self::Rdm
    }
}

/// A background image/video data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Bmp {
    /// The path to the image/video file. This is relative path from the BMS file.
    pub file: PathBuf,
    /// The color which should to be treated as transparent. It should be used only if `file` is an image.
    pub transparent_color: Argb,
}

/// A header parsed from [`TokenStream`](crate::lex::token::TokenStream).
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Header {
    /// The play style of the score.
    pub player: Option<PlayerMode>,
    /// The genre of the score.
    pub genre: Option<String>,
    /// The title of the score.
    pub title: Option<String>,
    /// The subtitle of the score.
    pub subtitle: Option<String>,
    /// The artist of the music in the score.
    pub artist: Option<String>,
    /// The co-artist of the music in the score.
    pub sub_artist: Option<String>,
    /// Who placed the notes into the score.
    pub maker: Option<String>,
    /// The text messages of the score. It may be closed with double quotes.
    pub comment: Option<Vec<String>>,
    /// The email address of the author.
    pub email: Option<String>,
    /// The url of the author.
    pub url: Option<String>,
    /// The message for overriding options of some BMS player.
    pub options: Option<Vec<String>>,
    /// The initial BPM of the score.
    pub bpm: Option<f64>,
    /// The play level of the score.
    pub play_level: Option<u8>,
    /// The judgement level of the score.
    pub rank: Option<JudgeLevel>,
    /// The difficulty of the score.
    pub difficulty: Option<u8>,
    /// The total gauge percentage when all notes is got as PERFECT.
    pub total: Option<f64>,
    /// The volume of the score.
    pub volume: Volume,
    /// The LN notation type of the score.
    pub ln_type: LnType,
    /// The display mode for background image/video.
    pub poor_bga_mode: PoorMode,
    /// The path of background image, which is shown while playing the score.
    pub back_bmp: Option<PathBuf>,
    /// The path of splash screen image, which is shown before playing the score.
    pub stage_file: Option<PathBuf>,
    /// The path of banner image.
    pub banner: Option<PathBuf>,
    /// Whether the score is the octave mode.
    pub is_octave: bool,
    /// The path of MIDI file, which is played as BGM while playing the score.
    pub midi_file: Option<PathBuf>,
    /// The path of the background video. The video should be started the playing from the section 000.
    pub video_file: Option<PathBuf>,
    /// The path to override the base path of the WAV file path.
    pub wav_path_root: Option<PathBuf>,
    /// The WAV file paths corresponding to the id of the note object.
    pub wav_files: HashMap<ObjId, PathBuf>,
    /// The path of image, which is shown when the player got POOR.
    pub poor_bmp: Option<PathBuf>,
    /// The BMP file paths corresponding to the id of the background image/video object.
    pub bmp_files: HashMap<ObjId, Bmp>,
    /// The BPMs corresponding to the id of the BPM change object.
    pub bpm_changes: HashMap<ObjId, f64>,
    /// The texts corresponding to the id of the text object.
    pub texts: HashMap<ObjId, String>,
    /// The option messages corresponding to the id of the change option object.
    pub change_options: HashMap<ObjId, String>,
    /// Stop lengths by stop object id.
    pub stops: HashMap<ObjId, u32>,
}

impl Header {
    pub(crate) fn parse(&mut self, token: &Token) -> Result<()> {
        match *token {
            Token::Artist(artist) => self.artist = Some(artist.into()),
            Token::AtBga { .. } => todo!(),
            Token::Banner(file) => self.banner = Some(file.into()),
            Token::BackBmp(bmp) => self.back_bmp = Some(bmp.into()),
            Token::Bga { .. } => todo!(),
            Token::Bmp(id, path) => {
                if id.is_none() {
                    self.poor_bmp = Some(path.into());
                    return Ok(());
                }
                let id = id.unwrap();
                if self
                    .bmp_files
                    .insert(
                        id,
                        Bmp {
                            file: path.into(),
                            transparent_color: Argb::default(),
                        },
                    )
                    .is_some()
                {
                    eprintln!(
                        "duplicated bmp definition found: {:?} {:?}",
                        id,
                        path.display()
                    );
                }
            }
            Token::Bpm(bpm) => {
                if let Ok(parsed) = bpm.parse() {
                    if 0.0 < parsed {
                        self.bpm = Some(parsed);
                    } else {
                        eprintln!("not positive bpm found: {:?}", parsed);
                    }
                } else {
                    eprintln!("not number bpm found: {:?}", bpm);
                }
            }
            Token::BpmChange(id, bpm) => {
                let parsed: f64 = bpm
                    .parse()
                    .map_err(|_| ParseError::BpmParseError(bpm.into()))?;
                if parsed <= 0.0 || !parsed.is_finite() {
                    return Err(ParseError::BpmParseError(bpm.into()));
                }
                if self.bpm_changes.insert(id, parsed).is_some() {
                    eprintln!("duplicated bpm change definition found: {:?} {:?}", id, bpm);
                }
            }
            Token::ChangeOption(id, option) => {
                if self.change_options.insert(id, option.into()).is_some() {
                    eprintln!(
                        "duplicated change option definition found: {:?} {}",
                        id, option
                    );
                }
            }
            Token::Comment(comment) => self
                .comment
                .get_or_insert_with(Vec::new)
                .push(comment.into()),
            Token::Difficulty(diff) => self.difficulty = Some(diff),
            Token::Email(email) => self.email = Some(email.into()),
            Token::ExBmp(id, transparent_color, path) => {
                if self
                    .bmp_files
                    .insert(
                        id,
                        Bmp {
                            file: path.into(),
                            transparent_color,
                        },
                    )
                    .is_some()
                {
                    eprintln!(
                        "duplicated bmp definition found: {:?} {:?}",
                        id,
                        path.display()
                    );
                }
            }
            Token::ExRank(_, _) => todo!(),
            Token::ExWav(_, _, _) => todo!(),
            Token::Genre(genre) => self.genre = Some(genre.to_owned()),
            Token::LnTypeRdm => {
                self.ln_type = LnType::Rdm;
            }
            Token::LnTypeMgq => {
                self.ln_type = LnType::Mgq;
            }
            Token::Maker(maker) => self.maker = Some(maker.into()),
            Token::MidiFile(midi_file) => self.midi_file = Some(midi_file.into()),
            Token::OctFp => self.is_octave = true,
            Token::Option(option) => self
                .options
                .get_or_insert_with(Vec::new)
                .push(option.into()),
            Token::PathWav(wav_path_root) => self.wav_path_root = Some(wav_path_root.into()),
            Token::Player(player) => self.player = Some(player),
            Token::PlayLevel(play_level) => self.play_level = Some(play_level),
            Token::PoorBga(poor_bga_mode) => self.poor_bga_mode = poor_bga_mode,
            Token::Rank(rank) => self.rank = Some(rank),
            Token::StageFile(file) => self.stage_file = Some(file.into()),
            Token::Stop(id, len) => {
                self.stops
                    .entry(id)
                    .and_modify(|current_len| *current_len += len)
                    .or_insert(len);
            }
            Token::SubArtist(sub_artist) => self.sub_artist = Some(sub_artist.into()),
            Token::SubTitle(subtitle) => self.subtitle = Some(subtitle.into()),
            Token::Text(id, text) => {
                if self.texts.insert(id, text.into()).is_some() {
                    eprintln!("duplicated text definition found: {:?} {}", id, text);
                }
            }
            Token::Title(title) => self.title = Some(title.into()),
            Token::Total(total) => {
                if let Ok(parsed) = total.parse() {
                    self.total = Some(parsed);
                } else {
                    eprintln!("not number total found: {:?}", total);
                }
            }
            Token::Url(url) => self.url = Some(url.into()),
            Token::VideoFile(video_file) => self.video_file = Some(video_file.into()),
            Token::VolWav(volume) => self.volume = volume,
            Token::Wav(id, path) => {
                if self.wav_files.insert(id, path.into()).is_some() {
                    eprintln!(
                        "duplicated wav definition found: {:?} {:?}",
                        id,
                        path.display()
                    );
                }
            }
            _ => {}
        }
        Ok(())
    }
}
