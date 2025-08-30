//! Fancy diagnostics support using `ariadne`.
//!
//! 本模块在不修改现有错误类型定义的前提下，为携带 `SourcePosMixin` 的错误
//! （例如 `LexWarningWithPos`、`ParseWarningWithPos`、`AstBuildWarningWithPos`、
//! `AstParseWarningWithPos` 以及聚合的 `BmsWarning`）提供转为 `ariadne::Report`
//! 的便捷方法。
//!
//! 由于 `SourcePosMixin` 仅包含行列信息（1-based），为了构造 `ariadne` 需要的
//! 字节范围，本模块提供了从行列到字节偏移的转换工具，按行定位整行范围。
//!
//! 使用方式：
//! - 启用 `diagnostics` feature。
//! - 使用 [`ToAriadne`] 扩展 trait 将错误转换为 `ariadne` 报告，然后 `finish().print(...)`。

use crate::bms::{
    ast::{AstBuildWarningWithPos, AstParseWarning, AstParseWarningWithPos},
    lex::LexWarningWithPos,
    parse::ParseWarningWithPos,
    BmsWarning,
};

use ariadne::{Color, Label, Report, ReportKind, Source};

/// 简单的源映射，支持按行将 `(row, column)` 转成字节偏移。
/// 为避免与现有结构冲突，独立实现且仅在 `diagnostics` 启用时使用。
pub struct SimpleSource<'a> {
    name: &'a str,
    text: &'a str,
    /// 每行的起始字节偏移（包含虚拟第0行起点0），长度为 `lines + 1`，末尾为 `text.len()`。
    line_starts: Vec<usize>,
}

impl<'a> SimpleSource<'a> {
    pub fn new(name: &'a str, text: &'a str) -> Self {
        let mut line_starts = Vec::with_capacity(text.lines().count() + 2);
        line_starts.push(0);
        let mut acc = 0usize;
        for line in text.split_inclusive(['\n']) {
            acc += line.len();
            line_starts.push(acc);
        }
        if *line_starts.last().unwrap_or(&0) != text.len() {
            line_starts.push(text.len());
        }
        Self { name, text, line_starts }
    }

    fn line_start(&self, row1: usize) -> usize {
        // row1 从1开始；越界时钳制
        let idx = row1.saturating_sub(1).min(self.line_starts.len().saturating_sub(2));
        self.line_starts[idx]
    }

    fn line_end(&self, row1: usize) -> usize {
        let idx = row1.min(self.line_starts.len().saturating_sub(1));
        self.line_starts[idx]
    }

    /// 将 1-based (row, col) 转换成字节偏移，列按字符计数并在行范围内钳制。
    fn offset_of(&self, row1: usize, col1: usize) -> usize {
        let start = self.line_start(row1);
        let end = self.line_end(row1);
        let line = &self.text[start..end];
        let mut char_count = 0usize;
        let mut byte_off = 0usize;
        for (i, ch) in line.char_indices() {
            char_count += 1;
            if char_count >= col1 { byte_off = i; break; }
            byte_off = i + ch.len_utf8();
        }
        start + if col1 <= 1 { 0 } else { byte_off }
    }

    /// 一行的字节范围。
    fn line_span(&self, row1: usize) -> std::ops::Range<usize> {
        self.line_start(row1)..self.line_end(row1)
    }
}

/// 将带位置的错误转换为 `ariadne::Report`。
pub trait ToAriadne {
    fn to_report<'a>(&self, src: &SimpleSource<'a>) -> Report<'a, (String, std::ops::Range<usize>)>;
}

impl ToAriadne for LexWarningWithPos {
    fn to_report<'a>(&self, src: &SimpleSource<'a>) -> Report<'a, (String, std::ops::Range<usize>)> {
        let (row, col) = self.as_pos();
        let span = src.line_span(row);
        Report::build(ReportKind::Warning, src.name.to_string(), span.start)
            .with_message("lex: ".to_string() + &self.content().to_string())
            .with_label(Label::new((src.name.to_string(), span.clone()))
                .with_message(format!("位置 {}:{}", row, col))
                .with_color(Color::Yellow))
            .finish()
    }
}

