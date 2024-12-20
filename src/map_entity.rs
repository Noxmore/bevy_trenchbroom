use crate::*;

/// An entity read from a TrenchBroom map, though it can also be created manually.
#[derive(Reflect, Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapEntity {
    /// If this entity was loaded from a [Map], This will be the index of the entity in said map.
    pub ent_index: Option<usize>,
    /// The properties defined in this entity instance.
    /// If you want to get a property that accounts for base classes, use [MapEntityPropertiesView].
    pub properties: HashMap<String, String>,
    pub geometry: MapEntityGeometry,
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

/// The geometry that might be stored in a [MapEntity].
#[derive(Reflect, Clone, Serialize, Deserialize)]
pub enum MapEntityGeometry {
    /// Raw brush data that still needs to be computed into meshes.
    Map(Vec<Brush>),

    #[serde(skip)]
    #[reflect(ignore)]
    /// Pre-computed geometry, maps textures to the mesh that uses it.
    Bsp(Vec<(MapEntityGeometryTexture, Mesh)>),
}
impl Default for MapEntityGeometry {
    fn default() -> Self {
        Self::Map(Vec::new())
    }
}
impl fmt::Debug for MapEntityGeometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Map(brushes) => write!(f, "Map(... len: {})", brushes.len()),
            Self::Bsp(meshes) => write!(f, "Bsp(... len: {})", meshes.len()),
        }
    }
}

#[derive(Reflect, Debug, Clone, PartialEq, Eq)]
pub struct MapEntityGeometryTexture {
    pub name: String,
    pub embedded: Option<BspEmbeddedTexture>,
    pub lightmap: Option<Handle<AnimatedLighting>>,
    /// If the texture should be full-bright
    pub special: bool,
}