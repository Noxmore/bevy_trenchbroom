#![doc = include_str!("../readme.md")]

#[cfg(all(feature = "rapier", feature = "avian"))]
compile_error!("can only have one collider backend enabled");

pub mod brush;
pub mod config;
pub mod load;
pub mod prelude;
pub mod util;
pub mod special_textures;
pub mod bsp_lighting;
pub mod class;
pub mod bsp;
pub mod qmap;
pub mod geometry;
pub mod fgd;
#[cfg(any(feature = "rapier", feature = "avian"))]
pub mod physics;

use bsp::BspLoader;
use physics::PhysicsPlugin;
pub(crate) use prelude::*;

// Re-exports
pub use anyhow;
pub use indexmap;
pub use toml;
#[cfg(feature = "auto_register")]
pub use inventory;
pub use bevy_materialize;

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
        if self.config.special_textures.is_some() {
            app.add_plugins(SpecialTexturesPlugin);
        }

        #[cfg(any(feature = "rapier", feature = "avian"))]
        app.add_plugins(PhysicsPlugin);
        
        app
            .add_plugins(BspLightingPlugin)
            // I'd rather not clone here, but i only have a reference to self
            .insert_resource(TrenchBroomServer::new(self.config.clone()))
            .init_asset_loader::<QuakeMapLoader>()
            .init_asset_loader::<BspLoader>()
            .init_asset::<MaterialProperties>()
            .init_asset_loader::<MaterialPropertiesLoader>();
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