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

# How to use
- Add the `bevy_trenchbroom` to your project: `cargo add bevy_trenchbroom`.

- Add the `TrenchBroomPlugin` with a supplied `TrenchBroomConfig` to your app like so:

```rust ignore
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;

fn main() {
    App::new()
        // ...
        .add_plugins(TrenchBroomPlugin(
            // Here you can customize the resulting bevy_trenchbroom
            // and game configuration with a builder syntax
            TrenchBroomConfig::new("your_game_name")
                // For example: by default, the scale is set to
                // 1 unit = 1 inch assuming that
                // 1 Bevy unit = 1 meter.
                // This makes 1 TrenchBroom unit = 1 Bevy unit.
                .scale(1.)
                // You should have a good look at all the settings,
                // as there are a few that can cause some nasty
                // little bugs if you don't know that they're
                // active. (e.g. `lightmap_exposure`)

                // ...
        ))
        // ...
    ;
}
```

NOTE: By default, `TrenchbroomConfig::auto_remove_textures` contains `__TB_empty`, meaning that when loading `.map`s, any face without a texture will be automatically ignored, saving processing and render time.

Quake's entity classes are treated as an analog to Bevy's components.

You can define your components like so to turn them into quake classes.

```rust
use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;
use bevy_trenchbroom::bsp::base_classes::*;

// The required worldspawn class makes up the main structural
// world geometry and settings. Exactly one exists in every map.
#[derive(SolidClass, Component, Reflect, Default)]
#[reflect(QuakeClass, Component)]
// Quake classes use an inheritance system alike OOP
// programming languages.
// If you're using a BSP workflow, this base class includes a bunch
// of useful compiler properties.
#[base(BspWorldspawn)]
#[geometry(GeometryProvider::new().trimesh_collider().smooth_by_default_angle().with_lightmaps())]
pub struct Worldspawn {
    /// A useful example property.
    pub fog_color: Color,
    pub fog_density: f32,
}

// BaseClass doesn't appear in editor, only giving properties to
// those which use it as a base class
// by using the `base` attribute.
#[derive(BaseClass, Component, Reflect, Default)]
#[reflect(QuakeClass, Component)]
pub struct MyBaseClass {
    /// Documentation comments will be visible in-editor!
    pub my_value: u32,
}

// SolidClass (also known as brush entities) makes the class
// contain its own geometry, such as a door or breakable
#[derive(SolidClass, Component, Reflect)]
#[reflect(QuakeClass, Component)]
#[base(Visibility, MyBaseClass)]
#[geometry(GeometryProvider::new().trimesh_collider().smooth_by_default_angle().with_lightmaps())]
// By default, names are converted into snake_case.
// Using the classname attribute, you can define the case you want
// it to be converted to instead.
#[classname(PascalCase)] // Would be FuncWall instead of func_wall
// Or you can just set the classname directly.
#[classname("func_wall")]
pub struct FuncWall;

#[derive(SolidClass, Component, Reflect)]
#[reflect(QuakeClass, Component)]
// If you're using a BSP workflow, this base class includes a bunch
// of useful compiler properties.
#[base(BspSolidEntity)]
// Don't include a collider for func_illusionary.
#[geometry(GeometryProvider::new().smooth_by_default_angle().with_lightmaps())]
pub struct FuncIllusionary;

// A more advanced example

// PointClass doesn't have any geometry built-in,
// simply just a point in space.

/// A GLTF model with no physics.
#[derive(PointClass, Component, Reflect)]
// Here you would do a
// #[spawn_hook(<function>)] to spawn the GLTF scene when
// this entity is spawned in the scene world.
//
// Alternatively, you could create a system with a query
// `Query<&StaticProp, Without<SceneRoot>>`
// and spawn it through that, but .
//
// If your entity has a hardcoded model, you can use a function
// like `spawn_class_gltf` to do the above automatically.
#[reflect(QuakeClass, Component)]
#[base(Visibility)]
// Sets the in-editor model using TrenchBroom's expression language.
#[model({ "path": model, "skin": skin })]
pub struct StaticProp {
    // no_default makes the field have an empty default value
    // in-editor, and will cause an error if not defined.
    #[no_default]
    pub model: String,
    /// Documentation comments on structs and their fields
    /// will show up in-editor.
    pub skin: u32,
    pub collision_type: CollisionType,
    pub enable_shadows: bool,
}
// If your struct has fields, you need to implement Default.
// I recommend using the `smart-default` crate for this.
impl Default for StaticProp {
    fn default() -> Self {
        Self {
            model: default(),
            skin: 0,
            collision_type: CollisionType::Model,
            enable_shadows: true,
        }
    }
}

/// A GLTF model with physics.
#[derive(PointClass, Component, Reflect)]
// Here you'd use #[spawn_hook(<function>)], a component hook, or a system to
// add a RigidBody of your preferred physics engine.
#[reflect(QuakeClass, Component)]
#[base(StaticProp)]
pub struct PhysicsProp;

// For `choices` properties, you can derive FgdType on a unit enum.
#[derive(Reflect, FgdType)]
pub enum CollisionType {
    /// Uses colliders defined in the model,
    /// or none if the model doesn't have any.
    Model,
    /// Mesh bounding box collider.
    BoundingBox,
    // No collision.
    None,
}
```

