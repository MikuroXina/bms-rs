//! Definitions of command argument data.
pub mod channel;

use std::{ops::Deref, time::Duration};

pub use channel::Channel;

use crate::time::ObjTime;

use super::{Result, cursor::Cursor};

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

impl PlayerMode {
    pub(crate) fn from(c: &mut Cursor) -> Result<Self> {
        Ok(match c.next_token() {
            Some("1") => Self::Single,
            Some("2") => Self::Two,
            Some("3") => Self::Double,
            _ => return Err(c.make_err_expected_token("one of 1, 2 or 3")),
        })
    }
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

impl JudgeLevel {
    pub(crate) fn try_read(c: &mut Cursor) -> Result<Self> {
        c.next_token()
            .ok_or(c.make_err_expected_token("one of [0,4]"))?
            .try_into()
            .map_err(|_| c.make_err_expected_token("one of [0,4]"))
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
    matches!(char_to_base62('/'), None);
    matches!(char_to_base62('0'), Some(b'0'));
    matches!(char_to_base62('9'), Some(b'9'));
    matches!(char_to_base62(':'), None);
    matches!(char_to_base62('@'), None);
    matches!(char_to_base62('A'), Some(b'A'));
    matches!(char_to_base62('Z'), Some(b'Z'));
    matches!(char_to_base62('['), None);
    matches!(char_to_base62('`'), None);
    matches!(char_to_base62('a'), Some(b'a'));
    matches!(char_to_base62('z'), Some(b'z'));
    matches!(char_to_base62('{'), None);
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

impl ObjId {
    pub(crate) fn try_load(value: &str, c: &mut Cursor) -> Result<Self> {
        Self::try_from(value).map_err(|_| c.make_err_object_id(value.to_string()))
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
    pub relative_percent: u64,
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

/// A kind of the note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NoteKind {
    /// A normal note can be seen by the user.
    Visible,
    /// A invisible note cannot be played by the user.
    Invisible,
    /// A long-press note (LN), requires the user to hold pressing the key.
    Long,
    /// A landmine note that treated as POOR judgement when
    Landmine,
}

impl NoteKind {
    /// Returns whether the note is a playable.
    pub const fn is_playable(self) -> bool {
        !matches!(self, Self::Invisible)
    }

    /// Returns whether the note is a long-press note.
    pub const fn is_long(self) -> bool {
        matches!(self, Self::Long)
    }
}

/// A key of the controller or keyboard.
///
/// - Beat 5K/7K/10K/14K:
/// ```text
/// |---------|----------------------|
/// |         |   [K2]  [K4]  [K6]   |
/// |(Scratch)|[K1]  [K3]  [K5]  [K7]|
/// |---------|----------------------|
/// ```
///
/// - PMS:
/// ```text
/// |----------------------------|
/// |   [K2]  [K4]  [K6]  [K8]   |
/// |[K1]  [K3]  [K5]  [K7]  [K9]|
/// |----------------------------|
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Key {
    /// The leftmost white key.
    /// `11` in BME-type Player1.
    Key1,
    /// The leftmost black key.
    /// `12` in BME-type Player1.
    Key2,
    /// The second white key from the left.
    /// `13` in BME-type Player1.
    Key3,
    /// The second black key from the left.
    /// `14` in BME-type Player1.
    Key4,
    /// The third white key from the left.
    /// `15` in BME-type Player1.
    Key5,
    /// The rightmost black key.
    /// `18` in BME-type Player1.
    Key6,
    /// The rightmost white key.
    /// `19` in BME-type Player1.
    Key7,
    /// The extra black key. Used in PMS or other modes.
    Key8,
    /// The extra white key. Used in PMS or other modes.
    Key9,
    /// The extra key for OCT/FP.
    Key10,
    /// The extra key for OCT/FP.
    Key11,
    /// The extra key for OCT/FP.
    Key12,
    /// The extra key for OCT/FP.
    Key13,
    /// The extra key for OCT/FP.
    Key14,
    /// The scratch disk.
    /// `16` in BME-type Player1.
    Scratch,
    /// The extra scratch disk on the right. Used in DSC and OCT/FP mode.
    ScratchExtra,
    /// The foot pedal.
    FootPedal,
    /// The zone that the user can scratch disk freely.
    /// `17` in BMS-type Player1.
    FreeZone,
}

/// A POOR BGA display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PoorMode {
    /// To hide the normal BGA and display the POOR BGA.
    Interrupt,
    /// To overlap the POOR BGA onto the normal BGA.
    Overlay,
    /// Not to display the POOR BGA.
    Hidden,
}

impl Default for PoorMode {
    fn default() -> Self {
        Self::Interrupt
    }
}

impl PoorMode {
    pub(crate) fn from(c: &mut Cursor) -> Result<Self> {
        Ok(match c.next_token() {
            Some("0") => Self::Interrupt,
            Some("1") => Self::Overlay,
            Some("2") => Self::Hidden,
            _ => return Err(c.make_err_expected_token("one of 0, 1 or 2")),
        })
    }
}

/// A track, or bar, in the score.
/// It is recommended to be greater than 0, but some scores may include the 0 track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Track(pub u64);

/// Pan value for ExWav sound effect.
/// Range: [-10000, 10000]. -10000 is leftmost, 10000 is rightmost.
/// Default: 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default)]
pub struct ExWavPan(i64);

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

impl TryFrom<i64> for ExWavPan {
    type Error = i64;

    fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
        Self::new(value).ok_or(value.clamp(-10000, 10000))
    }
}

/// Volume value for ExWav sound effect.
/// Range: [-10000, 0]. -10000 is 0%, 0 is 100%.
/// Default: 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default)]
pub struct ExWavVolume(i64);

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
pub struct ExWavFrequency(u64);

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

impl TryFrom<u64> for ExWavFrequency {
    type Error = u64;

