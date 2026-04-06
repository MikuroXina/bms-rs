#![cfg(feature = "bmson")]

//! Integration tests for `bms_rs::chart::BmsonProcessor`.

mod activate_time;
mod chart;
mod continue_time;
mod playback_state;
mod visible_events;

use super::{assert_time_close, MICROSECOND_EPSILON};
