#![cfg(feature = "bmson")]

//! Integration tests for `bms_rs::bmson::process` (Process trait on Bmson).

mod activate_time;
mod chart;
mod continue_time;
mod playback_state;
mod visible_events;

use super::{MICROSECOND_EPSILON, assert_time_close};
