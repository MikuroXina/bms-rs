//! Definitions of command argument data.
pub mod channel;
pub mod graphics;
pub mod time;

#[allow(unused)]
use std::time::Duration;

/// Export defs
pub use channel::*;
pub use graphics::*;
pub use time::*;

/// A play style of the score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlayerMode {
    /// For single play, a player uses 5 or 7 keys.
    Single,
    /// For couple play, two players use each 5 or 7 keys.
    Two,
    /// For double play, a player uses 10 or 14 keys.
    Double,
}

/// A rank to determine judge level, but treatment differs among the BMS players.
///
/// IIDX/LR2/beatoraja judge windows: https://iidx.org/misc/iidx_lr2_beatoraja_diff
///
/// Note: VeryEasy is not Implemented.
/// For `#RANK 4`, `#RANK 6` and `#RANK -1`: Usage differs among the BMS players.
/// See: https://github.com/MikuroXina/bms-rs/pull/122
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum JudgeLevel {
    /// Rank 0, the most difficult rank.
    VeryHard,
    /// Rank 1, the harder rank.
    Hard,
    /// Rank 2, the normal rank.
    Normal,
    /// Rank 3, the easier rank.
    Easy,
    /// Other integer value. Please See `JudgeLevel` for more details.
    OtherInt(i64),
}

impl From<i64> for JudgeLevel {
    fn from(value: i64) -> Self {
        match value {
            0 => Self::VeryHard,
            1 => Self::Hard,
            2 => Self::Normal,
            3 => Self::Easy,
            val => Self::OtherInt(val),
        }
    }
}

impl<'a> TryFrom<&'a str> for JudgeLevel {
    type Error = &'a str;
    fn try_from(value: &'a str) -> core::result::Result<Self, Self::Error> {
        value
            .parse::<i64>()
            .map(JudgeLevel::from)
            .map_err(|_| value)
    }
}

fn char_to_base62(ch: char) -> Option<u8> {
    match ch {
        '0'..='9' | 'A'..='Z' | 'a'..='z' => Some(ch as u32 as u8),
        _ => None,
    }
}

fn base62_to_byte(base62: u8) -> u8 {
    match base62 {
        b'0'..=b'9' => base62 - b'0',
        b'A'..=b'Z' => base62 - b'A' + 10,
        b'a'..=b'z' => base62 - b'a' + 36,
        _ => unreachable!(),
    }
}

#[test]
fn test_base62() {
    assert_eq!(char_to_base62('/'), None);
    assert_eq!(char_to_base62('0'), Some(b'0'));
    assert_eq!(char_to_base62('9'), Some(b'9'));
    assert_eq!(char_to_base62(':'), None);
    assert_eq!(char_to_base62('@'), None);
    assert_eq!(char_to_base62('A'), Some(b'A'));
    assert_eq!(char_to_base62('Z'), Some(b'Z'));
    assert_eq!(char_to_base62('['), None);
    assert_eq!(char_to_base62('`'), None);
    assert_eq!(char_to_base62('a'), Some(b'a'));
    assert_eq!(char_to_base62('z'), Some(b'z'));
    assert_eq!(char_to_base62('{'), None);
}

/// An object id. Its meaning is determined by the channel belonged to.
///
/// The representation is 2 digits of ASCII characters.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjId([u8; 2]);

impl std::fmt::Debug for ObjId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObjId")
            .field(&format!("{}{}", self.0[0] as char, self.0[1] as char))
            .finish()
    }
}

impl TryFrom<[char; 2]> for ObjId {
    type Error = [char; 2];
    fn try_from(value: [char; 2]) -> core::result::Result<Self, Self::Error> {
        Ok(Self([
            char_to_base62(value[0]).ok_or(value)?,
            char_to_base62(value[1]).ok_or(value)?,
        ]))
    }
}

impl TryFrom<[u8; 2]> for ObjId {
    type Error = [u8; 2];
    fn try_from(value: [u8; 2]) -> core::result::Result<Self, Self::Error> {
        <Self as TryFrom<[char; 2]>>::try_from([value[0] as char, value[1] as char])
            .map_err(|_| value)
    }
}

