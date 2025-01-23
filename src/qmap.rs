use bevy::asset::{AssetLoader, AsyncReadExt};
use brush::Brush;
use fgd::FgdType;

use crate::*;

#[derive(Reflect, Asset, Debug, Clone, Default)]
pub struct QuakeMapEntities {
    pub entities: Vec<QuakeMapEntity>,
}
impl QuakeMapEntities {
    pub fn from_quake_util(qmap: quake_util::qmap::QuakeMap, config: &TrenchBroomConfig) -> Self {
        let mut map = Self::default();
        map.entities.reserve(qmap.entities.len());

        for entity in qmap.entities {
            let properties = entity.edict
                .into_iter()
                .map(|(k, v)| (k.to_string_lossy().into(), v.to_string_lossy().into()))
                .collect::<HashMap<String, String>>();

            map.entities.push(QuakeMapEntity {
                properties,
                brushes: entity.brushes.iter().map(|brush | Brush::from_quake_util(brush, config)).collect(),
            });
        }

        map
    }

    /// Gets the worldspawn of this map, this will return `Some` on any valid map.
    ///
    /// worldspawn should be the first entity, so normally this will be an `O(1)` operation
    pub fn worldspawn(&self) -> Option<&QuakeMapEntity> {
        self.entities
            .iter()
            .find(|ent| ent.classname() == Ok("worldspawn"))
            .map(|v| &*v)
    }
}

#[derive(Reflect, Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuakeMapEntity {
    /// The properties defined in this entity instance.
    pub properties: HashMap<String, String>,
    pub brushes: Vec<Brush>,
}

impl QuakeMapEntity {
    /// Gets the classname of the entity, on any valid entity, this will return `Ok`. Otherwise it will return [QuakeEntityError::RequiredPropertyNotFound].
    pub fn classname(&self) -> Result<&str, QuakeEntityError> {
        self.properties
            .get("classname")
            .map(String::as_str)
            .ok_or_else(|| QuakeEntityError::RequiredPropertyNotFound {
                property: "classname".into(),
            })
    }

    /// Helper function to try to parse an [FgdType] property from this map entity.
    pub fn get<T: FgdType>(&self, key: &str) -> Result<T, QuakeEntityError> {
        let s = self.properties.get(key)
            .ok_or_else(|| QuakeEntityError::RequiredPropertyNotFound { property: key.s() })?;

        T::fgd_parse(s)
            .map_err(|err| QuakeEntityError::PropertyParseError { property: key.s(), required_type: type_name::<T>(), error: format!("{err}") })
    }
}

#[derive(Error, Debug, Clone, PartialEq)]
pub enum QuakeEntityError {
    #[error("required property `{property}` not found")]
    RequiredPropertyNotFound { property: String },
    #[error("requires property `{property}` to be a valid `{required_type}`. Error: {error}")]
    PropertyParseError {
        property: String,
        required_type: &'static str,
        error: String,
    },
    #[error("definition for \"{classname}\" not found")]
    DefinitionNotFound { classname: String },
    #[error("Entity class {classname} has a base of {base_name}, but that class does not exist")]
    InvalidBase {
        classname: String,
        base_name: String,
    },
}

pub struct QuakeMapLoader {
    pub tb_server: TrenchBroomServer,
}
impl FromWorld for QuakeMapLoader {
    fn from_world(world: &mut World) -> Self {
        Self {
            tb_server: world.resource::<TrenchBroomServer>().clone(),
        }
    }
}
impl AssetLoader for QuakeMapLoader {
    // TODO this should be some asset version of QuakeMap
    type Asset = QuakeMapEntities;
    type Settings = ();
    type Error = anyhow::Error;
    
    fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        _load_context: &mut bevy::asset::LoadContext,
    ) -> impl bevy::utils::ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async {
            let mut input = String::new();
            reader.read_to_string(&mut input).await?;

            let quake_util_map = quake_util::qmap::parse(&mut io::Cursor::new(input))?;

            Ok(QuakeMapEntities::from_quake_util(quake_util_map, &self.tb_server.config))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["map"]
    }
}