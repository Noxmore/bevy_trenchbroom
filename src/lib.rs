#![doc = include_str!("../readme.md")]

#[cfg(all(feature = "rapier", feature = "avian"))]
compile_error!("can only have one collider backend enabled");

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
use bsp::BspDataAsset;
pub(crate) use prelude::*;

// Re-exports
pub use anyhow;
pub use bevy_materialize;
pub use indexmap;
#[cfg(feature = "auto_register")]
pub use inventory;
pub use toml;

pub struct TrenchBroomPlugin {
	pub config: TrenchBroomConfig,
}

impl TrenchBroomPlugin {
	/// Creates a new [TrenchBroomPlugin] with the specified config.
	pub fn new(config: TrenchBroomConfig) -> Self {
		Self { config }
	}
}

impl Plugin for TrenchBroomPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MaterializeMarkerPlugin>() {
			app.add_plugins(MaterializePlugin::new(TomlMaterialDeserializer));
		}

		if self.config.special_textures.is_some() {
			app.add_plugins(special_textures::SpecialTexturesPlugin);
		}

		#[cfg(any(feature = "rapier", feature = "avian"))]
		app.add_plugins(physics::PhysicsPlugin);

		#[cfg(feature = "auto_register")]
		{
			let type_registry = app.world().resource::<AppTypeRegistry>();
			let mut type_registry = type_registry.write();
			for class in self.config.class_iter() {
				type_registry.add_registration((class.get_type_registration)());
				(class.register_type_dependencies)(&mut type_registry);
			}
		}

		if self.config.lightmap_exposure.is_some() {
			app.add_systems(Update, Self::set_lightmap_exposure);
		}

		#[rustfmt::skip]
		app
			.add_plugins((bsp::lighting::BspLightingPlugin, geometry::GeometryPlugin))
			// I'd rather not clone here, but i only have a reference to self
			.insert_resource(TrenchBroomServer::new(self.config.clone()))
			.init_asset::<qmap::QuakeMap>()
			.init_asset_loader::<qmap::QuakeMapLoader>()
			.init_asset::<BspDataAsset>()
			.init_asset::<bsp::Bsp>()
			.init_asset_loader::<bsp::BspLoader>()
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

/// The main hub of `bevy_trenchbroom`-related data. Similar to [AssetServer], all data this stores is reference counted and can be easily cloned.
#[derive(Resource, Debug, Clone)]
pub struct TrenchBroomServer {
	data: Arc<TrenchBroomServerData>,
}
impl TrenchBroomServer {
	pub fn new(config: TrenchBroomConfig) -> Self {
		Self {
			data: Arc::new(TrenchBroomServerData {
				config,
				material_cache: default(),
			}),
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
	// TODO remove?
	/// Caches textures used on brushes to [StandardMaterial] handles.
	pub material_cache: Mutex<HashMap<String, Handle<StandardMaterial>>>,
}
