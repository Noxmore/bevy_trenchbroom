#![doc = include_str!("../readme.md")]

#[cfg(all(feature = "rapier", feature = "avian"))]
compile_error!("can only have one collider backend enabled");

pub mod brush;
pub mod bsp_lighting;
pub mod class;
pub mod config;
pub mod definitions;
pub mod load;
pub mod map_entity;
pub mod material_properties;
pub mod prelude;
pub mod spawn;
pub mod special_textures;
pub mod util;

pub(crate) use prelude::*;

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

        app.add_plugins(BspLightingPlugin)
            // I'd rather not clone here, but i only have a reference to self
            .insert_resource(TrenchBroomServer::new(self.config.clone()))
            .init_asset::<Map>()
            .init_asset_loader::<MapLoader>()
            .init_asset_loader::<BspLoader>()
            .init_asset::<MaterialProperties>()
            .init_asset_loader::<MaterialPropertiesLoader>()
            .add_systems(PreUpdate, (Self::reload_maps, Self::spawn_maps));
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
    /// Caches textures used on brushes to [StandardMaterial] handles.
    pub material_cache: Mutex<HashMap<String, Handle<StandardMaterial>>>,
}

#[derive(Component, Clone, Reflect, Debug)]
#[require(Transform, Visibility)]
pub struct MapHandle(pub Handle<Map>);

/// A Quake map loaded from a .map or .bsp file.
#[derive(Asset, Reflect, Debug, Clone, Default)]
pub struct Map {
    /// A title for the map, currently it just mirrors it's path.
    pub name: String,
    pub entities: Vec<Arc<MapEntity>>,
    /// Textures embedded in a BSP file.
    pub embedded_textures: HashMap<String, BspEmbeddedTexture>,
    #[reflect(ignore)]
    pub bsp_data: Option<BspData>,
    pub irradiance_volumes: Vec<(IrradianceVolume, Transform)>,
}

impl Map {
    /// Gets the worldspawn of this map, this will return `Some` on any valid map.
    ///
    /// worldspawn should be the first entity, so normally this will be an `O(1)` operation
    pub fn worldspawn(&self) -> Option<&MapEntity> {
        self.entities
            .iter()
            .find(|ent| ent.classname() == Ok("worldspawn"))
            .map(|v| &**v)
    }
}
