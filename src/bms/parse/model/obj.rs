//! Definitions of the note object.
use crate::bms::{Decimal, command::*};

/// An object on the score.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Obj {
    /// The time offset in the track.
    pub offset: ObjTime,
    /// THe note kind of the the object.
    pub kind: NoteKind,
    /// The side of the player.
    pub side: PlayerSide,
    /// The key, or lane, where the object is placed.
    pub key: Key,
    /// The id of the object.
    pub obj: ObjId,
}

impl PartialOrd for Obj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Obj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.offset
            .cmp(&other.offset)
            .then(self.obj.cmp(&other.obj))
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum BgaLayer {
    /// The lowest layer.
    Base,
    /// Layer which is displayed only if a player missed to play notes.
    Poor,
    /// An overlaying layer.
    Overlay,
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

/// An object to change spacing factor between notes with linear interpolation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpacingFactorObj {
    /// The time to begin the change of BPM.
    pub time: ObjTime,
    /// The spacing factor to be.
    pub factor: Decimal,
}

impl PartialEq for SpacingFactorObj {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}

impl Eq for SpacingFactorObj {}

impl PartialOrd for SpacingFactorObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SpacingFactorObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

/// An extended object on the score.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExtendedMessageObj {
    /// The track which the message is on.
    pub track: Track,
    /// The channel which the message is on.
    pub channel: Channel,
    /// The extended message.
    pub message: String,
}

impl PartialEq for ExtendedMessageObj {
    fn eq(&self, other: &Self) -> bool {
        self.track == other.track
    }
}

impl Eq for ExtendedMessageObj {}

impl PartialOrd for ExtendedMessageObj {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExtendedMessageObj {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.track.cmp(&other.track)
    }
}
