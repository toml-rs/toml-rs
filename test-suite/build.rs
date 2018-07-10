extern crate rustc_version;
use rustc_version::{version, Version};

fn main() {
    if version().unwrap() >= Version::parse("1.20.0").unwrap() {
        println!(r#"cargo:rustc-cfg=feature="test-quoted-keys-in-macro""#);
        println!(r#"cargo:rustc-cfg=feature="test-nan-sign""#);
    }
}
