//! Chart Processor
//!
//! y 坐标统一定义：
//! - 在默认 4/4 拍的情况下，“一小节”的长度为 1。
//! - BMS：当节长为默认值时，每个 `Track` 的长度为 1。节长来自每小节的 `#XXX02:V` 消息，其中 `V` 表示默认长度的倍数（例如 `#00302:0.5` 表示第 3 小节长度为默认的一半）。累计 y 以该倍数线性换算。
//! - BMSON：`info.resolution` 是四分音符（1/4）对应的脉冲数，因而一小节长度为 `4 * resolution` 脉冲；所有位置 y 通过 `pulses / (4 * resolution)` 归一化为小节单位。

use crate::bms::prelude::{Key, NoteKind, PlayerSide};

pub mod bms_processor;
#[cfg(feature = "bmson")]
pub mod bmson_processor;
#[cfg(not(feature = "bmson"))]
pub mod bmson_processor {}

use std::{
    collections::HashMap,
    path::Path,
    time::{Duration, SystemTime},
};

/// 描述可见音符与已触发音符的最小视图。
/// 音符可见查询的最小视图。
#[derive(Debug, Clone)]
pub struct NoteView {
    /// 玩家侧（P1/P2）
    pub side: PlayerSide,
    /// 键位（含 1..=7、Scratch(1) 等）
    pub key: Key,
    /// 距离判定线的剩余位移（y 单位，>=0 表示尚未到达判定线）
    pub distance_to_hit: f64,
    /// 关联的声音资源索引（BMS 为 `#WAVxx` 映射的整数；BMSON 常为 None）
    pub wav_index: Option<usize>,
}

/// 播放过程中产生的事件（Elm 风格）。
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
        /// 发生位置的累计位移（y 单位）
        y: f64,
        /// 对应的声音资源索引（若有）
        wav_index: Option<usize>,
    },
    /// BGM 等非按键类触发（无有效 side/key）
    Bgm {
        /// 发生位置的累计位移（y 单位）
        y: f64,
        /// 对应的声音资源索引（若有）
        wav_index: Option<usize>,
    },
    /// BPM 变更
    BpmChange {
        /// 事件发生的累计位移（y 单位）
        y: f64,
        /// 新的 BPM 值（单位：每分钟拍数）
        bpm: f64,
    },
    /// Scroll 因子变更
    ScrollChange {
        /// 事件发生的累计位移（y 单位）
        y: f64,
        /// 滚动因子（相对值）
        factor: f64,
    },
    /// Speed 因子变更
    SpeedChange {
        /// 事件发生的累计位移（y 单位）
        y: f64,
        /// 间距因子（相对值）
        factor: f64,
    },
    /// 停止滚动事件
    Stop {
        /// 事件发生的累计位移（y 单位）
        y: f64,
        /// 停止时长（BMS：以谱面定义的时间单位折算；BMSON：脉冲数）
        duration: f64,
    },
    /// 设置：默认反应时间（秒）
    SetDefaultReactionTime {
        /// 反应时间（秒，>0）
        seconds: f64,
    },
    /// 设置：默认绑定 BPM
    SetDefaultBpmBound {
        /// 作为默认速度基准的 BPM（>0）
        bpm: f64,
    },
}

/// 统一的 y 单位说明：默认 4/4 拍下一小节为 1；BMS 以 `#SECLEN` 线性换算，BMSON 以 `pulses / (4*resolution)` 归一化。
pub trait ChartProcessor {
    /// 读取：音频文件资源（id 到路径映射）。
    fn audio_files(&self) -> HashMap<usize, &Path>;
    /// 读取：BGA/BMP 图像资源（id 到路径映射）。
    fn bmp_files(&self) -> HashMap<usize, &Path>;

    /// 读取：默认流速下的反应时间（从音符出现在可见区域到到达判定线的时间，单位秒）。
    fn default_reaction_time(&self) -> Duration;

    /// 读取：默认流速绑定的 BPM（用于将反应时间与 BPM 关联的基准）。
    fn default_bpm_bound(&self) -> f64;

    /// 读取：当前 BPM（随事件改变）。
    fn current_bpm(&self) -> f64;
    /// 读取：当前 Speed 因子（随事件改变）。
    fn current_speed(&self) -> f64;
    /// 读取：当前 Scroll 因子（随事件改变）。
    fn current_scroll(&self) -> f64;

    /// 通知：开始播放，记录起始绝对时间。
    fn start_play(&mut self, now: SystemTime);

    /// 更新：推进内部时间轴，返回自上次调用以来产生的时间轴事件（Elm 风格）。
    fn update(&mut self, now: SystemTime) -> Vec<ChartEvent>;

    /// 投递外部事件（例如设置默认反应时间/默认 BPM），将在下一次 `update` 前被消费。
    fn post_events(&mut self, events: &[ChartEvent]);

    /// 查询：当前可见区域中的所有音符（含其轨道与到判定线的剩余距离）。
    fn visible_notes(&mut self, now: SystemTime) -> Vec<NoteView>;
}
