//! Bms Processor Module.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::bms::prelude::*;
use crate::chart_process::ChartProcessor;

/// ChartProcessor of Bms files.
pub struct BmsProcessor {
    bms: Bms,
}

impl ChartProcessor for BmsProcessor {
    fn audio_files(&self) -> HashMap<usize, &Path> {
        self.bms
            .notes
            .wav_files
            .iter()
            .map(|(obj_id, path)| (obj_id.as_u16() as usize, path.as_path()))
            .collect()
    }

    fn bmp_files(&self) -> HashMap<usize, &Path> {
        self.bms
            .graphics
            .bmp_files
            .iter()
            .map(|(obj_id, bmp)| (obj_id.as_u16() as usize, bmp.file.as_path()))
            .collect()
    }
}
