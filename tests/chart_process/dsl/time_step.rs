//! Time step and builder implementation
//!
//! Provides `TimeStep` and `TimeStepBuilder` types for building and storing time advancement steps in tests.

use bms_rs::chart_process::player::UniversalChartPlayer;
use bms_rs::chart_process::resource::ResourceMapping;
use bms_rs::chart_process::{ControlEvent, PlayheadEvent};
use gametime::{TimeSpan, TimeStamp};

use super::driver::TestPlayerDriver;

/// Type alias: state assertion closure
type StateAssertion<R> = Box<dyn Fn(&UniversalChartPlayer<R>)>;

/// Type alias: event assertion closure
type EventAssertion = Box<dyn Fn(&[PlayheadEvent])>;

/// Type alias: control action closure
type ControlAction<R> = Box<dyn FnOnce(&mut UniversalChartPlayer<R>)>;

/// Time advancement step, storing duration, state assertions, and event assertions
pub struct TimeStep<R: ResourceMapping> {
    /// Duration of time advancement
    pub duration: TimeSpan,
    /// List of state assertions
    pub state_assertions: Vec<StateAssertion<R>>,
    /// List of event assertions
    pub event_assertions: Vec<EventAssertion>,
    /// List of control actions (executed before time advancement)
    pub control_actions: Vec<ControlAction<R>>,
}

impl<R: ResourceMapping> TimeStep<R> {
    /// Creates a new time step
    pub fn new(duration: TimeSpan) -> Self {
        Self {
            duration,
            state_assertions: Vec::new(),
            event_assertions: Vec::new(),
            control_actions: Vec::new(),
        }
    }

    /// Adds a state assertion
    pub fn add_state_assertion<F>(&mut self, assertion: F)
    where
        F: Fn(&UniversalChartPlayer<R>) + 'static,
    {
        self.state_assertions.push(Box::new(assertion));
    }

    /// Adds an event assertion
    pub fn add_event_assertion<F>(&mut self, assertion: F)
    where
        F: Fn(&[PlayheadEvent]) + 'static,
    {
        self.event_assertions.push(Box::new(assertion));
    }

    /// Adds a control action
    pub fn add_control_action<F>(&mut self, action: F)
    where
        F: FnOnce(&mut UniversalChartPlayer<R>) + 'static,
    {
        self.control_actions.push(Box::new(action));
    }
}

/// Time step builder, providing a fluent chainable API
pub struct TimeStepBuilder<'a, R: ResourceMapping> {
    /// Duration of time advancement (unused, reserved for future extensions)
    _duration: TimeSpan,
    /// Mutable reference to the main driver
    driver: &'a mut TestPlayerDriver<R>,
}

impl<'a, R: ResourceMapping> TimeStepBuilder<'a, R> {
    /// Creates a new time step builder
    pub(super) const fn new(duration: TimeSpan, driver: &'a mut TestPlayerDriver<R>) -> Self {
        Self {
            _duration: duration,
            driver,
        }
    }

    /// Adds a state assertion (corresponds to `View` in examples)
    ///
    /// # Parameters
    ///
    /// * `assertion` - State assertion closure, receives a reference to the player
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
    ///     .past(gametime::TimeSpan::SECOND)
    ///     .view(|p| assert_eq!(p.current_bpm(), &Decimal::from(120)))
    /// # ;
    /// ```
    #[must_use]
    pub fn view<F>(self, assertion: F) -> Self
    where
        F: Fn(&UniversalChartPlayer<R>) + 'static,
    {
        if let Some(step) = self.driver.time_steps.last_mut() {
            step.add_state_assertion(assertion);
        }
        self
    }

    /// Adds an event assertion (corresponds to `Events` in examples)
    ///
    /// # Parameters
    ///
    /// * `assertion` - Event assertion closure, receives a reference to the event slice
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bms_rs::chart_process::player::UniversalChartPlayer;
    /// # use bms_rs::chart_process::resource::HashMapResourceMapping;
    /// # use crate::tests::chart_process::dsl::TestPlayerDriver;
    /// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
    /// TestPlayerDriver::new(player)
    ///     .past(gametime::TimeSpan::SECOND)
    ///     .events(|events| assert_eq!(events.len(), 3))
    /// # ;
    /// ```
    #[must_use]
    pub fn events<F>(self, assertion: F) -> Self
    where
        F: Fn(&[PlayheadEvent]) + 'static,
    {
        if let Some(step) = self.driver.time_steps.last_mut() {
            step.add_event_assertion(assertion);
        }
        self
    }

    /// Manually trigger an update at the specified timestamp
    ///
    /// This is useful for testing update behavior directly.
    ///
    /// # Parameters
    ///
    /// * `time` - Timestamp to update to
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
    ///     .update_at(TimeStamp::now())
    /// # ;
    /// ```
    #[must_use]
    pub fn update_at(self, time: TimeStamp) -> Self {
        if let Some(step) = self.driver.time_steps.last_mut() {
            step.add_control_action(move |p| {
                let _ = p.update(time).count();
            });
        }
        self
    }

