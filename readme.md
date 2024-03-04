# bevy_trenchbroom

Full Bevy integration with TrenchBroom, supporting loading .map files, defining a TrenchBroom game configuration in-code, and more!

# How to use
Simply add the `TrenchBroomPlugin` with a supplied `TrenchBroomConfig` to your app like so:

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
	TrenchBroomConfig::new("boop") // <- The name of your game
		// Here you can customize the resulting game configuration with a builder-like syntax
		.entity_scale_expression("scale")
		// ...
		
		
		// You can define entity definitions here, these are written to your game's FGD file

		// It's highly recommended to make the first defined entity your `worldspawn`
		.define_entity("worldspawn", EntityDefinition::new_solid()
			.description("World Entity")
			
			.property("skybox", EntDefProperty::string().title("Skybox").description("Path to Skybox"))
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

// app.add_systems(Startup, write_trenchbroom_config_system)

fn write_trenchbroom_config_system(config: Res<TrenchBroomConfig>) {
	if let Err(err) = config.write_folder("<folder_path>") {
		error!("Could not write TrenchBroom config: {err}");
	}

	// This will write <folder_path>/GameConfig.cfg, and <folder_path>/boop.fgd
}
```

This writes it out every time your app starts, but depending on what you want to do, you might want to write it out some other time.