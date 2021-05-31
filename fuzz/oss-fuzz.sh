#!/bin/bash

set -eux

cargo fuzz build -O
cp fuzz/target/x86_64-unknown-linux-gnu/release/fuzz_from_to_string $OUT/fuzz_from_to_string
wget https://raw.githubusercontent.com/google/fuzzing/master/dictionaries/toml.dict -O $OUT/fuzz_from_to_string.dict
zip -r $OUT/fuzz_from_to_string_seed_corpus.zip test-suite/tests
