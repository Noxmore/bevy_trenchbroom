#!/bin/bash
set -e

echo "cargo fmt --check --all"
cargo fmt --check --all

echo "cargo clippy --tests --examples"
cargo clippy --tests --examples

echo "cargo clippy --tests --examples --no-default-features"
cargo clippy --tests --examples --no-default-features

echo "cargo clippy --tests --examples --features avian"
cargo clippy --tests --examples --features avian

echo "cargo clippy --tests --examples --features rapier"
cargo clippy --tests --examples --features rapier

echo "cargo test --workspace --doc --features bevy/x11 --features avian"
LD_LIBRARY_PATH="$(rustc --print target-libdir)" cargo test --locked --workspace --doc --features bevy/x11 --features avian

echo "cargo doc --no-deps"
cargo doc --no-deps