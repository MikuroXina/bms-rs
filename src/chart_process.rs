//! Chart Processor
//!
//! y 坐标统一定义：
//! - 在默认 4/4 拍的情况下，“一小节”的长度为 1。
//! - BMS：当节长为默认值时，每个 `Track` 的长度为 1。节长来自每小节的 `#XXX02:V` 消息，其中 `V` 表示默认长度的倍数（例如 `#00302:0.5` 表示第 3 小节长度为默认的一半）。累计 y 以该倍数线性换算。
//! - BMSON：`info.resolution` 是四分音符（1/4）对应的脉冲数，因而一小节长度为 `4 * resolution` 脉冲；所有位置 y 通过 `pulses / (4 * resolution)` 归一化为小节单位。
//! - Speed（默认 1.0）：仅影响显示坐标（例如 `visible_notes` 的 `distance_to_hit`），即对 y 差值做比例缩放；不改变时间推进与 BPM 值，也不改变该小节的实际持续时间。

use crate::bms::{
    Decimal,
    prelude::{BgaLayer, Key, NoteKind, PlayerSide},
};

#[cfg(feature = "minor-command")]
use crate::bms::prelude::SwBgaEvent;

pub mod bms_processor;
#[cfg(feature = "bmson")]
pub mod bmson_processor;
#[cfg(not(feature = "bmson"))]
pub mod bmson_processor {}

use std::{collections::HashMap, path::Path, time::SystemTime};

// 类型定义模块
pub mod types;

// Prelude 模块
pub mod prelude;

// 使用 prelude 中的类型
pub use prelude::{BmpId, DisplayRatio, WavId, YCoordinate};

/// 播放过程中产生的事件（Elm 风格）。
///
/// 这些事件代表图表播放过程中的实际事件，如音符触发、BGM播放、
/// BPM变化等。设置和控制相关的事件已分离到 [`ControlEvent`]。
#[derive(Debug, Clone)]
pub enum ChartEvent {
    /// 按键音符到达判定线（包含可见、长条、地雷、不可见等，通过 `kind` 区分）
    Note {
        /// 玩家侧
        side: PlayerSide,
        /// 键位
        key: Key,
        /// 音符类型（`NoteKind`）
        kind: NoteKind,
        /// 对应的声音资源ID（若有）
        wav_id: Option<WavId>,
        /// 音符长度（长条音符的结束位置，普通音符为 None）
        length: Option<YCoordinate>,
        /// 音符继续播放标志（BMS固定为false，Bmson依据Note.c字段）
        continue_play: bool,
    },
    /// BGM 等非按键类触发（无有效 side/key）
    Bgm {
        /// 对应的声音资源ID（若有）
        wav_id: Option<WavId>,
    },
    /// BPM 变更
    BpmChange {
        /// 新的 BPM 值（单位：每分钟拍数）
        bpm: Decimal,
    },
    /// Scroll 因子变更
    ScrollChange {
        /// 滚动因子（相对值）
        factor: Decimal,
    },
    /// Speed 因子变更
    SpeedChange {
        /// 间距因子（相对值）
        factor: Decimal,
    },
    /// 停止滚动事件
    Stop {
        /// 停止时长（BMS：以谱面定义的时间单位折算；BMSON：脉冲数）
        duration: Decimal,
    },
    /// BGA（背景动画）变化事件
    ///
    /// 当播放位置到达BGA变化时间点时触发，表示需要切换到指定的背景图像。
    /// 支持多个BGA层级：Base（基础层）、Overlay（覆盖层）、Overlay2（第二覆盖层）和Poor（失败时显示）。
    BgaChange {
        /// BGA 层级
        layer: BgaLayer,
        /// BGA/BMP 资源 ID，通过 `bmp_files()` 方法获取对应的文件路径（若有）
        bmp_id: Option<BmpId>,
    },
    /// BGA 不透明度变化事件（需要启用 minor-command 特性）
    ///
    /// 动态调整指定BGA层级的不透明度，实现淡入淡出效果。
    #[cfg(feature = "minor-command")]
    BgaOpacityChange {
        /// BGA 层级
        layer: BgaLayer,
        /// 不透明度值 (0x01-0xFF，0x01表示几乎透明，0xFF表示完全不透明)
        opacity: u8,
    },
    /// BGA ARGB 颜色变化事件（需要启用 minor-command 特性）
    ///
    /// 动态调整指定BGA层级的颜色，通过ARGB值实现颜色滤镜效果。
    #[cfg(feature = "minor-command")]
    BgaArgbChange {
        /// BGA 层级
        layer: BgaLayer,
        /// ARGB 颜色值 (格式：0xAARRGGBB)
        argb: u32,
    },
    /// BGM 音量变化事件
    ///
    /// 当播放位置到达BGM音量变化时间点时触发，用于调整背景音乐的音量。
    BgmVolumeChange {
        /// 音量值 (0x01-0xFF，0x01表示最小音量，0xFF表示最大音量)
        volume: u8,
    },
    /// KEY 音量变化事件
    ///
    /// 当播放位置到达KEY音量变化时间点时触发，用于调整按键音效的音量。
    KeyVolumeChange {
        /// 音量值 (0x01-0xFF，0x01表示最小音量，0xFF表示最大音量)
        volume: u8,
    },
    /// 文本显示事件
    ///
    /// 当播放位置到达文本显示时间点时触发，用于在谱面中显示文本信息。
    TextDisplay {
        /// 要显示的文本内容
        text: String,
    },
    /// 判定等级变化事件
    ///
    /// 当播放位置到达判定等级变化时间点时触发，用于调整判定窗口的严格程度。
    JudgeLevelChange {
        /// 判定等级 (VeryHard, Hard, Normal, Easy, OtherInt)
        level: crate::bms::command::JudgeLevel,
    },
    /// 视频跳转事件（需要启用 minor-command 特性）
    ///
    /// 当播放位置到达视频跳转时间点时触发，用于视频播放控制。
    #[cfg(feature = "minor-command")]
    VideoSeek {
        /// 跳转到的时间点（秒）
        seek_time: f64,
    },
    /// BGA 键绑定事件（需要启用 minor-command 特性）
    ///
    /// 当播放位置到达BGA键绑定时间点时触发，用于BGA与按键的绑定控制。
    #[cfg(feature = "minor-command")]
    BgaKeybound {
        /// BGA 键绑定事件类型
        event: SwBgaEvent,
    },
    /// 选项变化事件（需要启用 minor-command 特性）
    ///
    /// 当播放位置到达选项变化时间点时触发，用于动态调整游戏选项。
    #[cfg(feature = "minor-command")]
    OptionChange {
        /// 选项内容
        option: String,
    },
    /// 小节线事件
    ///
    /// 当播放位置到达小节线位置时触发，用于谱面结构的显示。
    BarLine,
}

