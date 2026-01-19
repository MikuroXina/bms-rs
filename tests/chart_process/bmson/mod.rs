#![cfg(feature = "bmson")]

//! Integration tests for `bms_rs::chart_process::BmsonProcessor`.

mod activate_time;
mod chart;
mod continue_time;
mod playback_state;
mod visible_events;

use gametime::{TimeSpan, TimeStamp};
use num::{One, ToPrimitive};

use bms_rs::bms::Decimal;
use bms_rs::bmson::parse_bmson;
use bms_rs::chart_process::PlayheadEvent;
use bms_rs::chart_process::prelude::*;

use super::{MICROSECOND_EPSILON, assert_time_close};
