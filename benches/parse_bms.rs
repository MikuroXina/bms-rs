//! Benchmark for `BMS` file parsing.

use bms_rs::bms::{default_config, parse_bms};
use criterion::{Criterion, Throughput};

struct BmsFile {
    name: String,
    source: String,
}

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

fn main() {
    let mut criterion = Criterion::default();
    bench_parse_bms(&mut criterion);
}