impl ToAriadne for ParseWarningWithPos {
    fn to_report<'a>(&self, src: &SimpleSource<'a>) -> Report<'a, (String, std::ops::Range<usize>)> {
        let (row, col) = self.as_pos();
        let span = src.line_span(row);
        Report::build(ReportKind::Warning, src.name.to_string(), span.start)
            .with_message("parse: ".to_string() + &self.content().to_string())
            .with_label(Label::new((src.name.to_string(), span.clone()))
                .with_message(format!("位置 {}:{}", row, col))
                .with_color(Color::Blue))
            .finish()
    }
}

impl ToAriadne for AstBuildWarningWithPos {
    fn to_report<'a>(&self, src: &SimpleSource<'a>) -> Report<'a, (String, std::ops::Range<usize>)> {
        let (row, col) = self.as_pos();
        let span = src.line_span(row);
        Report::build(ReportKind::Warning, src.name.to_string(), span.start)
            .with_message("ast_build: ".to_string() + &self.content().to_string())
            .with_label(Label::new((src.name.to_string(), span.clone()))
                .with_message(format!("位置 {}:{}", row, col))
                .with_color(Color::Cyan))
            .finish()
    }
}

impl ToAriadne for AstParseWarningWithPos {
    fn to_report<'a>(&self, src: &SimpleSource<'a>) -> Report<'a, (String, std::ops::Range<usize>)> {
        let (row, col) = self.as_pos();
        let span = src.line_span(row);

        // AstParseWarning 内部有嵌套的 SourcePosMixin<RangeInclusive<BigUint>>，但其自身也有顶层位置。
        // 我们使用顶层位置标注整行，并在消息中追加 expected/actual 的说明。
        let details = match self.content() {
            AstParseWarning::RandomGeneratedValueOutOfRange { expected, actual }
            | AstParseWarning::SwitchGeneratedValueOutOfRange { expected, actual } => {
                format!("expected {:?}, got {}", expected.content(), actual)
            }
        };

        Report::build(ReportKind::Warning, src.name.to_string(), span.start)
            .with_message(format!("ast_parse: {} ({})", self.content(), details))
            .with_label(Label::new((src.name.to_string(), span.clone()))
                .with_message(format!("位置 {}:{}", row, col))
                .with_color(Color::Magenta))
            .finish()
    }
}

impl ToAriadne for BmsWarning {
    fn to_report<'a>(&self, src: &SimpleSource<'a>) -> Report<'a, (String, std::ops::Range<usize>)> {
        use BmsWarning::*;
        match self {
            Lex(e) => e.to_report(src),
            AstBuild(e) => e.to_report(src),
            AstParse(e) => e.to_report(src),
            Parse(e) => e.to_report(src),
            // PlayingWarning / PlayingError 没有位置，定位到文件起始 0..0
            PlayingWarning(w) => {
                let span = 0..0;
                Report::build(ReportKind::Warning, src.name.to_string(), 0)
                    .with_message(format!("playing warning: {}", w))
                    .with_label(Label::new((src.name.to_string(), span)))
                    .finish()
            }
            PlayingError(e) => {
                let span = 0..0;
                Report::build(ReportKind::Error, src.name.to_string(), 0)
                    .with_message(format!("playing error: {}", e))
                    .with_label(Label::new((src.name.to_string(), span)))
                    .finish()
            }
        }
    }
}

/// 便捷方法：批量渲染 `BmsWarning` 列表。
pub fn emit_bms_warnings<'a>(
    name: &'a str,
    source: &'a str,
    warnings: impl IntoIterator<Item = &'a BmsWarning>,
) {
    let simple = SimpleSource::new(name, source);
    let ariadne_source = Source::from(source);
    for w in warnings {
        let report = w.to_report(&simple);
        let _ = report.print((name.to_string(), ariadne_source.clone()));
    }
}


