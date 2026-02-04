#![doc = include_str!("../readme.md")]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_doctest_main)]

// For proc macros to be able to use the `bevy_trenchbroom` path.
extern crate self as bevy_trenchbroom;

pub mod manual {
	// This module is dedicated to storing the Manual for use with doc tests and to be viewable on docs.rs.
	#![doc = include_str!("../Manual.md")]
}

pub mod brush;
#[cfg(feature = "bsp")]
pub mod bsp;
pub mod class;
pub mod config;
pub mod fgd;
#[cfg(feature = "client")]
pub mod fix_default_sampler;
pub mod geometry;
#[cfg(feature = "physics-integration")]
pub mod physics;
pub mod prelude;
pub mod qmap;
#[cfg(feature = "client")]
pub mod special_textures;
pub mod util;

use bevy::app::PluginGroupBuilder;
use bevy_materialize::MaterializeMarkerPlugin;
pub(crate) use prelude::*;

// Re-exports
pub use anyhow;
pub use bevy_materialize;

#[cfg(feature = "client")]
use crate::util::{DEFAULT_MISSING_TEXTURE_SIZE, create_missing_texture};

/// Contains all the plugins that makes up bevy_trenchbroom. Most of these you don't want to get rid of or change, but there are a few exceptions.
/// - If you want to change the [`LightingWorkflow`](class::builtin::LightingWorkflow) you're using, set [`LightingClassesPlugin`](class::builtin::LightingClassesPlugin).
/// - [`WriteTrenchBroomConfigOnStartPlugin`](config::WriteTrenchBroomConfigOnStartPlugin) writes the [`TrenchBroomConfig`] on startup, disable if you want to write it out a different time.
pub struct TrenchBroomPlugins(pub TrenchBroomConfig);

impl PluginGroup for TrenchBroomPlugins {
	fn build(self) -> PluginGroupBuilder {
		let builder = PluginGroupBuilder::start::<Self>()
			.add(CorePlugin(self.0))
			.add(class::QuakeClassPlugin)
			.add_group(class::builtin::BasicClassesPlugins)
			.add(qmap::QuakeMapPlugin)
			.add(geometry::GeometryPlugin)
			.add(util::UtilPlugin);

		// Have to use let here because "attributes on expressions are experimental"
		#[cfg(feature = "client")]
		let builder = builder
			.add(special_textures::SpecialTexturesPlugin)
			.add(fix_default_sampler::RepeatDefaultSamplerPlugin);

		#[cfg(feature = "bsp")]
		let builder = builder.add(bsp::BspPlugin);

		#[cfg(all(not(target_family = "wasm"), feature = "client"))]
		let builder = builder.add(config::WriteTrenchBroomConfigOnStartPlugin);

		builder
	}
}

/// The plugin at the center of bevy_trenchbroom. Inserts the [`TrenchBroomServer`], [`MaterializePlugin`], and some tiny miscellaneous things.
pub struct CorePlugin(pub TrenchBroomConfig);

impl Plugin for CorePlugin {
	fn build(&self, app: &mut App) {
		let CorePlugin(config) = self;

		// This isn't part of the plugin group because the generics would make it annoying to disable if you were to add your own `MaterializePlugin`.
		if !app.is_plugin_added::<MaterializeMarkerPlugin>() {
			app.add_plugins(MaterializePlugin::new(TomlMaterialDeserializer));
		}

		#[cfg(all(feature = "client", feature = "bsp"))]
		if config.lightmap_exposure.is_some() {
			app.add_systems(Update, Self::set_lightmap_exposure);
		}

		let asset_server = app.world().resource::<AssetServer>();

		// I'd rather not clone here, but i only have a reference to self
		app.insert_resource(TrenchBroomServer::new(config.clone(), asset_server));
	}
}
impl CorePlugin {
	#[cfg(all(feature = "client", feature = "bsp"))]
	pub fn set_lightmap_exposure(
		mut asset_events: MessageReader<AssetEvent<StandardMaterial>>,
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
	pub fn new(config: TrenchBroomConfig, asset_server: &AssetServer) -> Self {
		Self {
			data: Arc::new(TrenchBroomServerData {
				config,
				#[cfg(feature = "client")]
				missing_material: RwLock::new(asset_server.add(GenericMaterial::new(asset_server.add(StandardMaterial {
					base_color_texture: Some(asset_server.add(create_missing_texture())),
					// Instead of scaling UVs in the mesh, we scale down the missing texture itself.
					uv_transform: Affine2::from_scale(1. / Vec2::splat(DEFAULT_MISSING_TEXTURE_SIZE as f32)),
					..default()
				})))),
				#[cfg(not(feature = "client"))]
				missing_material: RwLock::new(asset_server.add(GenericMaterial::default())),
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
	/// The material used when a texture can't be found when loading a map.
	pub missing_material: RwLock<Handle<GenericMaterial>>,
}
