#![doc = include_str!("../readme.md")]

pub mod brush;
pub mod config;
pub mod definitions;
pub mod insertion;
pub mod loader;
pub mod map_entity;
pub mod material_properties;
pub mod prelude;
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
            .insert_resource(self.config.clone())
            .register_type::<MapEntity>()
            .register_type::<Map>()
            .register_type::<SpawnedMap>()
            .register_type::<MapSpawningSettings>()
            .init_asset::<Map>()
            .init_asset_loader::<MapLoader>()
            .init_asset::<MaterialProperties>()
            .init_asset_loader::<MaterialPropertiesLoader>()
            .add_systems(PreUpdate, (mirror_trenchbroom_config, spawn_maps));
    }
}

/// A TrenchBroom map loaded from a .map file.
#[derive(Asset, Reflect, Debug, Clone, Default)]
pub struct Map {
    /// A title for the map, currently it just mirrors it's path.
    pub name: String,
    pub entities: Vec<MapEntity>,
    /// The material properties required by the textures of the map.
    pub material_properties_map: HashMap<String, Handle<MaterialProperties>>,
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
