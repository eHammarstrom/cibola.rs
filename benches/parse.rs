#[macro_use]
extern crate criterion;

use criterion::black_box;
use criterion::Benchmark;
use criterion::Criterion;
use criterion::Throughput;

use std::fs::File;
use std::io::prelude::*;

use cibola::json::JSON;

fn parse_json(path: &'static str) {
    let mut f = File::open(path).unwrap();

    let mut txt = String::new();

    let _ = f.read_to_string(&mut txt).unwrap();

    if let Err(e) = JSON::parse(&txt) {
        panic!("Simple failed with: {}", e);
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let b = Benchmark::new("canada", |b| {
        b.iter(|| parse_json(black_box("tests/canada.json")))
    });

    let b = b.throughput(Throughput::Bytes(2251051)).sample_size(50);

    c.bench("parsing", b);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
