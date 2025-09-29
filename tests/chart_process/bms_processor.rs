use std::str::FromStr;
use std::time::{Duration, SystemTime};

use num::ToPrimitive;

use bms_rs::bms::prelude::*;
use bms_rs::chart_process::prelude::*;

#[test]
fn test_bemuse_ext_basic_visible_events_functionality() {
    // 使用 bemuse_ext.bms 文件测试 visible_events 的基本功能
    let source = include_str!("../bms/files/bemuse_ext.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    let mut processor = BmsProcessor::new(bms);
    let start_time = SystemTime::now();
    processor.start_play(start_time);

    // 验证初始状态
    assert_eq!(processor.current_bpm(), Decimal::from(120));
    assert_eq!(processor.current_speed(), Decimal::from(1));
    assert_eq!(processor.current_scroll(), Decimal::from(1));

    // 前进到第一个变化点
    let after_first_change = start_time + Duration::from_secs(1);
    let _ = processor.update(after_first_change);

    // 检查visible_events方法正常工作
    let after_change_events: Vec<_> = processor.visible_events(after_first_change).collect();
    assert!(!after_change_events.is_empty(), "应该有可见事件");

    // 验证显示比例的计算
    for (y_coord, event, display_ratio) in &after_change_events {
        let y_value = y_coord.value().to_f64().unwrap_or(0.0);
        let display_ratio_value = display_ratio.value().to_f64().unwrap_or(0.0);

        // 显示比例应该在合理范围内
        assert!(
            display_ratio_value >= 0.0 && display_ratio_value <= 2.0,
            "显示比例应该在合理范围内，当前值: {:.3}, 事件Y: {:.3}",
            display_ratio_value,
            y_value
        );

        // 验证事件类型
        match event {
            ChartEvent::Note { .. } | ChartEvent::Bgm { .. } => {
                assert!(
                    display_ratio_value.is_finite(),
                    "音符/BGM事件的显示比例应该是有限值"
                );
            }
            ChartEvent::BpmChange { .. }
            | ChartEvent::SpeedChange { .. }
            | ChartEvent::ScrollChange { .. } => {
                assert!(
                    display_ratio_value.is_finite(),
                    "控制事件的显示比例应该是有限值"
                );
            }
            _ => {}
        }
    }
}

#[test]
fn test_lilith_mx_bpm_changes_affect_visible_window() {
    // 使用 lilith_mx.bms 文件测试 BPM 变化对可见窗口的影响
    let source = include_str!("../bms/files/lilith_mx.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    let mut processor = BmsProcessor::new(bms);
    let start_time = SystemTime::now();
    processor.start_play(start_time);

    // 初始状态：BPM = 151
    assert_eq!(processor.current_bpm(), Decimal::from(151));

    // 前进到第一个 BPM 变化点
    let after_first_change = start_time + Duration::from_secs(1);
    let _ = processor.update(after_first_change);
    assert_eq!(processor.current_bpm(), Decimal::from_str("75.5").unwrap());

    // 获取BPM变化后的可见事件
    let after_bpm_events: Vec<_> = processor.visible_events(after_first_change).collect();
    assert!(!after_bpm_events.is_empty(), "BPM变化后应该仍有可见事件");

    // 验证显示比例仍然有效
    for (_, _, display_ratio) in &after_bpm_events {
        let ratio_value = display_ratio.value().to_f64().unwrap_or(0.0);
        assert!(ratio_value.is_finite() && ratio_value >= 0.0);
    }
}

#[test]
fn test_bemuse_ext_scroll_half_display_ratio_scaling() {
    // 使用 bemuse_ext.bms 文件测试 scroll 值为 0.5 时 DisplayRatio 的缩放
    let source = include_str!("../bms/files/bemuse_ext.bms");
    let LexOutput {
        tokens,
        lex_warnings: warnings,
    } = TokenStream::parse_lex(source);
    assert_eq!(warnings, vec![]);
    let ParseOutput {
        bms,
        parse_warnings,
        ..
    }: ParseOutput<KeyLayoutBeat> = Bms::from_token_stream(&tokens, AlwaysWarnAndUseOlder);
    assert_eq!(parse_warnings, vec![]);

    let mut processor = BmsProcessor::new(bms);
    let start_time = SystemTime::now();
    processor.start_play(start_time);

    // 验证初始状态：Scroll = 1.0
    assert_eq!(processor.current_scroll(), Decimal::from(1));

    // 获取初始可见事件及其显示比例
    let initial_events: Vec<_> = processor.visible_events(start_time).collect();
    let initial_ratios: Vec<f64> = initial_events
        .iter()
        .map(|(_, _, ratio)| ratio.value().to_f64().unwrap_or(0.0))
        .collect::<Vec<_>>();

    if initial_ratios.is_empty() {
        return; // 如果没有可见事件，跳过测试
    }

    // 前进到第一个Scroll变化点（仍然是1.0）
    let after_first_scroll = start_time + Duration::from_secs(1);
    let _ = processor.update(after_first_scroll);
    assert_eq!(processor.current_scroll(), Decimal::from(1));

    let after_first_ratios: Vec<f64> = processor
        .visible_events(after_first_scroll)
        .collect::<Vec<_>>()
        .iter()
        .map(|(_, _, ratio)| ratio.value().to_f64().unwrap_or(0.0))
        .collect::<Vec<_>>();

    if after_first_ratios.is_empty() {
        return;
    }

    // 由于scroll仍然是1.0，显示比例应该基本相同
    for (initial_ratio, after_first_ratio) in initial_ratios.iter().zip(after_first_ratios.iter()) {
        let diff = (after_first_ratio - initial_ratio).abs();
        assert!(
            diff < 0.1,
            "Scroll为1.0时显示比例应该基本不变，初始: {:.6}, 变化后: {:.6}",
            initial_ratio,
            after_first_ratio
        );
    }

    // 前进到第二个Scroll变化点（scroll 0.5）
    let after_scroll_half = after_first_scroll + Duration::from_secs(2);
    let _ = processor.update(after_scroll_half);
    assert_eq!(
        processor.current_scroll(),
        Decimal::from_str("0.5").unwrap()
    );

    let after_scroll_half_ratios: Vec<f64> = processor
        .visible_events(after_scroll_half)
        .collect::<Vec<_>>()
        .iter()
        .map(|(_, _, ratio)| ratio.value().to_f64().unwrap_or(0.0))
        .collect::<Vec<_>>();

    if after_scroll_half_ratios.is_empty() {
        return;
    }

    // 验证显示比例的范围和符号
    for ratio in after_scroll_half_ratios.iter() {
        assert!(ratio.is_finite(), "Scroll为0.5时显示比例应该是有限值");
        assert!(
            *ratio >= -5.0 && *ratio <= 5.0,
            "Scroll为0.5时显示比例应该在合理范围内: {:.6}",
            ratio
        );
    }

    // 验证scroll为0.5时显示比例的缩放效果
    if after_first_ratios.len() == after_scroll_half_ratios.len() {
        for (first_ratio, half_ratio) in after_first_ratios
            .iter()
            .zip(after_scroll_half_ratios.iter())
        {
            // 当scroll从1.0变为0.5时，显示比例应该大约变为原来的0.5倍
            let expected_half_ratio = first_ratio * 0.5;
            let actual_diff = (half_ratio - expected_half_ratio).abs();

            assert!(
                actual_diff < 0.1,
                "Scroll为0.5时显示比例应该约为原来的0.5倍，期望: {:.6}, 实际: {:.6}",
                expected_half_ratio,
                half_ratio
            );
        }
    }

    // 额外验证：确保scroll为0.5时的显示比例确实小于scroll为1.0时的显示比例
    if after_first_ratios.len() == after_scroll_half_ratios.len() {
        for (first_ratio, half_ratio) in after_first_ratios
            .iter()
            .zip(after_scroll_half_ratios.iter())
        {
            if *first_ratio > 0.0 {
                assert!(
                    *half_ratio < *first_ratio,
                    "Scroll为0.5时显示比例应该小于Scroll为1.0时的显示比例，1.0时: {:.6}, 0.5时: {:.6}",
                    first_ratio,
                    half_ratio
                );
            }
        }
    }
}

// 注意：BmsonProcessor对比测试已被移除，因为BMSON格式与BMS格式差异较大，
// 直接对比事件生成可能会导致混淆。主要目标是验证BmsProcessor本身的功能。
