# bevy_trenchbroom

[![crates.io](https://img.shields.io/crates/v/bevy_trenchbroom)](https://crates.io/crates/bevy_trenchbroom)
[![docs.rs](https://docs.rs/bevy_trenchbroom/badge.svg)](https://docs.rs/bevy_trenchbroom)

NOTE: Main branch is not ready for real use yet. Expect (and report!) broken or missing features.

Integration and support for the following workflows:
- TrenchBroom -> .map -> Bevy
- TrenchBroom -> .map -> ericw-tools -> .bsp -> Bevy

<img src="assets/screenshots/ad_tears.png">
<sup>Arcane Dimensions - Tears of the False God .bsp loaded and rendered in Bevy</sup>

<br>

# How to use
- Add the `bevy_trenchbroom` to your project: `cargo add bevy_trenchbroom`.

- Add the `TrenchBroomPlugin` with a supplied `TrenchBroomConfig` to your app like so:

```rust
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;

fn main() {
    App::new()
        // ...
        // TrenchBroom maps use repeating textures, and currently by default bevy's images don't repeat.
        // Use `repeating_image_sampler` to easily create a sampler for this that is optionally filtered.
        .add_plugins(DefaultPlugins.set(ImagePlugin { default_sampler: repeating_image_sampler(false) }))
        .add_plugins(TrenchBroomPlugin::new(
            TrenchBroomConfig::new("example_game") // <- The name of your game
                // Here you can customize the resulting bevy_trenchbroom and game configuration with a builder syntax
                .special_textures(SpecialTexturesConfig::new()) // <- You'll want to enable this if you're loading BSPs with embedded textures via WADs
                // ...
        ))
        // ...
    ;
}
```

Quake's entity classes and their base classes are treated as an analog to Bevy's components and their required components.

You can define your components like so to turn them into quake classes.

```rust
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;

// The required worldspawn class makes up the main structural world geometry and settings. Exactly one exists in every map.
#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
#[geometry(GeometryProvider::new().convex_collider().smooth_by_default_angle().render().with_lightmaps())]
pub struct Worldspawn {
    pub fog_color: Color,
    pub fog_density: f32,
}

// BaseClass doesn't appear in editor, only giving properties to those which use it as a base class.
#[derive(BaseClass, Component, Reflect, Default)]
#[reflect(Component)]
pub struct SolidShadows {
    /// `ericw-tools` `light`: If 1, this model will cast shadows on other models and itself.
    /// Set to -1 on func_detail/func_group to prevent them from casting shadows.
    /// (Default: 0, no shadows)
    pub _shadow: i8,
}

// SolidClass (also known as brush entities) makes the class contain its own geometry, such as a door or breakable
#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
#[require(Visibility)]
// You can also use the #[base()] attribute which will take precedence over the require attribute if you want to require components that don't implement QuakeClass, or don't want to be a required component.
#[base(Visibility, SolidShadows)]
#[geometry(GeometryProvider::new().convex_collider().smooth_by_default_angle().render().with_lightmaps())]
// By default, names are converted into snake_case. Using the classname attribute, you can define the case you want it to be converted to instead.
#[classname(PascalCase)] // Would be FuncWall instead of func_wall
// Or you can just set the classname directly.
#[classname("func_wall")]
pub struct FuncWall;

#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
// Don't include a collider for func_illusionary.
#[geometry(GeometryProvider::new().smooth_by_default_angle().render().with_lightmaps())]
pub struct FuncIllusionary;

// A more advanced example

// PointClass doesn't have any geometry built-in -- simply just a point in space.

/// A GLTF model with no physics.
#[derive(PointClass, Component, Reflect)]
// Here you would probably do a #[component(on_add = "<function>")] to spawn the GLTF scene when this component is added.
// NOTE: If your GLTF model is rotated weird, add the TrenchBroomGltfRotationFix component when adding it.
#[reflect(Component)]
#[require(Transform, Visibility)]
// Sets the in-editor model using TrenchBroom's expression language.
#[model({ "path": model, "skin": skin })]
pub struct StaticProp {
    /// Documentation comments on structs and their fields will show up in-editor.
    pub model: String,
    pub skin: u32,
    pub collision_type: CollisionType,
    pub enable_shadows: bool,
}
// If your struct has fields, you need to implement Default for said fields.
// I recommend using the `smart-default` crate for this.
impl Default for StaticProp {
    fn default() -> Self {
        Self {
            model: default(),
            collision_type: CollisionType::Model,
            enable_shadows: true,
        }
    }
}

/// A GLTF model with physics.
#[derive(PointClass, Component, Reflect)]
// Here you'd use #[component(on_add = "<function>")] to add a RigidBody of your preferred physics engine.
#[reflect(Component)]
#[require(StaticProp)]
pub struct PhysicsProp;

// For `choices` fgd properties, you can derive FgdType on a unit enum.
#[derive(Reflect, FgdType)]
pub enum CollisionType {
    /// Uses colliders defined in the model, or none if the model doesn't have any
    Model,
    /// Mesh bounding box collider
    BoundingBox,
    // No collision
    None,
}
```

Then to access the config from TrenchBroom, at some point in your application, you need to call `TrenchBroomConfig::write_folder`. Example:

```rust
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;

// app.add_systems(Startup, write_trenchbroom_config)

fn write_trenchbroom_config(server: Res<TrenchBroomServer>) {
    if let Err(err) = server.config.write_folder("<folder_path>") {
        error!("Could not write TrenchBroom config: {err}");
    }

    // This will write <folder_path>/GameConfig.cfg, and <folder_path>/example_game.fgd
}
```

This writes it out every time your app starts, but depending on what you want to do, you might want to write it out some other time.

After you write it out, the folder the files need to end up in is your TrenchBroom games configuration folder which you can find the path of [here](https://trenchbroom.github.io/manual/latest/#game_configuration_files).

## Materials and bevy_materialize

Because Bevy's material system so heavily relies on generics, storing and inserting arbitrary materials at runtime is challenging.

To this end, i've created the [bevy_materialize crate](https://github.com/Noxmore/bevy_materialize),

TODO
If you're loading .map files, to avoid an unnecessary amount of polygons being rendered or used for trimesh collision, it's recommended to have `__TB_empty.material` in your textures root directory, with the following content:
```toml
[properties]
remove = true
```
This will make any face without a texture get ignored when creating a brush's mesh.

## Loading maps

Now that you have your environment setup, and have assumedly created your map, loading it is pretty easy.
```rust
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;

// app.add_systems(Startup, spawn_test_map)

fn spawn_test_map(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SceneRoot(asset_server.load("maps/test.map#Scene")));
    // Or, if you're using BSPs.
    commands.spawn(SceneRoot(asset_server.load("maps/test.bsp#Scene")));
}
```

## Physics/Collisions

`bevy_trenchbroom` supports [bevy_rapier3d](https://crates.io/crates/bevy_rapier3d) and [avian3d](https://crates.io/crates/avian3d) to easily add colliders when spawning geometry.

First, enable the `rapier` or `avian` feature on the crate, then either call `convex_collider` or `trimesh_collider` on your class's `GeometryProvider` to create the respective type of collider(s) with said geometry.

## Known Bugs

If you are using GLTF models, you might notice that they are rotated 90 degrees in TrenchBroom, compared to in Bevy.
To fix this, add the `TrenchBroomGltfRotationFix` Component to your entity in its spawner.

# Possible future plans
- Entity IO

# Supported Bevy && TrenchBroom Versions
| Bevy | bevy_trenchbroom | TrenchBroom |
---|--|---
| 0.15 | 0.6 | 2024.1 |
| 0.14 | 0.4-0.5 | 2024.1 |
| 0.13 | 0.1-0.3 | 2024.1 |

Note: There's a good chance that it will work for other TrenchBroom versions then the one your version of bevy_trenchbroom is made for.

This crate is still in early development and certainly has missing features, if your use case isn't covered, please make an issue!