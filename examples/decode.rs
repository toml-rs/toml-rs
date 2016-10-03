//! An example showing off the usage of `RustcDecodable` to automatically decode
//! TOML into a Rust `struct`
//!
//! Note that this works similarly with `serde` as well.

#![deny(warnings)]

extern crate toml;
extern crate rustc_serialize;

/// This is what we're going to decode into. Each field is optional, meaning
/// that it doesn't have to be present in TOML.
#[derive(Debug, RustcDecodable)]
struct Config {
    global_string: Option<String>,
    global_integer: Option<u64>,
    server: Option<ServerConfig>,
    peers: Option<Vec<PeerConfig>>,
}

/// Sub-structs are decoded from tables, so this will decode from the `[server]`
/// table.
///
/// Again, each field is optional, meaning they don't have to be present.
#[derive(Debug, RustcDecodable)]
struct ServerConfig {
    ip: Option<String>,
    port: Option<u64>,
}

#[derive(Debug, RustcDecodable)]
struct PeerConfig {
    ip: Option<String>,
    port: Option<u64>,
}

fn main() {
    let toml_str = r#"
        global_string = "test"
        global_integer = 5

        [server]
        ip = "127.0.0.1"
        port = 80

        [[peers]]
        ip = "127.0.0.1"
        port = 8080

        [[peers]]
        ip = "127.0.0.1"
    "#;

    // Use the `decode_str` convenience here to decode a TOML string directly
    // into the `Config` struct.
    //
    // Note that the errors reported here won't necessarily be the best, but you
    // can get higher fidelity errors working with `toml::Parser` directly.
    let decoded: Config = toml::decode_str(toml_str).unwrap();
    println!("{:#?}", decoded);
}