Now, just register these like you would any other type via `App::register_type::<T>()`, and you are good to go!

To access the config from TrenchBroom, at some point in your application, you need to call `TrenchBroomConfig::write_game_config` and `TrenchBroomConfig::add_game_to_preferences`. For example:

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

Specifically, it is oriented around using the latest [ericw-tools](https://ericwa.github.io/ericw-tools/) as the compiler, including some base classes such as `BspWorldspawn`, `BspSolidEntity`, and `BspLight` that contain various compiler-specific properties.

GPU-driven animated lighting is also supported, you can customize the animation with the [`LightingAnimators`](https://docs.rs/bevy_trenchbroom/latest/bevy_trenchbroom/bsp/lighting/types/struct.LightingAnimators.html) resource.

If you are to use BSPs, i recommend turning off ambient light `.insert_resource(AmbientLight::NONE)`, and using at least the following compiler settings for `qbsp` and `light`:

`qbsp -bsp2 -wrbrushesonly -nosubdivide -nosoftware -path assets -notex`
- `-bsp2` - Uses the more modern BSP2 format, expanding various limits.
- `-wrbrushesonly` - Adds brush data into the BSP, and removes hull collision data which is [useless to this crate](https://github.com/Noxmore/bevy_trenchbroom/issues/16). This is necessary if you want to use convex colliders.
- `-nosubdivide` - Don't subdivide geometry unnecessarily, there's probably some legacy support reason why it does this by default.
- `-nosoftware` - Explicitly drop support for software renderers.
- `-path assets` This lets the compiler read your loose textures from assets/textures (currently, ["textures" is hardcoded](https://github.com/ericwa/ericw-tools/issues/451)).
- `-notex` Allows use of loose textures, but doesn't embed WAD textures. See [this issue](https://github.com/ericwa/ericw-tools/issues/404) for context.

`light -extra4 -novanilla -lightgrid -path assets`
- `-extra4` - Multisampling, makes shadows smoother.
- `-novanilla` - Writes colored light data into a BSPX lump, instead of legacy colorless light data.
- `-lightgrid` - Calculate volumetric lighting parsed into irradiance volumes, dynamic objects won't have any lighting without this.
- `-path assets` - Same as above, for color bouncing

Currently, PVS data generated by `vis` isn't used.

## Physics/Collisions

`bevy_trenchbroom` supports [bevy_rapier3d](https://crates.io/crates/bevy_rapier3d) and [avian3d](https://crates.io/crates/avian3d) to easily add colliders when spawning geometry.

First, enable the `rapier` or `avian` feature on the crate, then either call `convex_collider` or `trimesh_collider` on your class's `GeometryProvider` to create the respective type of collider(s) with said geometry.

## Multiplayer

For dedicated servers `bevy_trenchbroom` supports headless mode by turning off its `client` feature. e.g.
```toml
bevy_trenchbroom = { version = "...", default-features = false }
```

# Possible future plans
- Entity IO

# Version support table
| Bevy | bevy_trenchbroom | TrenchBroom   | ericw-tools |
|------|------------------|---------------|-------------|
| 0.16 | 0.8              | 2025.3        | 2.0.0-alpha9
| 0.15 | 0.6-0.7          | 2025.1-2025.2 | N/A
| 0.14 | 0.4-0.5          | 2024.1        | N/A
| 0.13 | 0.1-0.3          | 2024.1        | N/A

<sup>There is a good chance other versions of TrenchBroom and ericw-tools will work, especially close ones, these are just the versions we officially support.</sup>

<sup>Versions before 0.8 didn't target a clear version of ericw-tools, or didn't support BSPs at all, which is why they are N/A.</sup>