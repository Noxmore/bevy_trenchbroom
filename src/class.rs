use crate::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QuakeClassType {
    /// Cannot be spawned in TrenchBroom, works like a base class in any object-oriented language.
    #[default]
    Base,
    /// An entity that revolves around a single point.
    Point,
    /// An entity that contains brushes.
    Solid,
}

/// A property for an entity definition. the property type (`ty`) doesn't have a set of different options, it more just tells users what kind of data you are expecting.
#[derive(Debug, Clone, Default, DefaultBuilder, Serialize, Deserialize)]
pub struct QuakeClassProperty {
    #[builder(skip)]
    pub ty: EntDefPropertyType,
    #[builder(into)]
    pub title: Option<String>,
    #[builder(into)]
    pub description: Option<String>,
    #[builder(skip)]
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuakeClassPropertyType {
    Value(String),
    Choices(Vec<(String, String)>),
}

impl Default for QuakeClassPropertyType {
    fn default() -> Self {
        Self::Value("string".into())
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuakeClassInfo {
    pub ty: QuakeClassType,
    pub name: &'static str,
    pub description: &'static str,
    pub base: &'static [&'static str],
    
    /// A model that the entity shows up as in the editor. See the page on the [TrenchBroom docs](https://trenchbroom.github.io/manual/latest/#display-models-for-entities) for more info.
    pub model: Option<&'static str>,
    pub color: Option<&'static str>,
    /// An icon that the entity appears as in the editor. Takes a single value representing the path to the image to show.
    pub iconsprite: Option<&'static str>,
    /// The size of the bounding box of the entity in the editor.
    pub size: Option<&'static str>,
}

pub struct QuakeClassProperties {
    pub values: IndexMap<String, QuakeClassProperty>,
}

pub trait QuakeClass: Component + Reflect + Sized {
    const CLASS_INFO: QuakeClassInfo;

    fn class_properties(server: &TrenchBroomServer, properties: &mut QuakeClassProperties);
    fn class_insert(server: &TrenchBroomServer, properties: &HashMap<String, String>, entity: EntityWorldMut) -> anyhow::Result<()>; // TODO more specific error?
}

pub struct ErasedQuakeClass {
    pub info: QuakeClassInfo,
    pub properties_fn: fn(&TrenchBroomServer, &mut QuakeClassProperties),
    pub insert_fn: fn(&TrenchBroomServer, &HashMap<String, String>, EntityWorldMut) -> anyhow::Result<()>,
}
impl ErasedQuakeClass {
    pub const fn of<T: QuakeClass>() -> Self {
        Self {
            info: T::CLASS_INFO,
            properties_fn: T::class_properties,
            insert_fn: T::class_insert,
        }
    }
}

#[cfg(feature = "auto_register")]
inventory::collect!(ErasedQuakeClass);