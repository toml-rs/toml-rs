#![no_main]
use libfuzzer_sys::fuzz_target;
extern crate toml;

fuzz_target!(|data: &[u8]| {
    if let Ok(data) = toml::from_slice::<toml::Value>(data) {
        let s = toml::to_string(&data).unwrap();
        let v: toml::Value = toml::from_str(&s).unwrap();
        let t = toml::to_string(&v).unwrap();
        assert_eq!(s, t);
    }
});
