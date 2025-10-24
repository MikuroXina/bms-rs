

use crate::bms::command::time::ObjTime;
use std::time::Duration;

use super::{ObjId, graphics::Argb};

/// Pan value for `#EXWAV` sound effect.
/// Range: \[-10000, 10000]. -10000 is leftmost, 10000 is rightmost.
/// Default: 0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExWavPan(i64);

impl ExWavPan {
    /// Creates a new [`ExWavPan`] value.
    /// Returns `None` if the value is out of range \[-10000, 10000].
    #[must_use]
    pub fn new(value: i64) -> Option<Self> {
        (-10000..=10000).contains(&value).then_some(Self(value))
    }

    /// Returns the underlying value.
    #[must_use]
    pub const fn value(self) -> i64 {
        self.0
    }

    /// Returns the default value (0).
    #[must_use]
    pub const fn default() -> Self {
        Self(0)
    }
}

impl TryFrom<i64> for ExWavPan {
    type Error = i64;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        Self::new(value).ok_or_else(|| value.clamp(-10000, 10000))
    }
}

/// Volume value for `#EXWAV` sound effect.
/// Range: \[-10000, 0]. -10000 is 0%, 0 is 100%.
/// Default: 0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExWavVolume(i64);

impl ExWavVolume {
    /// Creates a new [`ExWavVolume`] value.
    /// Returns `None` if the value is out of range `[-10000, 0]`.
    #[must_use]
    pub fn new(value: i64) -> Option<Self> {
        (-10000..=0).contains(&value).then_some(Self(value))
    }

    /// Returns the underlying value.
    #[must_use]
    pub const fn value(self) -> i64 {
        self.0
    }

    /// Returns the default value (0).
    #[must_use]
    pub const fn default() -> Self {
        Self(0)
    }
}

impl TryFrom<i64> for ExWavVolume {
    type Error = i64;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        Self::new(value).ok_or_else(|| value.clamp(-10000, 0))
    }
}

/// Frequency value for `#EXWAV` sound effect.
/// Range: \[100, 100000]. Unit: Hz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExWavFrequency(u64);

impl ExWavFrequency {
    const MIN_FREQUENCY: u64 = 100;
    const MAX_FREQUENCY: u64 = 100_000;

    /// Creates a new [`ExWavFrequency`] value.
    /// Returns `None` if the value is out of range [100, 100000].
    #[must_use]
    pub fn new(value: u64) -> Option<Self> {
        (Self::MIN_FREQUENCY..=Self::MAX_FREQUENCY)
            .contains(&value)
            .then_some(Self(value))
    }

    /// Returns the underlying value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for ExWavFrequency {
    type Error = u64;

    fn try_from(value: u64) -> std::result::Result<Self, Self::Error> {
        Self::new(value).ok_or_else(|| value.clamp(Self::MIN_FREQUENCY, Self::MAX_FREQUENCY))
    }
}

/// bemaniaDX type STP sequence definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StpEvent {
    /// The time of the stop.
    pub time: ObjTime,
    /// The duration of the stop.
    pub duration: Duration,
}

/// MacBeat `#WAVCMD` event.
///
/// Used for `#WAVCMD` command, represents `pitch`/`volume`/`time` adjustment for a specific WAV object.
/// - `param`: adjustment type (`pitch`/`volume`/`time`)
/// - `wav_index`: target WAV object ID
/// - `value`: adjustment value, meaning depends on param
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WavCmdEvent {
    /// Adjustment type (pitch/volume/time)
    pub param: WavCmdParam,
    /// Target WAV object ID
    pub wav_index: ObjId,
    /// Adjustment value, meaning depends on param
    pub value: u32,
}

/// SWBGA (Key Bind Layer Animation) event.
///
/// Used for `#SWBGA` command, describes key-bound BGA animation.
/// - `frame_rate`: frame interval (ms), e.g. 60FPS=17
/// - `total_time`: total animation duration (ms), 0 means while key is held
/// - line: applicable key channel (e.g. 11-19, 21-29)
/// - `loop_mode`: whether to loop (0: no loop, 1: loop)
/// - `argb`: transparent color (A,R,G,B)
/// - `pattern`: animation frame sequence (e.g. 01020304)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SwBgaEvent {
    /// Frame interval (ms), e.g. 60FPS=17.
    pub frame_rate: u32,
    /// Total animation duration (ms), 0 means while key is held.
    pub total_time: u32,
    /// Applicable key channel (e.g. 11-19, 21-29).
    pub line: u8,
    /// Whether to loop (0: no loop, 1: loop).
    pub loop_mode: bool,
    /// Transparent color (A,R,G,B).
    pub argb: Argb,
    /// Animation frame sequence (e.g. 01020304).
    pub pattern: String,
}

