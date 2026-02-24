//! Benchmark for `BMSON` to `PlayableChart` conversion.

use bms_rs::{
    bmson::{Bmson, parse_bmson},
    chart_process::processor::bmson::BmsonProcessor,
};
use criterion::Criterion;
use std::{collections::BTreeMap, sync::LazyLock};

type ParsedBmsonCharts = BTreeMap<String, Bmson<'static>>;

fn scan_chart_files(dir: &str, extensions: &[&str]) -> Vec<String> {
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
            path.file_stem()
                .and_then(|name| name.to_str())
                .map(String::from)
        })
        .collect()
}

fn load_bmson_charts() -> ParsedBmsonCharts {
    let names = scan_chart_files("tests/bmson/files", &[".bmson"]);

    names
        .into_iter()
        .map(|name| {
            let path = std::path::Path::new("tests/bmson/files").join(format!("{}.bmson", name));
            let source = std::fs::read_to_string(path).expect("Failed to load test file");

            // Leak the source to extend lifetime to 'static for benchmark caching
            let leaked_source: &'static str = Box::leak(source.into_boxed_str());
            let bmson = parse_bmson(leaked_source)
                .bmson
                .expect("Failed to parse BMSON");

            (name, bmson)
        })
        .collect()
}

static PARSED_CHARTS: LazyLock<ParsedBmsonCharts> = LazyLock::new(load_bmson_charts);

fn bench_bmson_to_chart(c: &mut Criterion) {
    let mut group = c.benchmark_group("bmson_to_chart");

    for (name, chart) in PARSED_CHARTS.iter() {
        group.bench_function(name, |b| {
            b.iter(|| BmsonProcessor::parse(std::hint::black_box(chart)));
        });
    }

    group.finish();
}

fn main() {
    let mut criterion = Criterion::default();
    bench_bmson_to_chart(&mut criterion);
}
