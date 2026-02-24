//! Benchmark for `BMS` to `PlayableChart` conversion.

use bms_rs::{
    bms::{default_config, parse_bms},
    chart_process::processor::bms::BmsProcessor,
};
use criterion::Criterion;
use std::{collections::BTreeMap, sync::LazyLock};

type ParsedBmsCharts = BTreeMap<String, bms_rs::bms::model::Bms>;

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

fn load_bms_charts() -> ParsedBmsCharts {
    let names = scan_chart_files("tests/bms/files", &[".bms", ".bme"]);

    names
        .into_iter()
        .map(|name| {
            let path = std::path::Path::new("tests/bms/files");
            let path_bms = path.join(format!("{}.bms", name));
            let path_bme = path.join(format!("{}.bme", name));

            let source = if path_bms.exists() {
                std::fs::read_to_string(path_bms).expect("Failed to load .bms file")
            } else {
                std::fs::read_to_string(path_bme).expect("Failed to load .bme file")
            };

            let bms = parse_bms(&source, default_config())
                .bms
                .expect("Failed to parse BMS");

            (name, bms)
        })
        .collect()
}

static PARSED_CHARTS: LazyLock<ParsedBmsCharts> = LazyLock::new(load_bms_charts);

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

fn main() {
    let mut criterion = Criterion::default();
    bench_bms_to_chart(&mut criterion);
}
