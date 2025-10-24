use std::path::PathBuf;

use crate::bms::prelude::*;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Sprites {
    /// The path of background image, which is shown while playing the score.
    /// This image is displayed behind the gameplay area.
    pub back_bmp: Option<PathBuf>,
    /// The path of splash screen image, which is shown before playing the score.
    /// This image is displayed during the loading screen.
    pub stage_file: Option<PathBuf>,
    /// The path of banner image.
    /// This image is used in music selection screens.
    pub banner: Option<PathBuf>,
    /// Extended-character events. `#ExtChr`

    pub extchr_events: Vec<ExtChrEvent>,
    /// Character file path. `#CHARFILE`

    pub char_file: Option<PathBuf>,
}
