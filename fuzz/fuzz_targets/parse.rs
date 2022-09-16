#![no_main]
use libfuzzer_sys::fuzz_target;
use toml::Value;

fuzz_target!(|data: &[u8]| {
    let toml = String::from_utf8_lossy(data);
    _ = toml.parse::<Value>();
});
