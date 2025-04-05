echo "Running cargo fmt"
cargo fmt --all --check || exit /b 1

echo "Running cargo clippy for default features"
cargo clippy --tests --examples || exit /b 1

echo "Running cargo clippy for no default features"
cargo clippy --tests --examples --no-default-features || exit /b 1

echo "Running cargo clippy for avian"
cargo clippy --tests --examples --features avian || exit /b 1

echo "Running cargo clippy for rapier"
cargo clippy --tests --examples --features rapier || exit /b 1

echo "Running cargo test for avian"
cargo test --features avian || exit /b 1

echo "Running cargo doc"
cargo doc --no-deps || exit /b 1
