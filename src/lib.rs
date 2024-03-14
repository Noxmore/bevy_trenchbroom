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

lazy_static! {
    /// In situations where you need [TrenchBroomConfig::scale] outside any system,
    /// this will always mirror the current value of said scale from your app's [TrenchBroomConfig] resource.
    pub static ref TRENCHBROOM_SCALE: RwLock<f32> = default();
}

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
            .register_type::<MaterialProperties>()
            .register_type::<MapEntity>()
            .register_type::<Map>()
            .init_asset::<Map>()
            .init_asset_loader::<MapLoader>()
            .add_event::<MapSpawnedEvent>()
            .add_systems(PreUpdate, (mirror_trenchbroom_scale, spawn_maps));
    }
}

/// A TrenchBroom map loaded from a .map file.
#[derive(Asset, Reflect, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Map {
    pub name: String,
    pub entities: Vec<MapEntity>,
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
