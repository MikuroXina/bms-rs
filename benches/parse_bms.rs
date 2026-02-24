//! Benchmark for `BMS` file parsing and chart conversion.

use bms_rs::{
    bms::{default_config, parse_bms},
    chart_process::processor::bms::BmsProcessor,
};
use criterion::{Criterion, Throughput};
use std::{collections::BTreeMap, sync::LazyLock};

struct BmsFile {
    name: String,
    source: String,
}

type ParsedBmsCharts = BTreeMap<String, bms_rs::bms::model::Bms>;

fn scan_bms_files() -> Vec<BmsFile> {
    let dir = "tests/bms/files";
    let extensions = [".bms", ".bme"];

    std::fs::read_dir(dir)
        .expect("Failed to read directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && extensions
                    .iter()
                    .any(|ext| path.to_string_lossy().ends_with(ext))
        })
        .filter_map(|path| {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)?;

            let source = std::fs::read_to_string(&path).expect("Failed to load test file");

            Some(BmsFile { name, source })
        })
        .collect()
}

fn load_bms_charts() -> ParsedBmsCharts {
    let files = scan_bms_files();

    files
        .into_iter()
        .map(|file| {
            let bms = parse_bms(&file.source, default_config())
                .bms
                .expect("Failed to parse BMS");

            (file.name, bms)
        })
        .collect()
}

fn bench_parse_bms(c: &mut Criterion) {
    let files = scan_bms_files();
    let mut group = c.benchmark_group("parse_bms");

    for file in files.iter() {
        group.throughput(Throughput::Bytes(file.source.len() as u64));
        group.bench_function(&file.name, |b| {
            b.iter(|| {
                parse_bms(
                    std::hint::black_box(&file.source),
                    std::hint::black_box(default_config()),
                )
            });
        });
    }

    group.finish();
}

fn bench_bms_to_chart(c: &mut Criterion) {
    let mut group = c.benchmark_group("bms_to_chart");

    for (name, chart) in PARSED_CHARTS.iter() {
        group.bench_function(name, |b| {
            b.iter(|| {
                BmsProcessor::parse::<bms_rs::bms::command::channel::mapper::KeyLayoutBeat>(
                    std::hint::black_box(chart),
                )
            });
        });
    }

    group.finish();
}

static PARSED_CHARTS: LazyLock<ParsedBmsCharts> = LazyLock::new(load_bms_charts);

fn main() {
    let mut criterion = Criterion::default();
    bench_parse_bms(&mut criterion);
    bench_bms_to_chart(&mut criterion);
}
