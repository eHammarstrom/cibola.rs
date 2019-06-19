use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::process;

use cibola::json::JSON;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: cibola FILE");
        process::exit(1);
    }

    let mut f = File::open(&args[1]).unwrap();

    let mut txt = String::new();

    let _ = f.read_to_string(&mut txt).unwrap();

    if let Err(e) = JSON::parse(&txt) {
        println!("Simple failed with: {}", e);
    }
}