    fn try_from(value: u64) -> std::result::Result<Self, Self::Error> {
        Self::new(value).ok_or(value.clamp(100, 100000))
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

/// A side of the player.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PlayerSide {
    /// The player 1 side.
    #[default]
    Player1,
    /// The player 2 side.
    Player2,
}

/// RGB结构体，用于#VIDEOCOLORS等命令。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rgb {
    /// 红色分量
    pub r: u8,
    /// 绿色分量
    pub g: u8,
    /// 蓝色分量
    pub b: u8,
}

/// 只要求finite的f64包装类型（如Scroll等）。
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FiniteF64(f64);

impl FiniteF64 {
    /// 创建一个只要finite的包装类型。
    pub fn new(val: f64) -> Option<Self> {
        if val.is_finite() {
            Some(Self(val))
        } else {
            None
        }
    }
    /// 获取原始值
    pub fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for FiniteF64 {
    type Error = f64;
    fn try_from(value: f64) -> std::result::Result<Self, Self::Error> {
        Self::new(value).ok_or(value)
    }
}

impl From<FiniteF64> for f64 {
    fn from(value: FiniteF64) -> Self {
        value.0
    }
}

impl Eq for FiniteF64 {}
impl Ord for FiniteF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl Deref for FiniteF64 {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Add for FiniteF64 {
    type Output = Option<Self>;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.0 + rhs.0)
    }
}
impl std::ops::Sub for FiniteF64 {
    type Output = Option<Self>;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.0 - rhs.0)
    }
}

/// 要求finite且大于0的f64包装类型（如Bpm/Stop/Speed/Total等）。
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PositiveFiniteF64(f64);

impl PositiveFiniteF64 {
    /// 创建一个finite且大于0的包装类型。
    pub fn new(val: f64) -> Option<Self> {
        if val.is_finite() && val > 0.0 {
            Some(Self(val))
        } else {
            None
        }
    }
    /// 获取原始值
    pub fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for PositiveFiniteF64 {
    type Error = f64;
    fn try_from(value: f64) -> std::result::Result<Self, Self::Error> {
        Self::new(value).ok_or(value)
    }
}

impl From<PositiveFiniteF64> for f64 {
    fn from(value: PositiveFiniteF64) -> Self {
        value.0
    }
}

impl Eq for PositiveFiniteF64 {}
impl Ord for PositiveFiniteF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl Deref for PositiveFiniteF64 {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Add for PositiveFiniteF64 {
    type Output = Option<Self>;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.0 + rhs.0)
    }
}
impl std::ops::Sub for PositiveFiniteF64 {
    type Output = Option<Self>;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.0 - rhs.0)
    }
}

/// Long Note Mode Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LnModeType {
    /// Normal Long Note (LN)
    Ln,
    /// Classic Long Note (CN)
    Cn,
    /// Hell Classic Long Note (HCN)
    Hcn,
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

/// bemaniaDX型STP序列定义。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
/// - Volume: volume percent (0-100)
/// - Time: playback time (ms*0.5, 0 means original length)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WavCmdParam {
    /// Pitch (0-127, 60 is C6)
    Pitch,
    /// Volume percent (0-100)
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// BM98 #ExtChr extended character customization event.
///
/// Used for #ExtChr command, implements custom UI element image replacement.
/// - sprite_num: character index to replace [0-1023]
/// - bmp_num: BMP index (hex to decimal, or -1/-257, etc.)
/// - start_x/start_y: crop start point
/// - end_x/end_y: crop end point
/// - offset_x/offset_y: offset (optional)
/// - abs_x/abs_y: absolute coordinate (optional)
#[derive(Debug, Clone, PartialEq, Eq)]
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
