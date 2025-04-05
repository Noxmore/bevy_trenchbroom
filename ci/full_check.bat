:: These commands correspond 1 to 1 with github actions.

echo Run cargo fmt
cargo fmt --all --check || exit /b 1


echo Run cargo clippy for default features
cargo clippy --tests --examples || exit /b 1

echo Run cargo clippy without default features
cargo clippy --tests --examples --no-default-features || exit /b 1

echo Run cargo clippy for avian
cargo clippy --tests --examples --features avian || exit /b 1

echo Run cargo clippy for rapier
cargo clippy --tests --examples --features rapier || exit /b 1


echo Run tests with avian
cargo test --features avian || exit /b 1


echo Run cargo doc with default features
cargo doc --no-deps || exit /b 1
