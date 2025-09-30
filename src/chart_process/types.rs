//! 类型定义模块

use crate::bms::Decimal;
use crate::chart_process::ChartEvent;
use fraction::{BigUint, GenericDecimal};
use std::str::FromStr;

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

/// 显示比例的包装类型，表示 note 实际在显示区域中的位置。
///
/// 0 为判定线，1 为一般情况下 note 刚开始出现的位置。
/// 这个类型的值只会受到：当前Y、Y可见范围和当前Speed、Scroll值这些因素的影响。
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct DisplayRatio(pub Decimal);

impl DisplayRatio {
    /// 创建一个新的 DisplayRatio
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

    /// 创建表示判定线的 DisplayRatio（值为 0）
    #[must_use]
    pub fn at_judgment_line() -> Self {
        Self(Decimal::from(0))
    }

    /// 创建表示 note 刚开始出现位置的 DisplayRatio（值为 1）
    #[must_use]
    pub fn at_appearance() -> Self {
        Self(Decimal::from(1))
    }
}

impl From<Decimal> for DisplayRatio {
    fn from(value: Decimal) -> Self {
        Self(value)
    }
}

impl From<f64> for DisplayRatio {
    fn from(value: f64) -> Self {
        // 将 f64 转换为字符串然后解析为 Decimal
        let decimal_str = value.to_string();
        let decimal = GenericDecimal::from_str(&decimal_str).unwrap_or_else(|_| {
            // 如果解析失败，使用 0
            GenericDecimal::from(BigUint::from(0u32))
        });
        Self(decimal)
    }
}

/// WAV音频文件ID的包装类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WavId(pub usize);

impl WavId {
    /// 创建一个新的WavId
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    /// 获取内部的usize值
    #[must_use]
    pub const fn value(self) -> usize {
        self.0
    }
}

impl From<usize> for WavId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<WavId> for usize {
    fn from(id: WavId) -> Self {
        id.0
    }
}

/// BMP/BGA图像文件ID的包装类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BmpId(pub usize);

impl BmpId {
    /// 创建一个新的BmpId
    #[must_use]
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    /// 获取内部的usize值
    #[must_use]
    pub const fn value(self) -> usize {
        self.0
    }
}

impl From<usize> for BmpId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<BmpId> for usize {
    fn from(id: BmpId) -> Self {
        id.0
    }
}

/// 时间轴事件及其位置的包装类型。
///
/// 表示图表播放过程中的一个事件及其在时间轴上的位置。
#[derive(Debug, Clone)]
pub struct ChartEventWithPosition {
    /// 事件在时间轴上的位置（y坐标）
    pub position: YCoordinate,
    /// 图表事件
    pub event: ChartEvent,
}

impl ChartEventWithPosition {
    /// 创建一个新的 ChartEventWithPosition
    #[must_use]
    pub fn new(position: YCoordinate, event: ChartEvent) -> Self {
        Self { position, event }
    }

    /// 获取事件位置
    #[must_use]
    pub fn position(&self) -> &YCoordinate {
        &self.position
    }

    /// 获取图表事件
    #[must_use]
    pub fn event(&self) -> &ChartEvent {
        &self.event
    }

    /// 解构为元组
    #[must_use]
    pub fn into_tuple(self) -> (YCoordinate, ChartEvent) {
        (self.position, self.event)
    }
}

impl From<(YCoordinate, ChartEvent)> for ChartEventWithPosition {
    fn from((position, event): (YCoordinate, ChartEvent)) -> Self {
        Self::new(position, event)
    }
}

impl From<ChartEventWithPosition> for (YCoordinate, ChartEvent) {
    fn from(wrapper: ChartEventWithPosition) -> Self {
        wrapper.into_tuple()
    }
}

/// 可见区域事件及其位置和显示比例的包装类型。
///
/// 表示在可见区域中的一个事件，包括其位置、事件内容和显示比例。
#[derive(Debug, Clone)]
pub struct VisibleEvent {
    /// 事件在时间轴上的位置（y坐标）
    pub position: YCoordinate,
    /// 图表事件
    pub event: ChartEvent,
    /// 显示比例
    pub display_ratio: DisplayRatio,
}

impl VisibleEvent {
    /// 创建一个新的 VisibleEvent
    #[must_use]
    pub fn new(position: YCoordinate, event: ChartEvent, display_ratio: DisplayRatio) -> Self {
        Self {
            position,
            event,
            display_ratio,
        }
    }

    /// 获取事件位置
    #[must_use]
    pub fn position(&self) -> &YCoordinate {
        &self.position
    }

    /// 获取图表事件
    #[must_use]
    pub fn event(&self) -> &ChartEvent {
        &self.event
    }

    /// 获取显示比例
    #[must_use]
    pub fn display_ratio(&self) -> &DisplayRatio {
        &self.display_ratio
    }

    /// 解构为元组
    #[must_use]
    pub fn into_tuple(self) -> (YCoordinate, ChartEvent, DisplayRatio) {
        (self.position, self.event, self.display_ratio)
    }
}

impl From<(YCoordinate, ChartEvent, DisplayRatio)> for VisibleEvent {
    fn from((position, event, display_ratio): (YCoordinate, ChartEvent, DisplayRatio)) -> Self {
        Self::new(position, event, display_ratio)
    }
}

impl From<VisibleEvent> for (YCoordinate, ChartEvent, DisplayRatio) {
    fn from(wrapper: VisibleEvent) -> Self {
        wrapper.into_tuple()
    }
}
