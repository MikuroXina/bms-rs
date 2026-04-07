//! # Chart Processor
//!
//! ## Y Coordinate Definition
//!
//! - In the default 4/4 time signature, the length of "one measure" is 1.
//! - BMS: When the section length is the default value, each `Track` has a length of 1. The section length comes from the `#XXX02:V` message per measure, where `V` represents the multiple of the default length (e.g., `#00302:0.5` means the 3rd measure has half the default length). Cumulative y is linearly converted with this multiple.
//! - BMSON: `info.resolution` is the number of pulses corresponding to a quarter note (1/4), so one measure length is `4 * resolution` pulses; all position y is normalized to measure units through `pulses / (4 * resolution)`.
//! - Speed (default 1.0): Only affects display coordinates (e.g., `visible_notes` `distance_to_hit`), that is, scales the y difference proportionally; does not change time progression and BPM values, nor the actual duration of that measure.
//!
//! ## Values and Formulas
//!
//! ### Core Constants
//!
//! - `BASE_VELOCITY_FACTOR = 1/240`: Base velocity factor (Y/sec per BPM)
//!
//! ### Visible Range Configuration
//!
//! **`VisibleRangePerBpm` creation:**
//! ```text
//! visible_range_per_bpm = reaction_time_seconds * 240 / base_bpm
//! ```
//!
//! **Simplified visible range (for display):**
//! ```text
//! visible_y_range = current_bpm * visible_range_per_bpm
//! ```
//!
//! **Full visible window (includes speed and playback ratio):**
//! ```text
//! visible_window_y = (current_speed * playback_ratio / 240) * reaction_time * base_bpm
//! ```
//!
//! This ensures events stay in visible window for exactly `reaction_time * base_bpm / current_bpm` duration.
//!
//! ### Time Progression
//!
//! **Velocity (Y units per second):**
//! ```text
//! velocity = (current_bpm / 240) * current_speed * playback_ratio
//! ```
//!
//! **Time integration (`step_to` algorithm):**
//! ```text
//! delta_y = velocity * elapsed_time_secs
//! time_to_event = distance_y / velocity
//! ```
//!
//! ### Display Coordinates
//!
//! **Display ratio (0 = judgment line, 1 = appearance position):**
//! ```text
//! display_ratio = (event_y - current_y) / visible_window_y * current_scroll
//! ```
//!
//! The value of this type is only affected by: current Y, Y visible range, and current Speed, Scroll values.
//!
//! ### Reaction Time
//!
//! **Calculate reaction time from visible range per BPM:**
//! ```text
//! reaction_time = visible_range_per_bpm / playhead_speed
//! where playhead_speed = 1/240
//! ```

pub mod event;

pub mod player;

pub mod prelude;

pub mod process;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use crate::chart::process::{AllEventsIndex, ChartResources, WavId};
use gametime::TimeSpan;
use strict_num_extended::FinF64;
use strict_num_extended::NonNegativeF64;
use strict_num_extended::PositiveF64;

use self::event::{FlowEvent, YCoordinate};

/// Maximum value for `NonNegativeF64` when overflow occurs
pub(crate) const MAX_NON_NEGATIVE_F64: NonNegativeF64 = NonNegativeF64::new_const(f64::MAX);

/// Maximum value for `FinF64` when overflow occurs
pub(crate) const MAX_FIN_F64: FinF64 = FinF64::new_const(f64::MAX);

/// Default BPM value
pub const DEFAULT_BPM: PositiveF64 = PositiveF64::new_const(120.0);

/// Default speed factor
pub const DEFAULT_SPEED: PositiveF64 = PositiveF64::ONE;

/// Playable chart data containing all precomputed information.
///
/// This structure is immutable and ready for playback. It can be used to create
/// multiple player instances. Note that this structure does NOT contain playback
/// state - playback state is managed by `ChartPlayer`.
#[derive(Debug, Clone)]
pub struct Chart {
    /// Resource file mapping.
    pub(crate) resources: ChartResources,
    /// Event index (by Y coordinate and time).
    pub(crate) events: AllEventsIndex,
    /// Flow event mapping (affects playback speed).
    pub(crate) flow_events: BTreeMap<YCoordinate, Vec<FlowEvent>>,
    /// Initial BPM.
    pub(crate) init_bpm: PositiveF64,
    /// Initial Speed (BMS-specific, BMSON defaults to 1.0).
    pub(crate) init_speed: PositiveF64,
}

impl Chart {
    /// Get resource file mapping.
    #[must_use]
    pub const fn resources(&self) -> &ChartResources {
        &self.resources
    }

    /// Get event index.
    #[must_use]
    pub const fn events(&self) -> &AllEventsIndex {
        &self.events
    }

    /// Get flow event mapping.
    #[must_use]
    pub const fn flow_events(&self) -> &BTreeMap<YCoordinate, Vec<FlowEvent>> {
        &self.flow_events
    }

    /// Get initial BPM.
    #[must_use]
    pub const fn init_bpm(&self) -> &PositiveF64 {
        &self.init_bpm
    }

    /// Get initial Speed.
    #[must_use]
    pub const fn init_speed(&self) -> &PositiveF64 {
        &self.init_speed
    }

    /// Get audio file resources (WAV ID to path mapping).
    ///
    /// This is a convenience method that directly accesses the audio files.
    /// Equivalent to `self.resources().wav_files()`.
    #[must_use]
    pub const fn audio_files(&self) -> &HashMap<WavId, PathBuf> {
        self.resources.wav_files()
    }

    /// Get BGA/BMP image resources (BMP ID to path mapping).
    ///
    /// This is a convenience method that directly accesses the image files.
    /// Equivalent to `self.resources().bmp_files()`.
    #[must_use]
    pub const fn bmp_files(&self) -> &HashMap<crate::chart::process::BmpId, PathBuf> {
        self.resources.bmp_files()
    }

    /// Create a new `Chart` from its constituent parts.
    ///
    /// This is an internal constructor used by chart processors to assemble
    /// a parsed chart from its components.
    #[must_use]
    pub(crate) const fn from_parts(
        resources: ChartResources,
        events: AllEventsIndex,
        flow_events: BTreeMap<YCoordinate, Vec<FlowEvent>>,
        init_bpm: PositiveF64,
        init_speed: PositiveF64,
    ) -> Self {
        Self {
            resources,
            events,
            flow_events,
            init_bpm,
            init_speed,
        }
    }
}
