# bevy_trenchbroom

[![crates.io](https://img.shields.io/crates/v/bevy_trenchbroom)](https://crates.io/crates/bevy_trenchbroom)
[![docs.rs](https://docs.rs/bevy_trenchbroom/badge.svg)](https://docs.rs/bevy_trenchbroom)

Quake level loading for Bevy!

More specifically, integration and support for the following workflows:
- TrenchBroom -> .map -> Bevy
- TrenchBroom -> .map -> ericw-tools -> .bsp -> Bevy

<img src="assets/screenshots/ad_tears.png">
<sup>Arcane Dimensions - Tears of the False God .bsp loaded and rendered in Bevy</sup>

<br>

# Quickstart
- Add the `bevy_trenchbroom` to your project: `cargo add bevy_trenchbroom`.

- Add the `TrenchBroomPlugin` with a supplied `TrenchBroomConfig` to your app like so:

```rust no_run
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;

fn main() {
    App::new()
        // ...
        .add_plugins(TrenchBroomPlugins(TrenchBroomConfig::new("your_game_name")))
        // ...
    ;
}
```

You can configure `TrenchBroomConfig` through a builder syntax.

Quake's entity classes are treated as an analog to Bevy's components. Here is an example of a simple point class:
```rust
use bevy_trenchbroom::prelude::*;

#[point_class]
#[derive(Default)]
struct MyClass {
	property_a: f32,
	property_b: String,
}
```

Then register the type with `.register_type::<MyClass>()` on app initialization.

To access your game from TrenchBroom, at some point in your application, you need to call `TrenchBroomConfig::write_game_config` and `TrenchBroomConfig::add_game_to_preferences`. For example:

```rust
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;

// app.add_systems(Startup, write_trenchbroom_config)

fn write_trenchbroom_config(
    server: Res<TrenchBroomServer>,
    type_registry: Res<AppTypeRegistry>,
) {
    // This will write <TB folder>/games/example_game/GameConfig.cfg,
    // and <TB folder>/games/example_game/example_game.fgd
    if let Err(err) = server.config.write_game_config_to_default_directory(&type_registry.read()) {
        error!("Could not write TrenchBroom game config: {err}");
    }

    // And this will add our game to <TB folder>/Preferences.json
    if let Err(err) = server.config.add_game_to_preferences_in_default_directory() {
        error!("Could not write TrenchBroom preferences: {err}");
    }
}
```

This writes it out every time your app starts, but depending on what you want to do, you might want to write it out some other time.

After you write it out, you have to select the created game config in TrenchBroom's preferences when creating a new map.

For more comprehensive documentation on this topic, see [the manual](https://docs.rs/bevy_trenchbroom/latest/bevy_trenchbroom/manual/index.html).

## Materials and `bevy_materialize`

Because Bevy's material system so heavily relies on generics, storing and inserting arbitrary materials at runtime is challenging.

To this end, i've created the [bevy_materialize crate](https://github.com/Noxmore/bevy_materialize), which `bevy_trenchbroom` uses.

`TrenchBroomPlugin` Automatically adds `MaterializePlugin` with the default `toml` deserializer. If you wish to use a different deserializer, add your own `MaterializePlugin` before adding `TrenchBroomPlugin`.

Texture loaders for loose and embedded textures can be changed in `TrenchBroomConfig`.

The default loader for loose textures first looks for `<texture>.<GenericMaterial extension>`.
`<GenericMaterial extension>` is also defined in your config, and is "material" by default.

If the file can't be found, it then tries to load `<texture>.<Image extension>` into a `StandardMaterial` as a fallback.
`<Image extension>` can similarly changed in your config.
The fallback is because if you have a bunch of simple textures where the material file would look something like
```toml
[material]
base_color_texture = "example.png"
```
it can get a bit repetitive.

You can also configure the rest of the properties of the default material in `MaterializePlugin`.

## Loading maps

Now that you have your environment setup, and have assumedly created your map, loading it is pretty easy.
```rust
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;

// app.add_systems(Startup, spawn_test_map)

fn spawn_test_map(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(SceneRoot(asset_server.load("maps/test.map#Scene")));
    // Or, if you're using BSPs.
    commands.spawn(SceneRoot(asset_server.load("maps/test.bsp#Scene")));
}
```

## BSP

`bevy_trenchbroom` supports BSP loading via the [qbsp](https://github.com/Noxmore/qbsp) crate when the `bsp` feature is activated.

For more information, please see [the manual](https://docs.rs/bevy_trenchbroom/latest/bevy_trenchbroom/manual/index.html#BSP).

## Physics/Collisions

`bevy_trenchbroom` supports [bevy_rapier3d](https://crates.io/crates/bevy_rapier3d) and [avian3d](https://crates.io/crates/avian3d) to easily add colliders when spawning geometry.

First, enable the `rapier` or `avian` feature on the crate, then either call `convex_collider` or `trimesh_collider` on your class's `SpawnHooks` to create the respective type of collider(s) with said geometry.

## Multiplayer

For dedicated servers `bevy_trenchbroom` supports headless mode by turning off its `client` feature. e.g.
```toml
bevy_trenchbroom = { version = "...", default-features = false }
```

# Migration Guidde
See the [Migration Guide](https://github.com/Noxmore/bevy_trenchbroom/blob/main/Migration%20Guide.md) when updating between versions!

# Version support table
| Bevy | bevy_trenchbroom | TrenchBroom   | ericw-tools |
|------|------------------|---------------|-------------|
| 0.16 | 0.8-0.9          | 2025.3        | 2.0.0-alpha9
| 0.15 | 0.6-0.7          | 2025.1-2025.2 | N/A
| 0.14 | 0.4-0.5          | 2024.1        | N/A
| 0.13 | 0.1-0.3          | 2024.1        | N/A

<sup>There is a good chance other versions of TrenchBroom and ericw-tools will work, especially close ones, these are just the versions we officially support.</sup>

<sup>Versions before 0.8 didn't target a clear version of ericw-tools, or didn't support BSPs at all, which is why they are N/A.</sup>
