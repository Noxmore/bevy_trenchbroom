# Introduction

Welcome to the bevy_trenchbroom documentation manual! This is a more comprehensive resource on how to use the library, and lays out the vast majority of its features!

The best place to read this document is on the [docs.rs page](https://docs.rs/bevy_trenchbroom/latest/bevy_trenchbroom/manual/index.html), as it correctly handles code blocks, type links, and has an easily accessible "Sections" menu on the left. This document is written in Markdown. While human-readable as raw text, it is strongly advised that you should read this as rendered Markdown such as on the aforementioned website.

This document won't go into how to use TrenchBroom. For that, it has [its own manual](https://trenchbroom.github.io/manual/latest), but this will touch a little on the mechanisms behind it and Quake, and how they relate to bevy_trenchbroom.

While this document does contain some repeat information, it is assumed you've gone over the `Quickstart` section in the [readme](https://github.com/Noxmore/bevy_trenchbroom/blob/main/readme.md) before reading this.

# Quake Classes

Quake and by extension TrenchBroom communicates types of entities though a class-based format called "FGD" not too unlike classes seen in object oriented programming.
Bevy and Rust don't support class hierarchies, so instead these classes are represented compositionally as Bevy Components, and the FGD is generated automatically from registered types.

There are 3 types of classes:
- **Base** - Like an abstract class in OOP; doesn't appear in-editor, meant to be inherited by other classes.
- **Point** - Appears in-editor in the Entity Browser. Represents a single point in space. Can have a model or billboarded sprite attached to it.
- **Solid** - An entity made up of brushes: convex hulls that make up the geometry of your map. Its origin is the world origin unless provided with an [origin brush](#special-textures).

A base class won't appear in the final FGD if no editor-visible class inherits it.

## Worldspawn
Worldspawn is a special entity that comes from Quake providing the main structural static world geometry that will almost always make up the majority of your map. Exactly one exists in every map, and thus it is also used to host global map-specific properties.
The only thing that defines the Worldspawn entity is that it has the name `worldspawn`. By default, an empty Worldspawn is defined for you and automatically registered. You can easily use `.override_class::<T>()` on app initialization to replace it with your own.

## Defining and Registering a Class
There are 3 attribute macros that will automatically implement the `QuakeClass` trait for your component. One for each class type. The reason it is not a derive macro is that the `QuakeClass` trait depends on a number of other derivable and reflected traits which these attribute macros imply if not already derived/reflected.

Here is the simplest possible point class:
```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
#[point_class]
struct MyClass;
```

This can be changed to a solid or base class by changing the class type in `#[<type>_class]`.

Any fields of the struct will appear as properties in TrenchBroom, and when a map is loaded, those fields will be automatically set with the values the properties were set to.<br>
The types of all fields must implement [`FgdType`](bevy_trenchbroom::fgd::FgdType) so bevy_trenchbroom knows how to parse and stringify them in adherence with TrenchBroom's formats.

If your class isn't a unit struct, it must implement [`Default`].

Documentation comments on classes and fields contained within them will appear in TrenchBroom when selected.

If not already derived, the attribute macro automatically derives required traits like `Reflect` and `Component`. Classes tie into Bevy's type registry, so to register the class to appear in TrenchBroom, simply call `.register_type::<MyClass>()` on app initialization. <br>
You can also disable a specific class from appearing in the [game configuration](#configuration) by calling `.disable_class::<T>()` from `App` during initialization. The `.override_class::<T>()` function effectively calls `disable_class::<T>()` then `register_type::<T>()` in a single function call.

A handful of widely useful classes are registered by default such as light, basic brush entities, and base classes. The plugins that add these classes are formatted as `*ClassesPlugin`, and can be disabled from `TrenchBroomPlugins`. Example:
```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
use bevy_trenchbroom::class::builtin::SolidClassesPlugin;
// ...
TrenchBroomPlugins(default()).build().disable::<SolidClassesPlugin>()
# ;
```
Builtin point classes often have default model/sprite paths and settings. If these don't align with your project, you may have to redefine them. If they do, then great! You can just place assets in the specified locations and you are good to go!

Hint: The assets in this repository are completely free to use for your own game!

## Class Configuration
You can use a number of different sub-attributes to configure exactly how the class appears in TrenchBroom as well as how it spawns with the following syntax:
```rust ignore
#[<type>_class(
	<attribute>(<args>),
	// ...
)]
```

A few of these specify expressions as inputs. these expressions follow the [TrenchBroom expression language](https://trenchbroom.github.io/manual/latest/#expression_language), which is surprisingly powerful, allowing for things like retrieving entity properties, doing number and boolean operations, and much more.

### ***Attribute: `model`***
Used to show a point class entity as a model rather than a default solid-colored cube in TrenchBroom.
The first form, `model(<path TB expression>)` takes in a path, whether from a property, string literal or whatnot, and displays the model at the path in TrenchBroom.

The second form is `model({ "path": <path TB expression>, "skin": <integer TB expression>, "frame": <integer TB expression>, "scale": <number/vector TB expression> })`. This form allows much more control over how the model appears. Only the `"path"` property is required, the rest can be omitted for their default values.

### ***Attribute: `color`***
`color(<red> <green> <blue>)` Changes the wireframe color of the entity. Each number has a range from 0 to 255.

### ***Attribute: `iconsprite`***
Alias for `model`. When this or `model` is set to an image, it displays the entity as said image, presented as a billboard (always facing the camera).

### ***Attribute: `size`***
The bounding box of a point class entity in TrenchBroom.
Syntax: `size(<-x> <-y> <-z>, <+x> <+y> <+z>)`

### ***Attribute: `classname`***
Used to set the name of the entity in TrenchBroom. By default, the name is a snake_case version of the Rust type name.

The `classname(<case type>)` form of this attribute lets you choose the case this type's name is converted into. snake_case is the default since TrenchBroom groups entities together based on the first element of the underscore-separated name.

The `classname(<string>)` form lets you set the classname directly.

Examples:
```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
#[base_class(
	classname(camelCase),
)]
struct MyBaseClass;

assert_eq!(MyBaseClass::CLASS_INFO.name, "myBaseClass");

#[solid_class(
	classname("new_classname"),
)]
struct MySolidClass;

assert_eq!(MySolidClass::CLASS_INFO.name, "new_classname");
```

### ***Attribute: `group`***
Prefixes the classname with `<group>_` to add it into the entity group `<group>` in TrenchBroom. This is mainly useful to avoid stuttering namespaces, e.g. EnemyZombie in `enemy.rs`.

Examples:
```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
#[point_class(
	group("enemy"),
)]
struct Zombie;

assert_eq!(Zombie::CLASS_INFO.name, "enemy_zombie");
```

### ***Attribute: `base`***
This attribute defines what base classes this class should inherit by providing a list of types, separated by commas. Each type can also have attributes applied to it, allowing gating certain base classes.

Examples:
``` rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
#[point_class(
	base(
		#[cfg(feature = "client")] Visibility,
		Target,
		Targetable,
	),
)]
struct MyPointClass;
```

### ***Attribute: `hooks`***
This attribute requires a Rust expression that produces a `SpawnHooks` instance. If this attribute isn't specified, it uses the default spawn hooks defined in [`TrenchBroomConfig`](bevy_trenchbroom::config::TrenchBroomConfig).

[`SpawnHooks`](bevy_trenchbroom::class::spawn_hooks::SpawnHooks) is a struct containing a list of functions to be called in the [scene world](#loading-maps) when an entity spawns, provided with [context](bevy_trenchbroom::class::QuakeClassSpawnView) for the spawn.

Examples:
```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
# use bevy_trenchbroom::class::QuakeClassSpawnView;
// When creating your App...
TrenchBroomPlugins(
	TrenchBroomConfig::new("example")
		.default_solid_spawn_hooks(|| SpawnHooks::new().smooth_by_default_angle())
		// The above call will make slightly angled brush faces have
		// interpolated normals, allowing smooth curves in geometry.
		// If you're using a physics engine integration, this is
		// also where you could put `.convex_collider()` or
		// `.trimesh_collider()`.
)
# ;
// ...

// Default solid class spawn hooks will be used.
#[solid_class]
struct SolidClassA;

#[solid_class(
	// This overrides the above hooks. This geometry will not be smoothed.
	hooks(SpawnHooks::new().push(Self::example_spawn_hook)),
)]
struct SolidClassB;
impl SolidClassB {
	/// An example spawn hook to give you a sense of the function signature.
	/// If you are to make a reusable hook, you should wrap it in an extension trait,
	/// so that users of it can use the simple builder syntax seen above.
	pub fn example_spawn_hook(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		// The view gives you access to everything you could want
		// about the class and entity being spawned, as well as
		// access to the scene world and entity being spawned into.
		Ok(())
	}
}
```

Some hooks are included by default, some you might be interested in are
- `.smooth_by_default_angle()` which smooths out the normals of curved surfaces.
- `.convex_collider()` and `.trimesh_collider()` which add colliders if you have a physics engine integration enabled.
- `.with(<bundle>)` and `.meshes_with(<bundle>)` if all you want to do is add components to the entity or its mesh entities.

Hacky note: Because of the macro implementation, you technically have access to the [`QuakeClassSpawnView`](bevy_trenchbroom::class::QuakeClassSpawnView) variable called `view` when creating the spawn hooks instance, allowing you to extend default hooks through it. You probably shouldn't rely on this.

### ***Field Attribute: `must_set`***
Use on fields you want to output an error if not defined, rather than just being replaced by the field's default value.

Examples:
```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
#[point_class(
	model({ "path": model }),
)]
#[derive(Default)]
struct Prop {
	#[class(must_set)] // A prop wouldn't make much sense without a model!
	model: String,
}
```

### ***Field Attribute: `ignore`***
Doesn't show the field in-editor as a property.

Examples:
```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
#[point_class]
#[derive(Default)]
struct CoolClass {
	foo: i32,
	
	#[class(ignore)] /// This doesn't implement FgdType!
	bar: Vec<f32>,
}

assert_eq!(CoolClass::CLASS_INFO.properties.len(), 1);
```

### ***Field Attribute: `rename`***
Renames the in-editor property. Doesn't rename the Rust field.

Examples:
```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
#[point_class]
#[derive(Default)]
struct CoolerClass {
	#[class(rename = "SoCool")]
	so_cool: u32,
}

# assert_eq!(CoolerClass::CLASS_INFO.properties[0].name, "SoCool");
```

### ***Field Attribute: `default`***
Overrides the default value of the property in TrenchBroom that appears as a hint in the property's UI.
Note that this does not change the default value of the field, only how it appears in TrenchBroom.
Without this, the default value of the field is used.

Note: Only integers can be without quotes.

Examples:
```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
#[point_class]
#[derive(Default)]
struct CoolestClass {
	#[class(default = 9999)]
	foo: u32,

	#[class(default = "Default Value!!!")]
	bar: i32,
}

# assert_eq!((CoolestClass::CLASS_INFO.properties[0].default_value.unwrap())(), "9999");
# assert_eq!((CoolestClass::CLASS_INFO.properties[1].default_value.unwrap())(), "\"Default Value!!!\"");
```

### ***Field Attribute: `title`***
Sets the TrenchBroom property title of the field. It's a very small change, when a property is selected, the help/info window below it starts with `Property "property_name" (property_name)`. With this set, it becomes `Property "property_name" (Property Title)`.


## Special Properties
Some properties have special UI in TrenchBroom, this includes
- colors, which have a color picker widget
- `choices`, a dropdown menu with some set options
- `spawnflags`, an integer with each bit having a checkbox

You can create a `choices` property by deriving [`FgdType`](bevy_trenchbroom::fgd::FgdType) on a unit enum.

```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
#[derive(FgdType)]
enum EnemyMovementType {
	/// This enemy will walk around the area, anchored to where you've placed it.
	Patrolling,
	/// This enemy will wait and watch for intruders.
	Stationary,
}
```

In TrenchBroom, the options in the dropdown will appear as `key : description`.
If a variant has a documentation comment, that will be used as `description`, otherwise it will use the variant name.

Keys will be the variant name as well. You can also make the keys numbers by including the `number_key` attribute and providing variant numbers.

```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
#[derive(FgdType)]
#[number_key]
enum DirtMode {
	Ordered = 0,
	Randomized = 1,
}
```

In TrenchBroom `DirtMode`'s dropdown will consist of
- `0 : Ordered`
- `1 : Randomized`

You can also create a `spawnflags` property by using the `enumflags2` crate (for technical reasons, we couldn't support `bitflags`), create some bitflags as normal, and wrap it in [`FgdFlags<T>`](bevy_trenchbroom::fgd::FgdFlags) for the property.
```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
# use bevy_trenchbroom::fgd::FgdFlags;
# use enumflags2::*;
#[bitflags(default = Beep | Bap)]
#[derive(Reflect, Debug, Clone, Copy)]
#[repr(u16)]
enum FlagsTest {
	/// Boop flag title
	Boop = 1,
	Beep = 1 << 1,
	Bap = 1 << 2,
}

#[point_class]
#[derive(Default)]
struct MyClass {
	some_flags: FgdFlags<FlagsTest>,
}
```

The checkboxes will be titled with the documentation comment, or with the variant name if one isn't set.

## Lighting
Quake originally only supported baked lightmaps provided by a compiled [BSP](#bsp), while Bevy supplies real-time point, spot, and directional/sun lights.

bevy_trenchbroom supports either, or a mix of both with 5 different supported lighting workflows out of the box (see [`LightingWorkflow`](bevy_trenchbroom::class::builtin::LightingWorkflow) docs for more information).<br>
4 of these include baked [BSP](#bsp) lights, so if you're not planning to [BSPs](#bsp), you'll almost certainly have all you need with the default builtin lighting classes.

# Loading Maps
To load a map into Bevy, load it as a regular Bevy scene like so
```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
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

`test.map` and `test.bsp` load `QuakeMap` and `Bsp` assets respectively. Both of these construct a ready-to-spawn scene when loaded, calling classes' spawn hooks in the loading process.
This scene is labeled "Scene" and can be retrieved with Bevy's `<path>#<label>` asset path syntax as the code above shows.

TIP: For processes in the main world that depend on colliders (e.g. AI navigation mesh construction), observe the `SceneCollidersReady` rather than the `SceneInstanceReady` trigger.

# Configuration
For TrenchBroom to know everything it needs to about your game, bevy_trenchbroom generates a TrenchBroom game configuration.

This is half of what [`TrenchBroomConfig`](bevy_trenchbroom::config::TrenchBroomConfig) provides control of. For specifics, you should check out [`TrenchBroomConfig`](bevy_trenchbroom::config::TrenchBroomConfig)'s documentation, as this section won't cover every single setting, only a few ones of note.

Once configured, you can write out a game configuration with [`write_game_config_to_default_directory(...)`](bevy_trenchbroom::config::TrenchBroomConfig::write_game_config_to_default_directory), and automatically set the game's directory to use for assets with [`add_game_to_preferences_in_default_directory()`](bevy_trenchbroom::config::TrenchBroomConfig::add_game_to_preferences_in_default_directory).
By default, this is automatically done by [`WriteTrenchBroomConfigOnStartPlugin`](bevy_trenchbroom::config::WriteTrenchBroomConfigOnStartPlugin). If you want to write the config some other time, you can disable it in [`TrenchBroomPlugins`](bevy_trenchbroom::TrenchBroomPlugins).

## Special Textures
Recreations of Quake's liquid and sky materials are included as [`LiquidMaterial`](bevy_trenchbroom::special_textures::LiquidMaterial) and [`QuakeSkyMaterial`](bevy_trenchbroom::special_textures::QuakeSkyMaterial) respectively.
When loading embedded textures (ones stored directly in the BSP file), these are automatically applied according to [Quake's texture rules](https://quakewiki.org/wiki/Textures).

[`TrenchBroomConfig::auto_remove_textures`](bevy_trenchbroom::config::TrenchBroomConfig::auto_remove_textures) is a set of texture names whose meshes are skipped on map load. By default, "__TB_empty"—the name used for untextured faces—is in this set. If you're using a `.map` workflow, this can drastically reduce the amount of redundant or unseen faces in your level mesh.

[`TrenchBroomConfig::origin_textures`](bevy_trenchbroom::config::TrenchBroomConfig::origin_textures) is a set of texture names that sets the transform origin of a brush entity to a brush within it if the brush is fully textured with any of these textures. This allows for example, a door or rotating entity to rotate around a specific point.<br>
NOTE: For [BSPs](#bsp), this step is done at compile time, and only works on a texture called "origin". For this reason, "origin" is the singular default string in this set.

# BSP
Quake doesn't support loading `.map`s, instead they are compiled into `.bsp` files by a map compiler, such as [ericw-tools](https://ericwa.github.io/ericw-tools/), which this crate also supports if the `bsp` feature is enabled. See the [version support table](https://github.com/Noxmore/bevy_trenchbroom/blob/main/readme.md#version-support-table) for which version of ericw-tools you should use for your version of bevy_trenchbroom. It tries to stay near or at the latest version.

## Should you use BSPs?
In favor of using BSPs...
- At the time of writing, BSPs are probably the easiest way to get baked lighting in Bevy, *especially* if you're familiar with the Quake or Source engine mapping stack. (this includes light volumes, bounce lighting, and baked ambient occlusion!)
- Out-of-the-box **animated** baked lighting with up to 253 animations you control.
- Occluded surfaces are deleted during the `bsp` step, reducing overdraw a lot. If your map is completely enclosed, outward facing surfaces are removed as well!

As nice as those features are (especially if you're supporting lower-end systems), there are a few limitations due to it being a level format for the 90s.
- Texture names are limited to 15 characters or less including directories.
- Point/Spot/Directional lights are merged into a single class, with the type of light determined from its properties.
- Light volumes/grids, converted to irradiance volumes for Bevy, doesn't have any directionality*.
- The compile step slows iteration time, though you can also use `.map` files for iterating, and a BSP for the final product.

<sup>* Fake static directionality is included [here](bevy_trenchbroom::config::TrenchBroomConfig::irradiance_volume_multipliers) to make objects appear more 3D.</sup>

Whether you use BSPs depends on the workflow you want and the type of game you are making.

## How to use BSPs
First, enable the `bsp` feature on the crate. You can now load `.bsp` files just like you would `.map`s.

Assuming you're using baked lighting, you should turn off ambient light with `.insert_resource(AmbientLight::NONE)`

For compiling the `.map` into a `.bsp`, many of the compiler's defaults are specifically for compiling for the Quake engine, which has quite a few more limitations than Bevy, so we'll have to remove some with command-line arguments. Here are some recommended ones:

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

As for `vis`, currently, PVS (potentially visible set) data generated by the `vis` tool isn't used.

## BSP Tips

- The base classes [`BspSolidEntity`](bevy_trenchbroom::class::builtin::BspSolidEntity) and [`BspWorldspawn`](bevy_trenchbroom::class::builtin::BspWorldspawn) are provided, and should be inherited on any solid classes and worldspawn implementations respectively, as they provide a ton of [properties used by ericw-tools](https://ericw-tools.readthedocs.io/en/latest/).

- At the time of writing, Bevy's default [`StandardMaterial::lightmap_exposure`](bevy::prelude::StandardMaterial::lightmap_exposure) makes lightmaps completely invisible. The [`TrenchBroomConfig::lightmap_exposure`](bevy_trenchbroom::config::TrenchBroomConfig::lightmap_exposure) setting automatically sets it to a reasonable value on any `StandardMaterial` loaded.

- You can change [`TrenchBroomConfig::bsp_parse_settings`](bevy_trenchbroom::config::TrenchBroomConfig::bsp_parse_settings) and [`compute_lightmap_settings`](bevy_trenchbroom::config::TrenchBroomConfig::compute_lightmap_settings) to further configure how BSPs are loaded.

- If you have harsh lighting and your lightmaps look blocky, try enabling [`TrenchBroomConfig::bicubic_lightmap_filtering`](bevy_trenchbroom::config::TrenchBroomConfig::bicubic_lightmap_filtering), and adding a pixel of padding in [`compute_lightmap_settings`](bevy_trenchbroom::config::TrenchBroomConfig::compute_lightmap_settings). This greatly reduces stair-stepping at the cost of occasional lightmap seams. Though generally textures cover up the seams pretty well.

## Animated Lighting
Each light can specify a [lighting style](qbsp::data::bsp::LightmapStyle) which governs the pattern of animation that light produces over time, affecting both lightmaps, and irradiance volumes.

The patterns are controlled by the [`LightingAnimators`](bevy_trenchbroom::bsp::lighting::LightingAnimators) resource.
Each [`LightingAnimator`](bevy_trenchbroom::bsp::lighting::LightingAnimator) within it contains a sequence of RGB multipliers, a speed, and how much the frames should interpolate between each other.

Here's an example of how to define some animators:
```rust
# use bevy::prelude::*;
# use bevy_trenchbroom::prelude::*;
// System you might run on Startup.
fn setup_lighting_animators(
	mut animators: ResMut<LightingAnimators>,
) {
	// A soft flickering, could be used for a candle.
	animators.values.insert(
		LightmapStyle(1),
		LightingAnimator::new(6., 0.7, [0.8, 0.75, 1., 0.7, 0.8, 0.7, 0.9, 0.7, 0.6, 0.7, 0.9, 1., 0.7].map(Vec3::splat)),
	);
	// Fades from 0 to 1 and back looping with an interval of 2 seconds.
	animators.values.insert(
		LightmapStyle(2),
		LightingAnimator::new(0.5, 1., [0., 1.].map(Vec3::splat)),
	);
}
```

Notice we start at one here. The convention is to reserve 0 for unanimated lighting ([`LightmapStyle::NORMAL`](qbsp::data::bsp::LightmapStyle::NORMAL)).
We're also converting these multipliers to [`Vec3`](bevy::math::Vec3)s through [`Vec3::splat`](bevy::math::Vec3::splat). If you wanted to separate the channels, you could, for instance, make the light flash green every once in a while, or make it cycle through all colors of the rainbow.

The 2 numbers before the pattern govern the speed in frames per second, and how much the pattern interpolates respectively. An interpolation of 0 will switch between frames instantly, 1 will linearly interpolate or fade between them, 0.5 will cause interpolation to happen twice as fast, meaning it finishes interpolating half way and stops, etc.

Each face in the BSP can contain up to 4 different lightmaps of different styles, which in most cases is enough for a good amount of overlapping animation.
