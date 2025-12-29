//! Definitions of the note object.

use crate::bms::{
    Decimal,
    command::{
        JudgeLevel, ObjId,
        channel::{Channel, NoteChannelId},
        time::{ObjTime, Track},
    },
};

use crate::bms::command::{graphics::Argb, minor_command::SwBgaEvent};

/// An object playing sound on the score.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WavObj {
    /// The time offset in the track.
    pub offset: ObjTime,
    /// The key, or lane, where the object is placed.
    pub channel_id: NoteChannelId,
    /// The `#WAVxx` id to be rung on play.
    pub wav_id: ObjId,
}

impl PartialOrd for WavObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WavObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.offset
            .cmp(&other.offset)
            .then(self.wav_id.cmp(&other.wav_id))
    }
}

impl WavObj {
    pub(crate) fn dangling() -> Self {
        Self {
            offset: ObjTime::start_of(1.into()),
            channel_id: NoteChannelId::bgm(),
            wav_id: ObjId::null(),
        }
    }
}

/// An object to change the BPM of the score.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BpmChangeObj {
    /// The time to begin the change of BPM.
    pub time: ObjTime,
    /// The BPM to be.
    pub bpm: Decimal,
}

impl PartialEq for BpmChangeObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for BpmChangeObj {}

impl PartialOrd for BpmChangeObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BpmChangeObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// An object to change its section length of the score.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SectionLenChangeObj {
    /// The target track to change.
    pub track: Track,
    /// The length to be.
    pub length: Decimal,
}

impl PartialEq for SectionLenChangeObj {
    fn eq(&self, other: &Self) -> bool {
        self.track == other.track
    }
}

impl Eq for SectionLenChangeObj {}

impl PartialOrd for SectionLenChangeObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SectionLenChangeObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.track.cmp(&other.track)
    }
}

/// An object to stop scrolling of score.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StopObj {
    /// Time to start the stop.
    pub time: ObjTime,
    /// Object duration how long stops scrolling of score.
    ///
    /// Note that the duration of stopping will not be changed by a current measure length but BPM.
    pub duration: Decimal,
}

impl PartialEq for StopObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for StopObj {}

impl PartialOrd for StopObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StopObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// An object to change the image for BGA (background animation).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BgaObj {
    /// Time to start to display the image.
    pub time: ObjTime,
    /// Identifier represents the image/video file registered in [`Header`].
    pub id: ObjId,
    /// Layer to display.
    pub layer: BgaLayer,
}

impl PartialEq for BgaObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for BgaObj {}

impl PartialOrd for BgaObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BgaObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// A layer where the image for BGA to be displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum BgaLayer {
    /// The lowest layer.
    Base,
    /// Layer which is displayed only if a player missed to play notes.
    Poor,
    /// An overlaying layer.
    Overlay,
    /// An overlaying layer.
    ///
    /// This layer is layered over [`BgaLayer::Overlay`].
    Overlay2,
}

impl BgaLayer {
    /// Convert a [`crate::bms::command::channel::Channel`] to a [`BgaLayer`].
    #[must_use]
    pub const fn from_channel(channel: Channel) -> Option<Self> {
        match channel {
            Channel::BgaBase | Channel::BgaBaseArgb | Channel::BgaBaseOpacity => Some(Self::Base),
            Channel::BgaLayer | Channel::BgaLayerArgb | Channel::BgaLayerOpacity => {
                Some(Self::Overlay)
            }
            Channel::BgaLayer2 | Channel::BgaLayer2Argb | Channel::BgaLayer2Opacity => {
                Some(Self::Overlay2)
            }
            Channel::BgaPoor | Channel::BgaPoorArgb | Channel::BgaPoorOpacity => Some(Self::Poor),
            _ => None,
        }
    }

    /// Convert a [`BgaLayer`] to a [`crate::bms::command::channel::Channel`].
    #[must_use]
    pub const fn to_channel(self) -> Channel {
        match self {
            Self::Base => Channel::BgaBase,
            Self::Overlay => Channel::BgaLayer,
            Self::Overlay2 => Channel::BgaLayer2,
            Self::Poor => Channel::BgaPoor,
        }
    }
}

/// An object to change scrolling factor of the score.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScrollingFactorObj {
    /// The time to begin the change of BPM.
    pub time: ObjTime,
    /// The scrolling factor to be.
    pub factor: Decimal,
}

impl PartialEq for ScrollingFactorObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for ScrollingFactorObj {}

impl PartialOrd for ScrollingFactorObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScrollingFactorObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// An object to change spacing factor among notes with linear interpolation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpeedObj {
    /// The time to begin the change of BPM.
    pub time: ObjTime,
    /// The spacing factor to be.
    pub factor: Decimal,
}

impl PartialEq for SpeedObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for SpeedObj {}

impl PartialOrd for SpeedObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SpeedObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// An object to change the opacity of BGA layers.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BgaOpacityObj {
    /// The time which the opacity change is on.
    pub time: ObjTime,
    /// The BGA layer to change opacity for.
    pub layer: BgaLayer,
    /// The opacity value (0x01-0xFF, where 0x01 is transparent and 0xFF is opaque).
    pub opacity: u8,
}

/// An object to change the ARGB color of BGA layers.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BgaArgbObj {
    /// The time which the ARGB change is on.
    pub time: ObjTime,
    /// The BGA layer to change ARGB color for.
    pub layer: BgaLayer,
    /// The ARGB color value (A,R,G,B each [0-255]).
    pub argb: Argb,
}

/// An object to change the volume of BGM sounds.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BgmVolumeObj {
    /// The time which the volume change is on.
    pub time: ObjTime,
    /// The volume value (0x01-0xFF, where 0x01 is minimum and 0xFF is maximum).
    pub volume: u8,
}

/// An object to change the volume of KEY sounds.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KeyVolumeObj {
    /// The time which the volume change is on.
    pub time: ObjTime,
    /// The volume value (0x01-0xFF, where 0x01 is minimum and 0xFF is maximum).
    pub volume: u8,
}

/// An object to seek video position.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SeekObj {
    /// The time which the seek event is on.
    pub time: ObjTime,
    /// The seek position value.
    pub position: Decimal,
}

/// An object to display text.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextObj {
    /// The time which the text is displayed.
    pub time: ObjTime,
    /// The text content.
    pub text: String,
}

/// An object to change judge level.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct JudgeObj {
    /// The time which the judge change is on.
    pub time: ObjTime,
    /// The judge level.
    pub judge_level: JudgeLevel,
}

/// An object to change BGA keybound.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BgaKeyboundObj {
    /// The time which the BGA keybound change is on.
    pub time: ObjTime,
    /// The BGA keybound event.
    pub event: SwBgaEvent,
}

/// An object to change option.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionObj {
    /// The time which the option change is on.
    pub time: ObjTime,
    /// The option content.
    pub option: String,
}
