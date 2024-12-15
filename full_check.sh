# Runs a bunch of commands to make sure everything is good. This is my bootleg replacement for CI :)

set -e

echo cargo c --all-targets
cargo c --all-targets

echo cargo c --features rapier --all-targets
cargo c --features rapier --all-targets

echo cargo c --features avian --all-targets
cargo c --features avian --all-targets

echo cargo test
cargo test