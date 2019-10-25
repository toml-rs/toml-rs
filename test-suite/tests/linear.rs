use std::time::{Duration, Instant};
use toml::Value;

const TOLERANCE: f64 = 2.0;

fn measure_time(entries: usize, f: impl Fn(usize) -> String) -> Duration {
    let start = Instant::now();
    let mut s = String::new();
    for i in 0..entries {
        s += &f(i);
        s += "entry = 42\n"
    }
    s.parse::<Value>().unwrap();
    Instant::now() - start
}

#[test]
fn linear_increase_map() {
    let time_1 = measure_time(100, |i| format!("[header_no_{}]\n", i));
    let time_4 = measure_time(400, |i| format!("[header_no_{}]\n", i));
    dbg!(time_1, time_4);
    // Now ensure that the deserialization time has increased linearly
    // (within a tolerance interval) instead of, say, quadratically
    assert!(time_4 > time_1.mul_f64(4.0 - TOLERANCE));
    assert!(time_4 < time_1.mul_f64(4.0 + TOLERANCE));
}

#[test]
fn linear_increase_array() {
    let time_1 = measure_time(100, |i| format!("[[header_no_{}]]\n", i));
    let time_4 = measure_time(400, |i| format!("[[header_no_{}]]\n", i));
    dbg!(time_1, time_4);
    // Now ensure that the deserialization time has increased linearly
    // (within a tolerance interval) instead of, say, quadratically
    assert!(time_4 > time_1.mul_f64(4.0 - TOLERANCE));
    assert!(time_4 < time_1.mul_f64(4.0 + TOLERANCE));
}
