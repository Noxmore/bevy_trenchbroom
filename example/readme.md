# bevy_trenchbroom examples
To run an example, use `cargo run --package <example>`.

To run `bsp_loading` or `map_loading` in a headless context, use `cargo run --package <example> --no-default-features`

For `physics`, you either need to supply `--features avian32`, `--features avian64` or `--features rapier` to specify the physics engine. It currently does not support headless.
