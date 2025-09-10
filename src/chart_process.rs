//! Chart Processor Trait

pub mod bms_processor;

use std::{collections::HashMap, path::Path};

/// Chart Processor Trait
pub trait ChartProcessor {
    /// Get Audio files with their ids
    fn audio_files(&self) -> HashMap<usize, &Path>;
    /// Get BMP (a.k.a. Graphics) files with their ids
    fn bmp_files(&self) -> HashMap<usize, &Path>;
}
