#!/bin/bash
set -e

echo cargo fmt --all --check
cargo fmt --all --check

echo cargo clippy --workspace
cargo clippy --workspace

echo cargo clippy --no-default-features --workspace
cargo clippy --no-default-features --workspace

echo cargo clippy --features rapier --workspace
cargo clippy --features rapier --workspace

echo cargo clippy --features avian --workspace
cargo clippy --features avian --workspace

echo cargo test --features avian
cargo test --features avian