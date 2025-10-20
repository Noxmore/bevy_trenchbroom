#!/bin/bash
set -e
set -x

# These commands correspond 1 to 1 with github actions.

{ echo; echo; echo "Run cargo fmt"; } 2> /dev/null
cargo fmt --check --all


{ echo; echo; echo "Run cargo clippy for default features"; } 2> /dev/null
cargo clippy --workspace --all-targets

{ echo; echo; echo "Run cargo clippy without default features"; } 2> /dev/null
cargo clippy --workspace --all-targets --no-default-features

{ echo; echo; echo "Run cargo clippy for bsps"; } 2> /dev/null
cargo clippy --workspace --all-targets --features bsp


{ echo; echo; echo "Run tests"; } 2> /dev/null
cargo test --locked --workspace --features bevy/x11 --features client,bsp
LD_LIBRARY_PATH="$(rustc --print target-libdir)" cargo test --locked --workspace --doc --features bevy/x11 --features client,bsp


{ echo; echo; echo "Run cargo doc with default features"; } 2> /dev/null
cargo doc --no-deps --workspace
