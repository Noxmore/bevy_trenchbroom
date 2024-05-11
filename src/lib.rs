#![doc = include_str!("../readme.md")]

#[cfg(all(feature = "rapier", feature = "xpbd"))]
compile_error!("can only have one collider backend enabled");

pub mod brush;
pub mod config;
pub mod definitions;
pub mod loader;
pub mod map_entity;
pub mod material_properties;
pub mod prelude;
pub mod spawning;
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
            .register_type::<SpawnedMapEntity>()
            .register_type::<Map>()
            .register_type::<SpawnedMap>()
            .init_asset::<Map>()
            .init_asset_loader::<MapLoader>()
            .init_asset::<MaterialProperties>()
            .init_asset_loader::<MaterialPropertiesLoader>()
            .add_systems(PreUpdate, (mirror_trenchbroom_config, spawn_maps));

        // Mirror before any schedule is run, so it won't crash on startup systems. (https://github.com/Noxmore/bevy_trenchbroom/issues/1)
        *TRENCHBROOM_CONFIG_MIRROR.write().unwrap() =
            Some(TrenchBroomConfigMirror::new(&self.config));
    }
}

/// A TrenchBroom map loaded from a .map file.
#[derive(Asset, Reflect, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Map {
    /// A title for the map, currently it just mirrors it's path.
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
