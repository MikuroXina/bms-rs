//! Benchmark for `BMSON` file parsing.

use bms_rs::bmson::parse_bmson;
use criterion::{Criterion, Throughput};

struct BmsonFile {
    name: String,
    source: String,
}

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

fn main() {
    let mut criterion = Criterion::default();
    bench_parse_bmson(&mut criterion);
}
