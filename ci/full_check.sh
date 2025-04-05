#!/bin/bash
set -e
set -x

# These commands correspond 1 to 1 with github actions.

{ echo; echo; echo "Run cargo fmt"; } 2> /dev/null
cargo fmt --check --all


{ echo; echo; echo "Run cargo clippy for default features"; } 2> /dev/null
cargo clippy --tests --examples

{ echo; echo; echo "Run cargo clippy without default features"; } 2> /dev/null
cargo clippy --tests --examples --no-default-features

{ echo; echo; echo "Run cargo clippy for avian"; } 2> /dev/null
cargo clippy --tests --examples --features avian

{ echo; echo; echo "Run cargo clippy for rapier"; } 2> /dev/null
cargo clippy --tests --examples --features rapier


{ echo; echo; echo "Run tests with avian"; } 2> /dev/null
LD_LIBRARY_PATH="$(rustc --print target-libdir)" cargo test --locked --workspace --doc --features bevy/x11 --features avian


{ echo; echo; echo "Run cargo doc with default features"; } 2> /dev/null
cargo doc --no-deps