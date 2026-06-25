use criterion::{black_box, criterion_group, criterion_main, Criterion};
use brc::brc;

pub fn criterion_benchmark(c: &mut Criterion) {
    let file: &str = "create_measurements/measurements.txt";
    c.bench_function("brc", |b| b.iter(|| brc(black_box(file))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);