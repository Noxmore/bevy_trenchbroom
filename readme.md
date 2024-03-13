# bevy_trenchbroom

[![crates.io](https://img.shields.io/crates/v/bevy_trenchbroom)](https://crates.io/crates/bevy_trenchbroom)
[![docs.rs](https://docs.rs/bevy_trenchbroom/badge.svg)](https://docs.rs/bevy_trenchbroom)

Full Bevy integration with TrenchBroom, supporting loading .map files, defining a TrenchBroom game configuration and entities definitions with code, and more!

<img src="assets/screenshots/rune_proto.png">
<label>(A testing map i made, loaded with bevy_trenchbroom)</label>

<br>

# How to use
First add the `bevy_trenchbroom` to your project: `cargo add bevy_trenchbroom`.

Then, simply add the `TrenchBroomPlugin` with a supplied `TrenchBroomConfig` to your app like so:

```rust
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;

fn main() {
    App::new()
        // ...
        .add_plugins(DefaultPlugins)
        .add_plugins(TrenchBroomPlugin::new(trenchbroom_config()))
        // ...
    ;
}

// I recommend putting your `TrenchBroomConfig` in a separate function, most likely in its own module.
fn trenchbroom_config() -> TrenchBroomConfig {
    TrenchBroomConfig::new("example_game") // <- The name of your game
        // Here you can customize the resulting game configuration with a builder-like syntax
        .entity_scale_expression("scale")
        // ...
        
        
        // You can define entity definitions here, these are written to your game's FGD file

        // It's highly recommended to make the first defined entity your `worldspawn`
        .define_entity("worldspawn", EntityDefinition::new_solid()
            .description("World Entity")
            
            .property("skybox", EntDefProperty::string().title("Skybox").description("Path to Skybox"))

            .inserter(|commands, entity, view| {
                view.spawn_brushes(commands, entity, BrushSpawnSettings::new().draw_mesh());
                Ok(())
            })
        )

        .define_entity("angles", EntityDefinition::new_base()
            .property("angles", EntDefProperty::vec3().title("Pitch Yaw Roll (Y Z X)").default_value(Vec3::ZERO))
        )

        .define_entity("player_spawnpoint", EntityDefinition::new_point()
            .description("Bap")
            .base(["angles"])
            
            .property("testing", EntDefProperty::boolean().title("Testing Boolean").default_value(true).description("Awesome description"))
        )
}
```

Then to access the config from TrenchBroom, at some point in your application, you need to call `TrenchBroomConfig::write_folder`. Example:

```rust
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;

// app.add_systems(Startup, write_trenchbroom_config)

fn write_trenchbroom_config(config: Res<TrenchBroomConfig>) {
    if let Err(err) = config.write_folder("<folder_path>") {
        error!("Could not write TrenchBroom config: {err}");
    }

    // This will write <folder_path>/GameConfig.cfg, and <folder_path>/example_game.fgd
}
```

This writes it out every time your app starts, but depending on what you want to do, you might want to write it out some other time.

After you write it out, the folder the files need to end up in is your TrenchBroom games configuration folder which you can find the path of [here](https://trenchbroom.github.io/manual/latest/#game_configuration_files).

## Material Properties

bevy_trenchbroom uses a material properties system to make the texture appear in-game properly. Right next to your texture (`textures/example.png`), add a `ron` file of the same name (`textures/example.ron`).
<br>
In this file you can define certain aspects of the material. (See docs on MaterialProperties for the complete list) 

To avoid an unnecessary amount of polygons, it's recommended to have `__TB_empty.ron` in your textures root directory, with the following content:
```ron
(
    kind: Empty
)
```
This will make any face without a texture get ignored when creating a brush's mesh.

## Loading maps

Now that you have your environment setup, and have assumedly created your map, loading it is pretty easy: simply put a `Handle<Map>` component in an entity, and it will spawn the map with the `spawn_maps` system.
<br>
You can also more easily do this with a `MapBundle`.
```rust
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;

// app.add_systems(Startup, spawn_test_map)

fn spawn_test_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(MapBundle {
        map: asset_server.load("maps/test.map"),
        ..default()
    });
}
```

Unlike scenes, maps support asset hot-reloading, meaning that iteration times are pretty much instant.

## Multiplayer

If you are making a multiplayer game, call `is_server()` when creating your config, and pass in whether the currently running application is a server.

Then, when spawning a map, you can add a `MapSpawningSettings` component to the entity and specify a uuid used to create unique identifiers for each map entity spawned, defining a custom global inserter for your config, you can then use this to add your networking solution's unique identifier type to the entity, allowing for network synchronization.

## Physics/Collisions

`bevy_trenchbroom` supports [rapier3d](https://crates.io/crates/bevy_rapier3d) to easily add colliders when spawning brushes.

First, enable the `rapier` feature on the crate, then either call `convex_collider` or `trimesh_collider` on your `BrushSpawnSettings` when spawning brushes to create the respective type of collider(s) on said brushes.

# Possible future plans
- Map GLTF exporting
- Offload map insertion to another thread (at least offload the filesystem calls)
- Find a more modular approach to material properties
- Radiosity baking (unlikely)

If you want to try to tackle, or have an idea of how to approach any of these, a PR/issue would be greatly appreciated!

# Supported Bevy && TrenchBroom Versions
| Bevy | bevy_trenchbroom | TrenchBroom |
---|--|---
| 0.13 | 0.1 | 2024.1 |

Note: There's a good chance that it will work for other TrenchBroom versions then the one your version of bevy_trenchbroom is made for.

This crate is still in early development and almost certainly has missing features, if your use case isn't covered, please make an issue!