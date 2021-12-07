use std::num::NonZeroU16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerMode {
    Single,
    Two,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JudgeLevel {
    VeryHard,
    Hard,
    Normal,
    Easy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WavId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BgiId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Volume {
    pub relative_percent: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Argb {
    pub alpha: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoteKind {
    Visible,
    Invisible,
    Long,
    Landmine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Scratch,
    FreeZone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Note {
    pub kind: NoteKind,
    pub is_player1: bool,
    pub key: Key,
    pub wav: Option<WavId>,
    pub damage: Option<NonZeroU16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpeedLength {
    integral: u64,
    fractional: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Channel {
    Bgm(WavId),
    SectionLen(SpeedLength),
    BpmChange(u8),
    BgaBase(BgiId),
    ExtObj(String),
    SeekObj(i32),
    BgaPoor(BgiId),
    BgaLayer(BgiId),
    ExtBpmChange(SpeedLength),
    Stop(u64),
    BgaLayer2(BgiId),
    BgaBaseOpacity(u8),
    BgaLayerOpactiy(u8),
    BgaLayer2Opacity(u8),
    BgaPoorOpacity(u8),
    BgmVolume(u8),
    KeyVolume(u8),
    Text(String),
    BgaBaseArgb(Argb),
    BgaLayerArgb(Argb),
    BgaLayer2Argb(Argb),
    BgaPoorArgb(Argb),
    BgaKeyBound(String),
    ChangeOption(String),
    Note(Note),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Track(u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Message<'a> {
    pub track: Track,
    pub channel: Channel,
    pub message: &'a str,
}
