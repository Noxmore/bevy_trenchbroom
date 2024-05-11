use crate::*;

/// An entity read from a TrenchBroom map, although it can also be created manually.
///
/// When put on an entity, it will spawn the contents of this MapEntity into the Bevy world based on your [TrenchBroomConfig].
#[derive(Component, Reflect, Debug, Clone, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct MapEntity {
    /// If this entity was loaded from a [Map], This will be the index of the entity in said map.
    pub ent_index: Option<usize>,
    /// The properties defined in this entity instance.
    /// If you want to get a property that accounts for base classes, use [MapEntityPropertiesView].
    pub properties: HashMap<String, String>,
    pub brushes: Vec<Brush>,
}

impl MapEntity {
    /// Gets the classname of the entity, on any valid entity, this will return `Ok`. Otherwise it will return [MapEntitySpawnError::RequiredPropertyNotFound].
    pub fn classname(&self) -> Result<&str, MapEntitySpawnError> {
        self.properties
            .get("classname")
            .map(String::as_str)
            .ok_or_else(|| MapEntitySpawnError::RequiredPropertyNotFound {
                property: "classname".into(),
            })
    }
}

/// Marker component for a [MapEntity] that has been spawned, to respawn a [MapEntity], remove this component.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SpawnedMapEntity;
