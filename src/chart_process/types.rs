//! 类型定义模块

use crate::bms::Decimal;
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
