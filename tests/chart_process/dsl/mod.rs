//! Test DSL module
//!
//! Provides a builder pattern-based test DSL to simplify player lifecycle management in test code.
//!
//! # Core Types
//!
//! - [`TestPlayerDriver`](TestPlayerDriver) - Main driver that manages the entire test lifecycle
//! - [`TimeStepBuilder`](TimeStepBuilder) - Time step builder for adding assertions
//!
//! # Usage Examples
//!
//! ## Simple Scenario
//!
//! ```no_run
//! # use bms_rs::bms::Decimal;
//! # use bms_rs::chart_process::player::UniversalChartPlayer;
//! # use bms_rs::chart_process::resource::HashMapResourceMapping;
//! # use crate::tests::chart_process::dsl::TestPlayerDriver;
//! # use gametime::TimeSpan;
//! # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
//! TestPlayerDriver::new(player)
//!     .past(TimeSpan::SECOND)
//!     .view(|p| assert_eq!(p.current_bpm(), &Decimal::from(120)))
//!     .events(|evs| assert_eq!(evs.len(), 3))
//!     .run();
//! ```
//!
//! ## Multiple Time Steps
//!
//! ```no_run
//! # use bms_rs::bms::Decimal;
//! # use bms_rs::chart_process::player::UniversalChartPlayer;
//! # use bms_rs::chart_process::resource::HashMapResourceMapping;
//! # use crate::tests::chart_process::dsl::TestPlayerDriver;
//! # use gametime::TimeSpan;
//! # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
//! TestPlayerDriver::new(player)
//!     .past(TimeSpan::SECOND)
//!         .view(|p| assert_eq!(p.current_bpm(), &Decimal::from(120)))
//!     .past(TimeSpan::SECOND)
//!         .view(|p| assert_eq!(p.current_bpm(), &Decimal::from(180)))
//!     .past(TimeSpan::SECOND)
//!         .view(|p| assert_eq!(p.current_bpm(), &Decimal::from(240)))
//!     .run();
//! ```
//!
//! ## Visible Event Verification
//!
//! ```no_run
//! # use bms_rs::chart_process::{ChartEvent, player::UniversalChartPlayer, resource::HashMapResourceMapping};
//! # use crate::tests::chart_process::dsl::TestPlayerDriver;
//! # use gametime::TimeSpan;
//! # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
//! TestPlayerDriver::new(player)
//!     .past(TimeSpan::MILLISECOND * 100)
//!     .view(|p| {
//!         let mut found = false;
//!         for (ev, _) in p.visible_events() {
//!             if let ChartEvent::Note { .. } = ev.event() {
//!                 found = true;
//!                 break;
//!             }
//!         }
//!         assert!(found);
//!     })
//!     .run();
//! ```

mod driver;
mod helpers;
mod time_step;

pub use driver::TestPlayerDriver;
pub use helpers::{
    bms_driver_with_newer_prompter, bms_driver_with_older_prompter, test_player_driver,
};
pub use time_step::TimeStepBuilder;

#[cfg(feature = "bmson")]
pub use helpers::bmson_driver;
