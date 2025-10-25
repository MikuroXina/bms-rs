//! This module introduces struct [`MusicInfo`], which manages the music information of BMS scores.

use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// This aggregate manages the music information of BMS scores.
pub struct MusicInfo {
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
    /// Preview Music. Defines the preview audio file for music selection.
    /// This file is played when hovering over the song in the music select screen.
    pub preview_music: Option<PathBuf>,
}