/// BM98 `#ExtChr` extended character customization event.
///
/// Used for `#ExtChr` command, implements custom UI element image replacement.
/// - `sprite_num`: character index to replace `[0-1023]`
/// - `bmp_num`: BMP index (hex to decimal, or `-1`/`-257`, etc.)
/// - `start_x`/`start_y`: crop start point
/// - `end_x`/`end_y`: crop end point
/// - `offset_x`/`offset_y`: offset (optional)
/// - `abs_x`/`abs_y`: absolute coordinate (optional)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExtChrEvent {
    /// Character index to replace [0-1023]
    pub sprite_num: i32,
    /// BMP index (hex to decimal, or -1/-257, etc.)
    pub bmp_num: i32,
    /// Crop start point
    pub start_x: i32,
    /// Crop start point
    pub start_y: i32,
    /// Crop end point
    pub end_x: i32,
    /// Crop end point
    pub end_y: i32,
    /// Offset (optional)
    pub offset_x: Option<i32>,
    /// Offset (optional)
    pub offset_y: Option<i32>,
    /// Absolute coordinate (optional)
    pub abs_x: Option<i32>,
    /// Absolute coordinate (optional)
    pub abs_y: Option<i32>,
}

/// WAVCMD parameter type.
///
/// - Pitch: pitch (0-127, 60 is C6)
/// - Volume: volume percent (usually 0-100)
/// - Time: playback time (ms*0.5, 0 means original length)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WavCmdParam {
    /// Pitch (0-127, 60 is C6)
    Pitch,
    /// Volume percent (0-100 is recommended. Larger than 100 value is not recommended.)
    Volume,
    /// Playback time (ms*0.5, 0 means original length)
    Time,
}

impl WavCmdParam {
    /// Converts an operation type of `#WAVCMD` into the corresponding string literal.
    #[must_use]
    pub const fn to_str(self) -> &'static str {
        match self {
            Self::Pitch => "00",
            Self::Volume => "01",
            Self::Time => "02",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exwav_pan_try_from() {
        // Valid values
        assert!(ExWavPan::try_from(0).is_ok());
        assert!(ExWavPan::try_from(10000).is_ok());
        assert!(ExWavPan::try_from(-10000).is_ok());
        assert!(ExWavPan::try_from(5000).is_ok());
        assert!(ExWavPan::try_from(-5000).is_ok());

        // Invalid values
        assert!(ExWavPan::try_from(10001).is_err());
        assert!(ExWavPan::try_from(-10001).is_err());
        assert!(ExWavPan::try_from(i64::MAX).is_err());
        assert!(ExWavPan::try_from(i64::MIN).is_err());
    }

    #[test]
    fn test_exwav_volume_try_from() {
        // Valid values
        assert!(ExWavVolume::try_from(0).is_ok());
        assert!(ExWavVolume::try_from(-10000).is_ok());
        assert!(ExWavVolume::try_from(-5000).is_ok());

        // Invalid values
        assert!(ExWavVolume::try_from(1).is_err());
        assert!(ExWavVolume::try_from(-10001).is_err());
        assert!(ExWavVolume::try_from(i64::MAX).is_err());
        assert!(ExWavVolume::try_from(i64::MIN).is_err());
    }

    #[test]
    fn test_exwav_frequency_try_from() {
        // Valid values
        assert!(ExWavFrequency::try_from(100).is_ok());
        assert!(ExWavFrequency::try_from(100000).is_ok());
        assert!(ExWavFrequency::try_from(50000).is_ok());

        // Invalid values
        assert!(ExWavFrequency::try_from(99).is_err());
        assert!(ExWavFrequency::try_from(100001).is_err());
        assert!(ExWavFrequency::try_from(0).is_err());
        assert!(ExWavFrequency::try_from(u64::MAX).is_err());
    }

    #[test]
    fn test_exwav_values() {
        // Test value() method
        let pan = ExWavPan::try_from(5000).unwrap();
        assert_eq!(pan.value(), 5000);

        let volume = ExWavVolume::try_from(-5000).unwrap();
        assert_eq!(volume.value(), -5000);

        let frequency = ExWavFrequency::try_from(48000).unwrap();
        assert_eq!(frequency.value(), 48000);
    }

    #[test]
    fn test_exwav_defaults() {
        assert_eq!(ExWavPan::default().value(), 0);
        assert_eq!(ExWavVolume::default().value(), 0);
    }
}
