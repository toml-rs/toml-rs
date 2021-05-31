#!/bin/bash

set -eux

cargo fuzz build -O
wget https://raw.githubusercontent.com/google/fuzzing/master/dictionaries/toml.dict -O $OUT/fuzz_from_to_string.dict
cp fuzz/target/x86_64-unknown-linux-gnu/release/fuzz_from_to_string $OUT/fuzz_from_to_string
