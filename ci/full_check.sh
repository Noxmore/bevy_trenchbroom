#!/bin/bash
set -e

echo cargo clippy --all-targets
cargo clippy --all-targets

echo cargo clippy --no-default-features --all-targets
cargo clippy --no-default-features --all-targets

echo cargo clippy --features rapier --all-targets
cargo clippy --features rapier --all-targets

echo cargo clippy --features avian --all-targets
cargo clippy --features avian --all-targets

echo cargo fmt --all --check
cargo fmt --all --check

echo cargo test --features avian
cargo test --features avian