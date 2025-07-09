# 0.8 to 0.9
- Derive macros have been converted to attribute macros to greatly reduce boilerplate
	- Replace your `#[derive(<type>Class)]` with `#[<type>_class]`
	- `#[derive(<type>Class, Component, Reflect)]` is now implied if not specified.
	- `#[reflect(QuakeClass, Component)]` is now implied if not specified.
	- Attributes have been put into the macro body, though they are not needed.
	- `spawn_hooks` attribute has been renamed to just `hooks`.
	- The `no_default` field attribute has been renamed to `must_set`.
	- Here's an example:
		```rust
		#[solid_class(
			base(BspWorldspawn),
			hooks(SpawnHooks::new().smooth_by_default_angle()),
		)]
		struct Worldspawn;

		#[solid_class]
		#[derive(Default)]
		struct FuncDoor {
			#[must_set]
			pub my_field: f32,
		}
		```
- Many generic/widely useful classes have been created and automatically registered for convenience.
	- You can remove all of them by disabling the various `*ClassesPlugin`s in the `TrenchBroomPlugins` plugin group, originating from the `BasicClassesPlugins` plugin group.
	- Disable specific classes with `.disable_class::<T>()` in app initialization.

# 0.7 to 0.8
- `TrenchBroomPlugin` has been changed to the `TrenchBroomPlugins` plugin group. The syntax is the exact same by default, you're just able to disable specific plugins now.
- QuakeClass registration has been moved from `TrenchBroomConfig` to Bevy's type registry.
	- Add `#[reflect(QuakeClass, Component, ...)]` to your classes. (To be clear, you had to reflect `Component` before as well)
	- Auto-registration has been removed, see below.
	- Add `.register_type::<T>()` to your app initialization to register your classes.
	- If a class by the same name has already been registered, use `.override_class::<T>()` instead.
- Auto-registration has been removed due to platform problems with wasm, and the move to Bevy's type registry. There are efforts to make [automatic type registration](https://github.com/bevyengine/bevy/pull/15030) happen officially, keep an eye out for that and optionally roll your own in the meantime.
- The `TrenchBroomConfig` configuration writing function has been split and improved.
	- Use `write_game_config_to_default_directory` to write the game config directly into the user's TrenchBroom installation.
	- Use `add_game_to_preferences_in_default_directory` to set the game directory in the user's TrenchBroom settings to the current working directory.
- The default material extension has been changed to `toml` due to the fact that the default material deserializer is `TomlMaterialDeserializer`. This can be changed in `TrenchBroomConfig`.
- `TrenchBroomConfig` now supports multiple texture and material extensions, and those fields have been pluralized to reflect that.
- BSP loading has been locked behind the `bsp` feature flag.
- We now ship with [`bevy_fix_gltf_coordinate_system`](https://github.com/janhohenheim/bevy_fix_gltf_coordinate_system) by default. You can disable it in your `TrenchBroomPlugins` if you don't want it or already add it, but be aware that without it glTFs in-editor won't align with those in-game.
- `GeometryProvider` has been removed in favor of spawn hooks.
	- Change `#[geometry(GeometryProvider::new().<...>)]` to `#[spawn_hooks(SpawnHooks::new().<...>)]`
	- `with_lightmaps` is now opt-out rather than opt-in (`without_lightmaps`)

### bevy_materialize
- Non-color maps in `StandardMaterial` now use a linear color space out of the box, making PBR materials look correctly.
- Material inheritance and processing has been added. See [its readme](https://github.com/Noxmore/bevy_materialize/blob/9d56fb86507ccfe26a4122406aff9bf64de43d3e/readme.md) for an overview of these features.
