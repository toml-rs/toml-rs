#![deny(warnings)]
extern crate toml;
extern crate rustc_serialize;

use rustc_serialize::Decodable;

#[derive(Debug,RustcDecodable)]
struct Config {
    global_string: Option<String>,
    global_integer: Option<u64>,
    server: Option<ServerConfig>,
    peers: Option<Vec<PeerConfig>>,
}

#[derive(Debug,RustcDecodable)]
struct ServerConfig {
    ip: Option<String>,
    port: Option<u64>,
}

#[derive(Debug,RustcDecodable)]
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
            port = 8081
        "#;

    let toml = toml::Parser::new(&toml_str).parse().unwrap();

    let mut decoder = toml::Decoder::new(toml::Value::Table(toml));
    let decoded = Config::decode(&mut decoder).unwrap();

    println!("{:?}", decoded);
}