/// 播放器控制和设置事件。
///
/// 这些事件用于控制播放器的配置参数，如可见Y范围。
/// 与图表播放相关的事件（如音符、BGM、BPM变化等）分离，以提供更清晰的API。
#[derive(Debug, Clone)]
pub enum ControlEvent {
    /// 设置：默认可见Y范围长度
    ///
    /// 可见Y范围长度是从音符出现在可见区域到到达判定线的距离。
    /// 这个长度会影响可见窗口的大小计算。
    SetDefaultVisibleYLength {
        /// 可见Y范围长度（y坐标单位，>0）
        length: YCoordinate,
    },
}

/// 统一的 y 单位说明：默认 4/4 拍下一小节为 1；BMS 以 `#SECLEN` 线性换算，BMSON 以 `pulses / (4*resolution)` 归一化。
pub trait ChartProcessor {
    /// 读取：音频文件资源（id 到路径映射）。
    fn audio_files(&self) -> HashMap<WavId, &Path>;
    /// 读取：BGA/BMP 图像资源（id 到路径映射）。
    fn bmp_files(&self) -> HashMap<BmpId, &Path>;

    /// 读取：默认可见Y范围长度（从音符出现在可见区域到到达判定线的距离，单位：y坐标）。
    fn default_visible_y_length(&self) -> YCoordinate;

    /// 读取：当前 BPM（随事件改变）。
    fn current_bpm(&self) -> Decimal;
    /// 读取：当前 Speed 因子（随事件改变）。
    fn current_speed(&self) -> Decimal;
    /// 读取：当前 Scroll 因子（随事件改变）。
    fn current_scroll(&self) -> Decimal;

    /// 通知：开始播放，记录起始绝对时间。
    fn start_play(&mut self, now: SystemTime);

    /// 更新：推进内部时间轴，返回自上次调用以来产生的时间轴事件（Elm 风格）。
    fn update(&mut self, now: SystemTime) -> impl Iterator<Item = (YCoordinate, ChartEvent)>;

    /// 投递外部控制事件（例如设置默认反应时间/默认 BPM），将在下一次 `update` 前被消费。
    ///
    /// 这些事件用于动态调整播放器的配置参数。图表播放相关的事件（如音符、BGM等）
    /// 由 [`update`] 方法返回，不通过此方法投递。
    fn post_events(&mut self, events: &[ControlEvent]);

    /// 查询：当前可见区域中的所有事件（预先加载逻辑）。
    fn visible_events(
        &mut self,
        now: SystemTime,
    ) -> impl Iterator<Item = (YCoordinate, ChartEvent, DisplayRatio)>;
}
