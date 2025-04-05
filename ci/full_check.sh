#!/bin/bash
set -e

echo "Running cargo fmt"
cargo fmt --check --all

echo "Running cargo clippy for default features"
cargo clippy --tests --examples

echo "Running cargo clippy for no default features"
cargo clippy --tests --examples --no-default-features

echo "Running cargo clippy for avian"
cargo clippy --tests --examples --features avian

echo "Running cargo clippy for rapier"
cargo clippy --tests --examples --features rapier

echo "Running cargo test for avian"
cargo test --features avian

echo "Running cargo doc"
cargo doc --no-deps
