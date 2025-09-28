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

/// Y 坐标的包装类型，使用任意精度十进制数。
///
/// 统一的 y 单位说明：默认 4/4 拍下一小节为 1；BMS 以 `#SECLEN` 线性换算，BMSON 以 `pulses / (4*resolution)` 归一化为小节单位。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct YCoordinate(pub Decimal);

impl YCoordinate {
    /// 创建一个新的 YCoordinate
    #[must_use]
    pub fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// 获取内部的 Decimal 值
    #[must_use]
    pub fn value(&self) -> &Decimal {
        &self.0
    }

    /// 转换为 f64（用于兼容性）
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        self.0.to_string().parse::<f64>().unwrap_or(0.0)
    }
}

impl From<Decimal> for YCoordinate {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<f64> for YCoordinate {
    fn from(value: f64) -> Self {
        use fraction::{BigUint, GenericDecimal};
        use std::str::FromStr;
        // 将 f64 转换为字符串然后解析为 Decimal
        let decimal_str = value.to_string();
        let decimal = GenericDecimal::from_str(&decimal_str).unwrap_or_else(|_| {
            // 如果解析失败，使用 0
            GenericDecimal::from(BigUint::from(0u32))
        });
        Self(decimal)
    }
}

impl std::ops::Add for YCoordinate {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for YCoordinate {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Mul for YCoordinate {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl std::ops::Div for YCoordinate {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 / rhs.0)
    }
}

/// 描述可见音符与已触发音符的最小视图。
/// 音符可见查询的最小视图。
#[derive(Debug, Clone)]
pub struct NoteView {
    /// 玩家侧（P1/P2）
    pub side: PlayerSide,
    /// 键位（含 1..=7、Scratch(1) 等）
    pub key: Key,
    /// 距离判定线的剩余位移（y 单位，>=0 表示尚未到达判定线）
    pub distance_to_hit: YCoordinate,
    /// 关联的声音资源索引（BMS 为 `#WAVxx` 映射的整数；BMSON 常为 None）
    pub wav_index: Option<usize>,
}

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
        /// 对应的声音资源索引（若有）
        wav_index: Option<usize>,
    },
    /// BGM 等非按键类触发（无有效 side/key）
    Bgm {
        /// 对应的声音资源索引（若有）
        wav_index: Option<usize>,
    },
    /// BPM 变更
    BpmChange {
        /// 新的 BPM 值（单位：每分钟拍数）
        bpm: f64,
    },
    /// Scroll 因子变更
    ScrollChange {
        /// 滚动因子（相对值）
        factor: f64,
    },
    /// Speed 因子变更
    SpeedChange {
        /// 间距因子（相对值）
        factor: f64,
    },
    /// 停止滚动事件
    Stop {
        /// 停止时长（BMS：以谱面定义的时间单位折算；BMSON：脉冲数）
        duration: f64,
    },
    /// BGA（背景动画）变化事件
    ///
    /// 当播放位置到达BGA变化时间点时触发，表示需要切换到指定的背景图像。
    /// 支持多个BGA层级：Base（基础层）、Overlay（覆盖层）、Overlay2（第二覆盖层）和Poor（失败时显示）。
    BgaChange {
        /// BGA 层级
        layer: BgaLayer,
        /// BGA/BMP 资源 ID，通过 `bmp_files()` 方法获取对应的文件路径
        bmp_index: usize,
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
}

/// 播放器控制和设置事件。
///
/// 这些事件用于控制播放器的配置参数，如反应时间和BPM基准。
/// 与图表播放相关的事件（如音符、BGM、BPM变化等）分离，以提供更清晰的API。
#[derive(Debug, Clone)]
pub enum ControlEvent {
    /// 设置：默认反应时间（秒）
    ///
    /// 反应时间是从音符出现在可见区域到到达判定线的时间。
    /// 这个时间会影响可见窗口的大小计算。
    SetDefaultReactionTime {
        /// 反应时间（秒，>0）
        seconds: f64,
    },
    /// 设置：默认绑定 BPM
    ///
    /// 这个BPM值用作速度计算的基准。
    /// 实际播放速度 = 当前BPM / 默认BPM基准
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
    fn update(&mut self, now: SystemTime) -> Vec<(YCoordinate, ChartEvent)>;

    /// 投递外部控制事件（例如设置默认反应时间/默认 BPM），将在下一次 `update` 前被消费。
    ///
    /// 这些事件用于动态调整播放器的配置参数。图表播放相关的事件（如音符、BGM等）
    /// 由 [`update`] 方法返回，不通过此方法投递。
    fn post_events(&mut self, events: &[ControlEvent]);

    /// 查询：当前可见区域中的所有音符（含其轨道与到判定线的剩余距离）。
    fn visible_notes(&mut self, now: SystemTime) -> Vec<NoteView>;
}

/// 示例：处理BGA事件的辅助函数
///
/// ```rust,ignore
/// use bms_rs::chart_process::{ChartProcessor, ChartEvent, BgaLayer};
///
/// fn handle_bga_events(processor: &mut impl ChartProcessor, now: std::time::SystemTime) {
///     let events = processor.update(now);
///
///     for (y, event) in events {
///         match event {
///             ChartEvent::BgaChange { layer, bmp_index } => {
///                 println!("BGA变化: 层级={:?}, 图片ID={}", layer, bmp_index);
///                 // 这里可以加载并显示对应的BMP文件
///                 let bmp_files = processor.bmp_files();
///                 if let Some(bmp_path) = bmp_files.get(&bmp_index) {
///                     println!("显示BGA图片: {}", bmp_path.display());
///                 }
///             }
///             ChartEvent::BgaOpacityChange { layer, opacity } => {
///                 println!("BGA不透明度变化: 层级={:?}, 不透明度={}", layer, opacity);
///                 // 调整BGA层级的不透明度
///             }
///             ChartEvent::BgaArgbChange { layer, argb } => {
///                 println!("BGA颜色变化: 层级={:?}, ARGB={:08X}", layer, argb);
///                 // 应用颜色滤镜效果
///             }
///             _ => {} // 处理其他事件类型
///         }
///     }
/// }
/// ```
impl ChartProcessor for () {
    fn audio_files(&self) -> std::collections::HashMap<usize, &std::path::Path> {
        std::collections::HashMap::new()
    }

    fn bmp_files(&self) -> std::collections::HashMap<usize, &std::path::Path> {
        std::collections::HashMap::new()
    }

    fn default_reaction_time(&self) -> std::time::Duration {
        std::time::Duration::from_millis(500)
    }

    fn default_bpm_bound(&self) -> f64 {
        120.0
    }

    fn current_bpm(&self) -> f64 {
        120.0
    }

    fn current_speed(&self) -> f64 {
        1.0
    }

    fn current_scroll(&self) -> f64 {
        1.0
    }

    fn start_play(&mut self, _now: std::time::SystemTime) {}

    fn update(
        &mut self,
        _now: std::time::SystemTime,
    ) -> Vec<(crate::chart_process::YCoordinate, ChartEvent)> {
        Vec::new()
    }

    fn post_events(&mut self, _events: &[crate::chart_process::ControlEvent]) {}

    fn visible_notes(
        &mut self,
        _now: std::time::SystemTime,
    ) -> Vec<crate::chart_process::NoteView> {
        Vec::new()
    }
}
