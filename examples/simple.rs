use std::fs::File;
use std::io::prelude::*;

use cibola::json::JSON;

fn main() {
    let mut f = File::open("tests/citylots.json").unwrap();

    let mut txt = String::new();

    let _ = f.read_to_string(&mut txt).unwrap();

    if let Err(e) = JSON::parse(&txt) {
        println!("Simple failed with: {}", e);
    }
}
