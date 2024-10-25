#![doc = include_str!("../readme.md")]

#[cfg(all(feature = "rapier", feature = "avian"))]
compile_error!("can only have one collider backend enabled");

pub mod brush;
pub mod config;
pub mod definitions;
pub mod load;
pub mod map_entity;
pub mod material_properties;
pub mod prelude;
pub mod spawn;
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
        app
            // I'd rather not clone here, but i only have a reference to self
            .insert_resource(TrenchBroomServer::new(self.config.clone()))
            .register_type::<MapEntity>()
            .register_type::<SpawnedMapEntity>()
            .register_type::<Map>()
            .register_type::<SpawnedMap>()
            .init_asset::<Map>()
            .init_asset_loader::<MapLoader>()
            .init_asset_loader::<BspLoader>()
            .init_asset::<MaterialProperties>()
            .init_asset_loader::<MaterialPropertiesLoader>()
            .add_systems(
                PreUpdate,
                (reload_maps, spawn_maps),
            );
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

/// A Quake map loaded from a .map or .bsp file.
#[derive(Asset, Reflect, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Map {
    /// A title for the map, currently it just mirrors it's path.
    pub name: String,
    pub entities: Vec<MapEntity>,
    /// Textures embedded in a BSP file.
    #[serde(skip)]
    pub embedded_textures: HashMap<String, Handle<Image>>,
}

impl Map {
    /// Gets the worldspawn of this map, this will return `Some` on any valid map.
    ///
    /// worldspawn should be the first entity, so normally this will be an `O(1)` operation
    pub fn worldspawn(&self) -> Option<&MapEntity> {
        self.entities
            .iter()
            .find(|ent| ent.classname() == Ok("worldspawn"))
    }
}

#[derive(Bundle, Default)]
pub struct MapBundle {
    pub map: Handle<Map>,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}
