#!/bin/bash

set -eux

cargo fuzz build -O
cp fuzz/target/x86_64-unknown-linux-gnu/release/fuzz_from_to_string $OUT/fuzz_from_to_string
