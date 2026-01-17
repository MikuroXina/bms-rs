//! Test helpers for creating model objects in integration tests.
//!
//! These functions are provided for testing purposes only.
//! They allow test code to create model objects without exposing
//! the internal constructors publicly.

use crate::bms::{
    Decimal,
    command::{
        JudgeLevel, ObjId, channel::NoteChannelId, graphics::Argb, minor_command::SwBgaEvent,
        time::ObjTime,
    },
    model::obj::{
        BgaArgbObj, BgaKeyboundObj, BgaLayer, BgaObj, BpmChangeObj, JudgeObj, OptionObj,
        ScrollingFactorObj, SeekObj, SpeedObj, StopObj, TextObj, WavObj,
    },
};

/// Creates a new WAV object for testing purposes.
#[must_use]
pub const fn wav_obj(offset: ObjTime, channel_id: NoteChannelId, wav_id: ObjId) -> WavObj {
    WavObj::new(offset, channel_id, wav_id)
}

/// Creates a new BGA object for testing purposes.
#[must_use]
pub const fn bga_obj(time: ObjTime, id: ObjId, layer: BgaLayer) -> BgaObj {
    BgaObj::new(time, id, layer)
}

/// Creates a new BPM change object for testing purposes.
#[must_use]
pub const fn bpm_change_obj(time: ObjTime, def_id: ObjId, bpm: Decimal) -> BpmChangeObj {
    BpmChangeObj::new(time, def_id, bpm)
}

/// Creates a new stop object for testing purposes.
#[must_use]
pub const fn stop_obj(time: ObjTime, def_id: ObjId, duration: Decimal) -> StopObj {
    StopObj::new(time, def_id, duration)
}

/// Creates a new scrolling factor object for testing purposes.
#[must_use]
pub const fn scrolling_factor_obj(
    time: ObjTime,
    def_id: ObjId,
    factor: Decimal,
) -> ScrollingFactorObj {
    ScrollingFactorObj::new(time, def_id, factor)
}

/// Creates a new speed object for testing purposes.
#[must_use]
pub const fn speed_obj(time: ObjTime, def_id: ObjId, factor: Decimal) -> SpeedObj {
    SpeedObj::new(time, def_id, factor)
}

/// Creates a new BGA ARGB object for testing purposes.
#[must_use]
pub const fn bga_argb_obj(time: ObjTime, layer: BgaLayer, def_id: ObjId, argb: Argb) -> BgaArgbObj {
    BgaArgbObj::new(time, layer, def_id, argb)
}

/// Creates a new seek object for testing purposes.
#[must_use]
pub const fn seek_obj(time: ObjTime, def_id: ObjId, position: Decimal) -> SeekObj {
    SeekObj::new(time, def_id, position)
}

/// Creates a new text object for testing purposes.
#[must_use]
pub const fn text_obj(time: ObjTime, def_id: ObjId, text: String) -> TextObj {
    TextObj::new(time, def_id, text)
}

/// Creates a new judge object for testing purposes.
#[must_use]
pub const fn judge_obj(time: ObjTime, def_id: ObjId, judge_level: JudgeLevel) -> JudgeObj {
    JudgeObj::new(time, def_id, judge_level)
}

/// Creates a new BGA keybound object for testing purposes.
#[must_use]
pub const fn bga_keybound_obj(time: ObjTime, def_id: ObjId, event: SwBgaEvent) -> BgaKeyboundObj {
    BgaKeyboundObj::new(time, def_id, event)
}

/// Creates a new option object for testing purposes.
#[must_use]
pub const fn option_obj(time: ObjTime, def_id: ObjId, option: String) -> OptionObj {
    OptionObj::new(time, def_id, option)
}