impl<'a> TryFrom<&'a str> for ObjId {
    type Error = &'a str;
    fn try_from(value: &'a str) -> core::result::Result<Self, Self::Error> {
        if value.len() != 2 {
            return Err(value);
        }
        let mut chars = value.bytes();
        let [Some(ch1), Some(ch2), None] = [chars.next(), chars.next(), chars.next()] else {
            return Err(value);
        };
        Self::try_from([ch1, ch2]).map_err(|_| value)
    }
}

impl From<ObjId> for u16 {
    fn from(value: ObjId) -> Self {
        base62_to_byte(value.0[0]) as u16 * 62 + base62_to_byte(value.0[1]) as u16
    }
}

impl From<ObjId> for u32 {
    fn from(value: ObjId) -> Self {
        Into::<u16>::into(value) as u32
    }
}

impl From<ObjId> for u64 {
    fn from(value: ObjId) -> Self {
        Into::<u16>::into(value) as u64
    }
}

impl ObjId {
    /// Instances a special null id, which means the rest object.
    pub const fn null() -> Self {
        Self([0, 0])
    }

    /// Converts the object id into an `u16` value.
    pub fn as_u16(self) -> u16 {
        self.into()
    }

    /// Converts the object id into an `u32` value.
    pub fn as_u32(self) -> u32 {
        self.into()
    }

    /// Converts the object id into an `u64` value.
    pub fn as_u64(self) -> u64 {
        self.into()
    }

    /// Makes the object id uppercase.
    pub fn make_uppercase(&mut self) {
        self.0[0] = self.0[0].to_ascii_uppercase();
        self.0[1] = self.0[1].to_ascii_uppercase();
    }
}

/// A play volume of the sound in the score. Defaults to 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Volume {
    /// A play volume percentage of the sound.
    pub relative_percent: u8,
}

impl Default for Volume {
    fn default() -> Self {
        Self {
            relative_percent: 100,
        }
    }
}

/// An alpha-red-gree-blue color data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Argb {
    /// A component of alpha.
    pub alpha: u8,
    /// A component of red.
    pub red: u8,
    /// A component of green.
    pub green: u8,
    /// A component of blue.
    pub blue: u8,
}

impl Default for Argb {
    fn default() -> Self {
        Self {
            alpha: 255,
            red: 0,
            green: 0,
            blue: 0,
        }
    }
}

/// A POOR BGA display mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PoorMode {
    /// To hide the normal BGA and display the POOR BGA.
    #[default]
    Interrupt,
    /// To overlap the POOR BGA onto the normal BGA.
    Overlay,
    /// Not to display the POOR BGA.
    Hidden,
}

/// Pan value for ExWav sound effect.
/// Range: [-10000, 10000]. -10000 is leftmost, 10000 is rightmost.
/// Default: 0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "minor-command")]
pub struct ExWavPan(i64);

#[cfg(feature = "minor-command")]
impl ExWavPan {
    /// Creates a new ExWavPan value.
    /// Returns `None` if the value is out of range [-10000, 10000].
    pub fn new(value: i64) -> Option<Self> {
        (-10000..=10000).contains(&value).then_some(Self(value))
    }

    /// Returns the underlying value.
    pub fn value(self) -> i64 {
        self.0
    }

    /// Returns the default value (0).
    pub const fn default() -> Self {
        Self(0)
    }
}

#[cfg(feature = "minor-command")]
impl TryFrom<i64> for ExWavPan {
    type Error = i64;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        Self::new(value).ok_or(value.clamp(-10000, 10000))
    }
}

/// Volume value for ExWav sound effect.
/// Range: [-10000, 0]. -10000 is 0%, 0 is 100%.
/// Default: 0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "minor-command")]
pub struct ExWavVolume(i64);

#[cfg(feature = "minor-command")]
impl ExWavVolume {
    /// Creates a new ExWavVolume value.
    /// Returns `None` if the value is out of range [-10000, 0].
    pub fn new(value: i64) -> Option<Self> {
        (-10000..=0).contains(&value).then_some(Self(value))
    }

    /// Returns the underlying value.
    pub fn value(self) -> i64 {
        self.0
    }

    /// Returns the default value (0).
    pub const fn default() -> Self {
        Self(0)
    }
}

#[cfg(feature = "minor-command")]
impl TryFrom<i64> for ExWavVolume {
    type Error = i64;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        Self::new(value).ok_or(value.clamp(-10000, 0))
    }
}

