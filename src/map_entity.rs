use crate::*;

/// A component containing all the entity information used to create this map entity.
#[derive(Component, Reflect, Debug, Clone, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub struct MapEntity {
    pub ent_index: usize,
    /// The properties defined in this entity instance.
    /// If you want to get a property that accounts for base classes, use [MapEntityPropertiesView].
    pub properties: HashMap<String, String>,
    pub brushes: Vec<Brush>,
}

impl MapEntity {
    /// Gets the classname of the entity, on any valid entity, this will return `Ok`. Otherwise it will return [MapEntityInsertionError::RequiredPropertyNotFound].
    pub fn classname(&self) -> Result<&str, MapEntityInsertionError> {
        self.properties
            .get("classname")
            .map(String::as_str)
            .ok_or_else(|| MapEntityInsertionError::RequiredPropertyNotFound {
                property: "classname".into(),
            })
    }
}
