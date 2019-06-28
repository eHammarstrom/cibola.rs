#[macro_use]
extern crate criterion;

use criterion::black_box;
use criterion::Benchmark;
use criterion::Criterion;
use criterion::Throughput;

use std::fs::File;
use std::io::prelude::*;

use cibola::json::JSON;

use json;

use serde_json;

fn file_to_str(path: &'static str) -> String {
    let mut f = File::open(path).unwrap();

    let mut txt = String::new();

    let _ = f.read_to_string(&mut txt).unwrap();

    txt
}

fn parse_json(path: &'static str) {
    let txt = file_to_str(path);

    if let Err(e) = JSON::parse(&txt) {
        panic!("Cibola failed with: {}", e);
    }
}

fn json_rust_reference_parser(path: &'static str) {
    let txt = file_to_str(path);

    if let Err(e) = json::parse(&txt) {
        panic!("json-rust failed with: {}", e);
    }
}

fn serde_json_reference_parser(path: &'static str) {
    let txt = file_to_str(path);

    let res: serde_json::Result<serde_json::Value> = serde_json::from_str(&txt);

    if let Err(e) = res {
        panic!("serde_json failed with: {}", e);
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let b = Benchmark::new("CIBOLA::canada", |b| {
        b.iter(|| parse_json(black_box("tests/canada.json")))
    })
    .with_function("serde_json::canada", |b| {
        b.iter(|| serde_json_reference_parser(black_box("tests/canada.json")))
    })
    .with_function("json-rust::canada", |b| {
        b.iter(|| json_rust_reference_parser(black_box("tests/canada.json")))
    });

    let b = b.throughput(Throughput::Bytes(2251051)).sample_size(20);

    c.bench("parsing", b);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
