:: These commands correspond 1 to 1 with github actions.

echo Run cargo fmt
cargo fmt --all --check || exit /b 1


echo Run cargo clippy for default features
cargo clippy --all-targets || exit /b 1

echo Run cargo clippy without default features
cargo clippy --all-targets --no-default-features || exit /b 1

echo Run cargo clippy for avian
cargo clippy --all-targets --features avian || exit /b 1

echo Run cargo clippy for rapier
cargo clippy --all-targets --features rapier || exit /b 1


echo Run tests with avian
cargo test --features avian || exit /b 1


echo Run cargo doc with default features
cargo doc --no-deps || exit /b 1
