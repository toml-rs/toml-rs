#!/bin/bash

set -eux

build_type=${1:-"release"}
build_args="--release"
if [[ "$build_type" =~ "dev" ]]; then
    build_type="debug"
    build_args="--dev"
fi

cargo fuzz build $build_args --verbose
cp "fuzz/target/x86_64-unknown-linux-gnu/$build_type/fuzz_from_to_string" $OUT/fuzz_from_to_string
wget https://raw.githubusercontent.com/google/fuzzing/master/dictionaries/toml.dict -O $OUT/fuzz_from_to_string.dict
zip -r $OUT/fuzz_from_to_string_seed_corpus.zip test-suite/tests
