//! Benchmark for `BMSON` file parsing and chart conversion.

use bms_rs::{
    bmson::{Bmson, parse_bmson},
    chart_process::processor::bmson::BmsonProcessor,
};
use criterion::{Criterion, Throughput};
use std::{collections::BTreeMap, sync::LazyLock};

struct BmsonFile {
    name: String,
    source: String,
}

type ParsedBmsonCharts = BTreeMap<String, Bmson<'static>>;

fn scan_bmson_files() -> Vec<BmsonFile> {
    let dir = "tests/bmson/files";
    let extensions = [".bmson"];

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

            Some(BmsonFile { name, source })
        })
        .collect()
}

fn load_bmson_charts() -> ParsedBmsonCharts {
    let files = scan_bmson_files();

    files
        .into_iter()
        .map(|file| {
            // Leak the source to extend lifetime to 'static for benchmark caching
            let leaked_source: &'static str = Box::leak(file.source.into_boxed_str());
            let bmson = parse_bmson(leaked_source)
                .bmson
                .expect("Failed to parse BMSON");

            (file.name, bmson)
        })
        .collect()
}

fn bench_parse_bmson(c: &mut Criterion) {
    let files = scan_bmson_files();
    let mut group = c.benchmark_group("parse_bmson");

    for file in files.iter() {
        group.throughput(Throughput::Bytes(file.source.len() as u64));
        group.bench_function(&file.name, |b| {
            b.iter(|| parse_bmson(std::hint::black_box(&file.source)));
        });
    }

    group.finish();
}

fn bench_bmson_to_chart(c: &mut Criterion) {
    let mut group = c.benchmark_group("bmson_to_chart");

    for (name, chart) in PARSED_CHARTS.iter() {
        group.bench_function(name, |b| {
            b.iter(|| BmsonProcessor::parse(std::hint::black_box(chart)));
        });
    }

    group.finish();
}

static PARSED_CHARTS: LazyLock<ParsedBmsonCharts> = LazyLock::new(load_bmson_charts);

fn main() {
    let mut criterion = Criterion::default();
    bench_parse_bmson(&mut criterion);
    bench_bmson_to_chart(&mut criterion);
}