/// Frequency value for ExWav sound effect.
/// Range: [100, 100000]. Unit: Hz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "minor-command")]
pub struct ExWavFrequency(u64);

#[cfg(feature = "minor-command")]
impl ExWavFrequency {
    /// Creates a new ExWavFrequency value.
    /// Returns `None` if the value is out of range [100, 100000].
    pub fn new(value: u64) -> Option<Self> {
        (100..=100000).contains(&value).then_some(Self(value))
    }

    /// Returns the underlying value.
    pub fn value(self) -> u64 {
        self.0
    }
}

#[cfg(feature = "minor-command")]
impl TryFrom<u64> for ExWavFrequency {
    type Error = u64;

    fn try_from(value: u64) -> std::result::Result<Self, Self::Error> {
        Self::new(value).ok_or(value.clamp(100, 100000))
    }
}

/// RGB struct, used for #VIDEOCOLORS and similar commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rgb {
    /// Red component
    pub r: u8,
    /// Green component
    pub g: u8,
    /// Blue component
    pub b: u8,
}

/// A notation type about LN in the score. But you don't have to take care of how the notes are actually placed in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LnType {
    /// The RDM type.
    #[default]
    Rdm,
    /// The MGQ type.
    Mgq,
}

/// Long Note Mode Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum LnModeType {
    /// Normal Long Note (LN)
    Ln = 1,
    /// Classic Long Note (CN)
    Cn = 2,
    /// Hell Classic Long Note (HCN)
    Hcn = 3,
}

impl From<LnModeType> for u8 {
    fn from(mode: LnModeType) -> u8 {
        match mode {
            LnModeType::Ln => 1,
            LnModeType::Cn => 2,
            LnModeType::Hcn => 3,
        }
    }
}

impl TryFrom<u8> for LnModeType {
    type Error = u8;
    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        Ok(match value {
            1 => LnModeType::Ln,
            2 => LnModeType::Cn,
            3 => LnModeType::Hcn,
            _ => return Err(value),
        })
    }
}

/// bemaniaDX type STP sequence definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "minor-command")]
pub struct StpEvent {
    /// The time of the stop.
    pub time: ObjTime,
    /// The duration of the stop.
    pub duration: Duration,
}

/// MacBeat WAVCMD event.
///
/// Used for #WAVCMD command, represents pitch/volume/time adjustment for a specific WAV object.
/// - param: adjustment type (pitch/volume/time)
/// - wav_index: target WAV object ID
/// - value: adjustment value, meaning depends on param
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "minor-command")]
pub struct WavCmdEvent {
    /// Adjustment type (pitch/volume/time)
    pub param: WavCmdParam,
    /// Target WAV object ID
    pub wav_index: ObjId,
    /// Adjustment value, meaning depends on param
    pub value: u32,
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

/// SWBGA (Key Bind Layer Animation) event.
///
/// Used for #SWBGA command, describes key-bound BGA animation.
/// - frame_rate: frame interval (ms), e.g. 60FPS=17
/// - total_time: total animation duration (ms), 0 means while key is held
/// - line: applicable key channel (e.g. 11-19, 21-29)
/// - loop_mode: whether to loop (0: no loop, 1: loop)
/// - argb: transparent color (A,R,G,B)
/// - pattern: animation frame sequence (e.g. 01020304)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "minor-command")]
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

/// BM98 #ExtChr extended character customization event.
///
/// Used for #ExtChr command, implements custom UI element image replacement.
/// - sprite_num: character index to replace [0-1023]
/// - bmp_num: BMP index (hex to decimal, or -1/-257, etc.)
/// - start_x/start_y: crop start point
/// - end_x/end_y: crop end point
/// - offset_x/offset_y: offset (optional)
/// - abs_x/abs_y: absolute coordinate (optional)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[cfg(test)]
mod tests {
    #[cfg(feature = "minor-command")]
    use super::*;

    #[test]
    #[cfg(feature = "minor-command")]
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
    #[cfg(feature = "minor-command")]
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
    #[cfg(feature = "minor-command")]
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
    #[cfg(feature = "minor-command")]
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
    #[cfg(feature = "minor-command")]
    fn test_exwav_defaults() {
        assert_eq!(ExWavPan::default().value(), 0);
        assert_eq!(ExWavVolume::default().value(), 0);
    }
}
