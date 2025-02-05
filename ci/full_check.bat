cargo clippy --all-targets || exit /b 1

cargo clippy --no-default-features --all-targets || exit /b 1

cargo clippy --features rapier --all-targets || exit /b 1

cargo clippy --features avian --all-targets || exit /b 1

cargo fmt --all --check || exit /b 1

cargo test --features avian || exit /b 1