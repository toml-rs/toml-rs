use bencher::{benchmark_group, benchmark_main, black_box, Bencher};
use std::str::FromStr;
use toml::value::Datetime;
use toml::Value;

fn dt_z(bench: &mut Bencher) {
    let s = black_box("1997-09-09T09:09:09Z");
    bench.iter(|| {
        black_box(Datetime::from_str(&s).unwrap());
    })
}

fn dt_custom_tz(bench: &mut Bencher) {
    let s = black_box("1997-09-09T09:09:09-09:09");
    bench.iter(|| {
        black_box(Datetime::from_str(&s).unwrap());
    })
}

fn dt_naive(bench: &mut Bencher) {
    let s = black_box("1997-09-09T09:09:09");
    bench.iter(|| {
        black_box(Datetime::from_str(&s).unwrap());
    })
}

fn date(bench: &mut Bencher) {
    let s = black_box("1997-09-09");
    bench.iter(|| {
        black_box(Datetime::from_str(&s).unwrap());
    })
}

fn time(bench: &mut Bencher) {
    let s = black_box("09:09:09.09");
    bench.iter(|| {
        black_box(Datetime::from_str(&s).unwrap());
    })
}

fn datetimes_as_toml(bench: &mut Bencher) {
    let mut s = String::new();
    s += "d1 = 1997-09-09T09:09:09Z\n";
    s += "d2 = 1997-09-09T09:09:09-09:09\n";
    s += "d3 = 09:09:09.09\n";
    let s = black_box(s);
    bench.iter(|| {
        black_box(s.parse::<Value>().unwrap());
    })
}

benchmark_group!(
    benches,
    dt_z,
    dt_custom_tz,
    dt_naive,
    date,
    time,
    datetimes_as_toml
);
benchmark_main!(benches);
