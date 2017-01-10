# toml-rs

[![Build Status](https://travis-ci.org/alexcrichton/toml-rs.svg?branch=master)](https://travis-ci.org/alexcrichton/toml-rs)
[![Coverage Status](https://coveralls.io/repos/alexcrichton/toml-rs/badge.svg?branch=master&service=github)](https://coveralls.io/github/alexcrichton/toml-rs?branch=master)

[Documentation](http://alexcrichton.com/toml-rs)

A [TOML][toml] decoder and encoder for Rust. This library is currently compliant with
the v0.4.0 version of TOML. This library will also likely continue to stay up to
date with the TOML specification as changes happen.

[toml]: https://github.com/toml-lang/toml

```toml
# Cargo.toml
[dependencies]
toml = "0.2"
```

By default this crate supports [`rustc-serialize`] style serialization. This can
be disabled though by disabling the default feature set:

[`rustc-serialize`]: http://github.com/rust-lang/rustc-serialize

```toml
# Cargo.toml
[dependencies]
toml = { version = "0.2", default-features = false }
```

If you'd like to enable support for [serde] you can enable the `serde` feature:

[serde]: https://github.com/serde-rs/serde

```toml
# Cargo.toml
[dependencies]
toml = { version = "0.2", features = ["serde"] }
```

If you'd like to *only* support serde, you can also write:

[serde]: https://github.com/serde-rs/serde

```toml
# Cargo.toml
[dependencies]
toml = { version = "0.2", features = ["serde"], default-features = false }
```

# License

`toml-rs` is primarily distributed under the terms of both the MIT license and
the Apache License (Version 2.0), with portions covered by various BSD-like
licenses.

See LICENSE-APACHE, and LICENSE-MIT for details.
