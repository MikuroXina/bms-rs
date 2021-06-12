use std::{collections::HashMap, ffi::OsString, num::NonZeroU16};

pub enum PlayerMode {
    Single,
    Two,
    Double,
}

pub enum JudgeLevel {
    VeryHard,
    Hard,
    Normal,
    Easy,
}

pub struct WavId(NonZeroU16);

pub struct BgiId(NonZeroU16);

pub struct Volume {
    relative_percent: u8,
}

pub struct Header {
    player_mode: PlayerMode,
    genre: String,
    title: String,
    artist: String,
    bpm: f64,
    midi_bgm: Option<OsString>,
    play_level: u8,
    wavs: HashMap<WavId, OsString>,
    bgis: HashMap<BgiId, OsString>,
}

pub struct Argb {
    alpha: u8,
    red: u8,
    green: u8,
    blue: u8,
}

pub enum NoteKind {
    Visible,
    Invisible,
    Long,
    Landmine,
}

pub struct Note {
    kind: NoteKind,
    is_player1: bool,
    key: Option<WavId>,
    damage: Option<NonZeroU16>,
}

pub enum Channel {
    Bgm(WavId),
    SectionLen(f64),
    BpmChange(u8),
    BgaBase(BgiId),
    ExtObj(String),
    SeekObj(i32),
    BgaPoor(BgiId),
    BgaLayer(BgiId),
    ExtBpmChange(f64),
    Stop(u64),
    BgaLayer2(BgiId),
    BgaBaseOpacity(u8),
    BgaLayerOpcatiy(u8),
    BgaLayer2Opacity(u8),
    BgaPoorOpacity(u8),
    BgmVolume(u8),
    KeyVolume(u8),
    Text(String),
    JudgeLevel(JudgeLevel),
    BgaBaseArgb(Argb),
    BgaLayerArgb(Argb),
    BgaLayer2Argb(Argb),
    BgaPoorArgb(Argb),
    BgaKeybound(String),
    ChangeOption(String),
    Note(Note),
}

pub struct Track(u32);

pub enum Command {
    Channel {
        track: Track,
        channel: Channel,
    },
}

