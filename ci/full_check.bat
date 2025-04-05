cargo fmt --all --check || exit /b 1
cargo clippy --tests --examples || exit /b 1
cargo clippy --tests --examples --no-default-features || exit /b 1
cargo clippy --tests --examples --features avian || exit /b 1
cargo clippy --tests --examples --features rapier || exit /b 1
cargo test --features avian || exit /b 1
cargo doc --no-deps || exit /b 1
