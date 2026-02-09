use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cmake_formatter::formatter::{format_text, FormatConfig};
use std::fs;

fn bench_format_small(c: &mut Criterion) {
    let input = fs::read_to_string("tests/corpus/generated/small_100.cmake")
        .expect("small_100.cmake must exist");
    let config = FormatConfig::default();

    c.bench_function("format_100_lines", |b| {
        b.iter(|| format_text(black_box(&input), &config))
    });
}

fn bench_format_medium(c: &mut Criterion) {
    let input = fs::read_to_string("tests/corpus/generated/medium_1k.cmake")
        .expect("medium_1k.cmake must exist");
    let config = FormatConfig::default();

    c.bench_function("format_1k_lines", |b| {
        b.iter(|| format_text(black_box(&input), &config))
    });
}

fn bench_format_large(c: &mut Criterion) {
    let input = fs::read_to_string("tests/corpus/generated/large_10k.cmake")
        .expect("large_10k.cmake must exist");
    let config = FormatConfig::default();

    // Use a larger stack for 10k lines (recursive descent parser needs it)
    c.bench_function("format_10k_lines", |b| {
        b.iter(|| {
            let input = input.clone();
            let config = config.clone();
            std::thread::Builder::new()
                .stack_size(16 * 1024 * 1024)
                .spawn(move || format_text(black_box(&input), &config))
                .unwrap()
                .join()
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_format_small, bench_format_medium, bench_format_large);
criterion_main!(benches);