    /// Post control events to the player
    ///
    /// # Parameters
    ///
    /// * `events` - Iterator of control events to post
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bms_rs::bms::Decimal;
    /// # use bms_rs::chart_process::{ControlEvent, player::UniversalChartPlayer, resource::HashMapResourceMapping};
    /// # use crate::tests::chart_process::dsl::TestPlayerDriver;
    /// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
    /// TestPlayerDriver::new(player)
    ///     .post_events([ControlEvent::SetPlaybackRatio { ratio: Decimal::from(2) }].into_iter())
    /// # ;
    /// ```
    #[must_use]
    pub fn post_events<I>(self, events: I) -> Self
    where
        I: IntoIterator<Item = ControlEvent> + 'static,
    {
        if let Some(step) = self.driver.time_steps.last_mut() {
            step.add_control_action(move |p| {
                p.post_events(events.into_iter());
            });
        }
        self
    }

    /// Checks player state (without time advancement)
    ///
    /// Similar to `view()`, but semantically more explicitly indicates this is a static state check.
    ///
    /// # Parameters
    ///
    /// * `check` - State check closure, receives a reference to the player
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
    /// # ;
    /// ```
    #[must_use]
    pub fn check<F>(self, check: F) -> Self
    where
        F: Fn(&UniversalChartPlayer<R>) + 'static,
    {
        if let Some(step) = self.driver.time_steps.last_mut() {
            step.add_state_assertion(check);
        }
        self
    }

    /// Completes the current step, returning a reference to the main driver
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bms_rs::chart_process::player::UniversalChartPlayer;
    /// # use bms_rs::chart_process::resource::HashMapResourceMapping;
    /// # use crate::tests::chart_process::dsl::TestPlayerDriver;
    /// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
    /// TestPlayerDriver::new(player)
    ///     .past(gametime::TimeSpan::SECOND)
    ///         .view(|p| { /* ... */ })
    ///     .then()
    ///     .run();
    /// ```
    #[must_use]
    pub const fn then(self) -> &'a mut TestPlayerDriver<R> {
        self.driver
    }

    /// Completes the current step and returns driver (for chaining control operations)
    ///
    /// Similar to `then()`, but semantically more explicitly indicates that what follows
    /// are control operations rather than time advancements.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bms_rs::chart_process::player::UniversalChartPlayer;
    /// # use bms_rs::chart_process::resource::HashMapResourceMapping;
    /// # use crate::tests::chart_process::dsl::TestPlayerDriver;
    /// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
    /// TestPlayerDriver::new(player)
    ///     .past_sec(5)
    ///         .view(|p| { /* ... */ })
    ///     .then_action()
    ///     .past_sec(2)
    ///     .run();
    /// ```
    #[must_use]
    pub const fn then_action(self) -> &'a mut TestPlayerDriver<R> {
        self.driver
    }

    /// Chain: directly enter the next time advancement step
    ///
    /// # Parameters
    ///
    /// * `duration` - Duration of the next time advancement
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
    ///     .past(gametime::TimeSpan::SECOND)
    ///         .view(|p| assert_eq!(p.current_bpm(), &Decimal::from(120)))
    ///     .past(gametime::TimeSpan::SECOND)
    ///         .view(|p| assert_eq!(p.current_bpm(), &Decimal::from(180)))
    ///     .then()
    ///     .run();
    /// ```
    #[must_use]
    pub fn past(self, duration: TimeSpan) -> Self {
        self.driver.past(duration)
    }

    /// Convenience method: advance by specified milliseconds
    #[must_use]
    pub fn past_ms(self, millis: i64) -> Self {
        self.past(TimeSpan::MILLISECOND * millis)
    }

    /// Convenience method: advance by specified seconds
    #[must_use]
    pub fn past_sec(self, secs: i64) -> Self {
        self.past(TimeSpan::SECOND * secs)
    }

    /// Runs the test (shortcut, automatically completes current step and runs)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use bms_rs::chart_process::player::UniversalChartPlayer;
    /// # use bms_rs::chart_process::resource::HashMapResourceMapping;
    /// # use crate::tests::chart_process::dsl::TestPlayerDriver;
    /// # let player = unsafe { std::mem::zeroed::<UniversalChartPlayer<HashMapResourceMapping>>() };
    /// TestPlayerDriver::new(player)
    ///     .past(gametime::TimeSpan::SECOND)
    ///     .view(|p| { /* ... */ })
    ///     .run();
    /// ```
    pub fn run(self) {
        // Complete current step and get mutable reference to driver
        self.then().run_mut();
    }
}
