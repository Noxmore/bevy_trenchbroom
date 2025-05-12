# 0.7 to 0.8
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

### bevy_materialize
- Non-color maps in `StandardMaterial` now use a linear color space out of the box, making PBR materials look correctly.
- Material inheritance and processing has been added. See [its readme](https://github.com/Noxmore/bevy_materialize/blob/9d56fb86507ccfe26a4122406aff9bf64de43d3e/readme.md) for an overview of these features.