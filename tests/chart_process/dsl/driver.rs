//! Test player driver implementation
//!
//! Provides the `TestPlayerDriver` type to simplify player lifecycle management in test code.

use std::ops::Add;

use bms_rs::chart_process::resource::ResourceMapping;
use bms_rs::chart_process::{ControlEvent, player::UniversalChartPlayer};
use gametime::{TimeSpan, TimeStamp};

use super::time_step::{TimeStep, TimeStepBuilder};

/// Test player driver
///
/// Provides a fluent builder API to simplify player lifecycle management in test code.
///
/// # Type Parameters
///
/// * `R` - Resource mapping type, must implement the `ResourceMapping` trait
///
/// # Examples
///
/// ```no_run
/// # use bms_rs::bms::Decimal;
/// # use bms_rs::chart_process::player::UniversalChartPlayer;
/// # use bms_rs::chart_process::resource::HashMapResourceMapping;
/// # use crate::tests::chart_process::dsl::TestPlayerDriver;
/// # use gametime::TimeSpan;
/// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
/// // Simple scenario: single time advancement
/// TestPlayerDriver::new(player)
///     .past(TimeSpan::SECOND)
///     .view(|p| assert_eq!(p.current_bpm(), &Decimal::from(120)))
///     .events(|evs| assert_eq!(evs.len(), 3))
///     .run();
///
/// // Complex scenario: multiple time advancements
/// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
/// TestPlayerDriver::new(player)
///     .past(TimeSpan::SECOND)
///         .view(|p| assert_eq!(p.current_bpm(), &Decimal::from(120)))
///     .past(TimeSpan::SECOND)
///         .view(|p| assert_eq!(p.current_bpm(), &Decimal::from(180)))
///     .past(TimeSpan::SECOND)
///         .view(|p| assert_eq!(p.current_bpm(), &Decimal::from(240)))
///     .run();
/// ```
pub struct TestPlayerDriver<R: ResourceMapping> {
    /// Player instance
    player: Option<UniversalChartPlayer<R>>,
    /// Start time
    start_time: Option<TimeStamp>,
    /// List of time advancement steps (public, for `TimeStepBuilder` access)
    pub(crate) time_steps: Vec<TimeStep<R>>,
}

impl<R: ResourceMapping> TestPlayerDriver<R> {
    /// Creates a new test driver
    ///
    /// # Parameters
    ///
    /// * `player` - Player instance to test
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bms_rs::chart_process::player::UniversalChartPlayer;
    /// # use bms_rs::chart_process::resource::HashMapResourceMapping;
    /// # use crate::tests::chart_process::dsl::TestPlayerDriver;
    /// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
    /// let driver = TestPlayerDriver::new(player);
    /// ```
    pub const fn new(player: UniversalChartPlayer<R>) -> Self {
        Self {
            player: Some(player),
            start_time: None,
            time_steps: Vec::new(),
        }
    }

    /// Sets the start time
    ///
    /// # Parameters
    ///
    /// * `time` - Timestamp to start playback, defaults to `TimeStamp::now()`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bms_rs::chart_process::player::UniversalChartPlayer;
    /// # use bms_rs::chart_process::resource::HashMapResourceMapping;
    /// # use crate::tests::chart_process::dsl::TestPlayerDriver;
    /// # use gametime::TimeStamp;
    /// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
    /// TestPlayerDriver::new(player)
    ///     .start_at(TimeStamp::start())
    ///     .run();
    /// ```
    #[must_use]
    pub const fn start_at(mut self, time: TimeStamp) -> Self {
        self.start_time = Some(time);
        self
    }

