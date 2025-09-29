//! Chart Process 模块的 Prelude
//!
//! 这个模块提供了 chart_process 模块中常用的类型和 trait 的重新导出，
//! 方便用户一次性导入所有需要的项目。

// 重新导出类型
pub use super::types::{BmpId, DisplayRatio, WavId, YCoordinate};

// 重新导出事件类型
pub use super::{ChartEvent, ControlEvent};

// 重新导出 trait
pub use super::ChartProcessor;

// 重新导出来自 bms 模块的常用类型
pub use crate::bms::prelude::{BgaLayer, Key, NoteKind, PlayerSide};

#[cfg(feature = "minor-command")]
pub use crate::bms::prelude::SwBgaEvent;

// 重新导出来自 bms_processor 模块的 BmsProcessor
pub use super::bms_processor::BmsProcessor;

// 重新导出来自 bmson_processor 模块的 BmsonProcessor
pub use super::bmson_processor::BmsonProcessor;
