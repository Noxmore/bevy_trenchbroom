cargo fmt --all --check || exit /b 1

cargo clippy --workspace || exit /b 1

cargo clippy --no-default-features --workspace || exit /b 1

cargo clippy --features rapier --workspace || exit /b 1

cargo clippy --features avian --workspace || exit /b 1

cargo test --features avian || exit /b 1