extern crate syntex;
extern crate serde_codegen;

use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let src = Path::new("test.rs.in");
    let dst = Path::new(&out_dir).join("test.rs");

    let mut registry = syntex::Registry::new();

    serde_codegen::register(&mut registry);
    registry.expand("", &src, &dst).unwrap();
}
