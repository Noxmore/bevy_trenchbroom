#![doc = include_str!("../readme.md")]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_doctest_main)]

#[cfg(all(feature = "rapier", feature = "avian"))]
compile_error!("can only have one collider backend enabled");

// For proc macros to be able to use the `bevy_trenchbroom` path.
extern crate self as bevy_trenchbroom;

pub mod brush;
pub mod bsp;
pub mod class;
pub mod config;
pub mod fgd;
pub mod geometry;
#[cfg(any(feature = "rapier", feature = "avian"))]
pub mod physics;
pub mod prelude;
pub mod qmap;
pub mod special_textures;
pub mod util;

use bevy_materialize::MaterializeMarkerPlugin;
pub(crate) use prelude::*;

// Re-exports
pub use anyhow;
pub use bevy_materialize;
pub use indexmap;
#[cfg(feature = "auto_register")]
pub use inventory;
pub use toml;

pub struct TrenchBroomPlugin(pub TrenchBroomConfig);

impl Plugin for TrenchBroomPlugin {
	fn build(&self, app: &mut App) {
		let TrenchBroomPlugin(config) = self;

		if !app.is_plugin_added::<MaterializeMarkerPlugin>() {
			app.add_plugins(MaterializePlugin::new(TomlMaterialDeserializer));
		}

		#[cfg(any(feature = "rapier", feature = "avian"))]
		app.add_plugins(physics::PhysicsPlugin);

		#[cfg(feature = "auto_register")]
		{
			let type_registry = app.world().resource::<AppTypeRegistry>();
			let mut type_registry = type_registry.write();
			for class in config.class_iter() {
				type_registry.add_registration((class.get_type_registration)());
				(class.register_type_dependencies)(&mut type_registry);
			}
		}

		if config.lightmap_exposure.is_some() {
			app.add_systems(Update, Self::set_lightmap_exposure);
		}

		#[rustfmt::skip]
		app
			// I'd rather not clone here, but i only have a reference to self
			.insert_resource(TrenchBroomServer::new(config.clone()))

			.add_plugins((
				fgd::FgdPlugin,
				special_textures::SpecialTexturesPlugin,
				qmap::QuakeMapPlugin,
				bsp::BspPlugin,
				geometry::GeometryPlugin,
			))
		;
	}
}
impl TrenchBroomPlugin {
	pub fn set_lightmap_exposure(
		mut asset_events: EventReader<AssetEvent<StandardMaterial>>,
		mut standard_materials: ResMut<Assets<StandardMaterial>>,
		tb_server: Res<TrenchBroomServer>,
	) {
		let Some(exposure) = tb_server.config.lightmap_exposure else { return };

		for event in asset_events.read() {
			let AssetEvent::Added { id } = event else { continue };
			// Sometimes this is called even when the asset doesn't exist?? TODO
			let Some(material) = standard_materials.get_mut(*id) else { continue };

			material.lightmap_exposure = exposure;
		}
	}
}

/// The main hub of `bevy_trenchbroom`-related data. Similar to [`AssetServer`], all data this stores is reference counted and can be easily cloned.
#[derive(Resource, Debug, Clone)]
pub struct TrenchBroomServer {
	data: Arc<TrenchBroomServerData>,
}
impl TrenchBroomServer {
	pub fn new(config: TrenchBroomConfig) -> Self {
		Self {
			data: Arc::new(TrenchBroomServerData { config }),
		}
	}
}
impl std::ops::Deref for TrenchBroomServer {
	type Target = TrenchBroomServerData;
	fn deref(&self) -> &Self::Target {
		&self.data
	}
}
#[derive(Debug)]
pub struct TrenchBroomServerData {
	pub config: TrenchBroomConfig,
}
