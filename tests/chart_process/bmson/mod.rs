#![cfg(feature = "bmson")]

//! Integration tests for `bms_rs::chart_process::BmsonProcessor`.

mod activate_time;
mod chart;
mod continue_time;
mod playback_state;
mod visible_events;

use super::{MICROSECOND_EPSILON, assert_time_close};