    /// Advances time (core method)
    ///
    /// Creates a new time advancement step and returns a step builder for adding assertions.
    ///
    /// # Parameters
    ///
    /// * `duration` - Duration of time advancement
    ///
    /// # Returns
    ///
    /// Returns a `TimeStepBuilder` for adding state assertions and event assertions.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bms_rs::bms::Decimal;
    /// # use bms_rs::chart_process::player::UniversalChartPlayer;
    /// # use bms_rs::chart_process::resource::HashMapResourceMapping;
    /// # use crate::tests::chart_process::dsl::TestPlayerDriver;
    /// # use gametime::TimeSpan;
    /// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
    /// TestPlayerDriver::new(player)
    ///     .past(TimeSpan::SECOND)
    ///         .view(|p| assert_eq!(p.current_bpm(), &Decimal::from(120)))
    ///         .events(|evs| assert_eq!(evs.len(), 3))
    ///     .run();
    /// ```
    pub fn past(&mut self, duration: TimeSpan) -> TimeStepBuilder<'_, R> {
        let step = TimeStep::new(duration);
        self.time_steps.push(step);
        TimeStepBuilder::new(duration, self)
    }

    /// Convenience method: advance by specified milliseconds
    ///
    /// # Parameters
    ///
    /// * `millis` - Milliseconds to advance
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bms_rs::chart_process::player::UniversalChartPlayer;
    /// # use bms_rs::chart_process::resource::HashMapResourceMapping;
    /// # use crate::tests::chart_process::dsl::TestPlayerDriver;
    /// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
    /// TestPlayerDriver::new(player)
    ///     .past_ms(500)
    ///     .view(|p| { /* ... */ })
    ///     .run();
    /// ```
    pub fn past_ms(&mut self, millis: i64) -> TimeStepBuilder<'_, R> {
        self.past(TimeSpan::MILLISECOND * millis)
    }

    /// Convenience method: advance by specified seconds
    ///
    /// # Parameters
    ///
    /// * `secs` - Seconds to advance
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bms_rs::chart_process::player::UniversalChartPlayer;
    /// # use bms_rs::chart_process::resource::HashMapResourceMapping;
    /// # use crate::tests::chart_process::dsl::TestPlayerDriver;
    /// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
    /// TestPlayerDriver::new(player)
    ///     .past_sec(2)
    ///     .view(|p| { /* ... */ })
    ///     .run();
    /// ```
    pub fn past_sec(&mut self, secs: i64) -> TimeStepBuilder<'_, R> {
        self.past(TimeSpan::SECOND * secs)
    }

    /// Runs the test
    ///
    /// Executes all time advancement steps and assertions.
    ///
    /// # Panics
    ///
    /// - Panics if player is not set
    /// - Panics if any assertion fails (via standard library `assert!` macro)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bms_rs::chart_process::player::UniversalChartPlayer;
    /// # use bms_rs::chart_process::resource::HashMapResourceMapping;
    /// # use crate::tests::chart_process::dsl::TestPlayerDriver;
    /// # use gametime::TimeSpan;
    /// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
    /// TestPlayerDriver::new(player)
    ///     .past(TimeSpan::SECOND)
    ///     .view(|p| { /* ... */ })
    ///     .run();
    /// ```
    pub fn run(mut self) {
        self.run_mut();
    }

    /// Checks player state (without time advancement)
    ///
    /// Adds a zero-duration time step for state checking.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bms_rs::bms::Decimal;
    /// # use bms_rs::chart_process::player::UniversalChartPlayer;
    /// # use bms_rs::chart_process::resource::HashMapResourceMapping;
    /// # use crate::tests::chart_process::dsl::TestPlayerDriver;
    /// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
    /// TestPlayerDriver::new(player)
    ///     .check(|p| assert_eq!(p.current_bpm(), &Decimal::from(120)))
    ///     .run();
    /// ```
    #[must_use]
    pub fn check<F>(mut self, check: F) -> Self
    where
        F: Fn(&UniversalChartPlayer<R>) + 'static,
    {
        let _ = self.past(TimeSpan::ZERO).view(check);
        self
    }

    /// Manually trigger an update at the specified timestamp
    ///
    /// # Parameters
    ///
    /// * `time` - Timestamp to update to
    #[must_use]
    pub fn update_at(mut self, time: TimeStamp) -> Self {
        let _ = self.past(TimeSpan::ZERO).update_at(time);
        self
    }

    /// Post control events to the player
    ///
    /// # Parameters
    ///
    /// * `events` - Iterator of control events to post
    #[must_use]
    pub fn post_events<I>(mut self, events: I) -> Self
    where
        I: IntoIterator<Item = ControlEvent> + 'static,
    {
        let _ = self.past(TimeSpan::ZERO).post_events(events);
        self
    }

    /// Runs the test (mutable reference version)
    ///
    /// Executes all time advancement steps and assertions. Unlike `run()`, this method
    /// accepts a mutable reference instead of taking ownership.
    pub(crate) fn run_mut(&mut self) {
        let mut player = self.player.take().expect("player not set");
        let start_time = self.start_time.unwrap_or_else(TimeStamp::now);

        // Auto-start playback
        player.start_play(start_time);

        let mut current_time = start_time;

        // Execute all time advancement steps
        for step in &mut self.time_steps {
            // 1. Execute control actions first
            let actions = std::mem::take(&mut step.control_actions);
            for action in actions {
                action(&mut player);
            }

            // 2. Advance time
            current_time = current_time.add(step.duration);

            // 3. Update player and collect events
            let events: Vec<_> = player.update(current_time).collect();

            // 4. Execute state assertions
            for assertion in &step.state_assertions {
                assertion(&player);
            }

            // 5. Execute event assertions
            for assertion in &step.event_assertions {
                assertion(&events);
            }
        }
    }
}
