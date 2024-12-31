use geometry::GeometryProvider;
use qmap::{QuakeEntityError, QuakeMapEntity};

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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuakeClassProperty {
    pub ty: QuakeClassPropertyType,
    pub title: Option<String>,
    pub description: Option<String>,
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
    pub description: Option<&'static str>,
    pub base: &'static [&'static str],
    
    /// A model that the entity shows up as in the editor. See the page on the [TrenchBroom docs](https://trenchbroom.github.io/manual/latest/#display-models-for-entities) for more info.
    pub model: Option<&'static str>,
    pub color: Option<&'static str>,
    /// An icon that the entity appears as in the editor. Takes a single value representing the path to the image to show.
    pub iconsprite: Option<&'static str>,
    /// The size of the bounding box of the entity in the editor.
    pub size: Option<&'static str>,
}

#[derive(Debug, Clone, Default, Deref, DerefMut)]
pub struct QuakeClassProperties {
    pub values: IndexMap<String, QuakeClassProperty>,
}
impl QuakeClassProperties {
    pub fn new() -> Self {
        Self::default()
    }
}

pub trait QuakeClass: Component + Reflect + Default {
    const CLASS_INFO: QuakeClassInfo;

    fn class_properties(server: &TrenchBroomConfig, properties: &mut QuakeClassProperties);
    fn class_insert(server: &TrenchBroomConfig, src_entity: &QuakeMapEntity, entity: &mut EntityWorldMut) -> anyhow::Result<()>; // TODO more specific error?
    fn geometry_provider(src_entity: &QuakeMapEntity) -> Option<GeometryProvider> {
        let _ = src_entity;
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ErasedQuakeClass {
    pub info: QuakeClassInfo,
    pub properties_fn: fn(&TrenchBroomConfig, &mut QuakeClassProperties),
    pub insert_fn: fn(&TrenchBroomConfig, &QuakeMapEntity, &mut EntityWorldMut) -> anyhow::Result<()>,
    pub geometry_provider_fn: fn(&QuakeMapEntity) -> Option<GeometryProvider>,
}
impl ErasedQuakeClass {
    pub const fn of<T: QuakeClass>() -> Self {
        Self {
            info: T::CLASS_INFO,
            properties_fn: T::class_properties,
            insert_fn: T::class_insert,
            geometry_provider_fn: T::geometry_provider,
        }
    }

    pub fn apply_insert_fn_recursive(&self, config: &TrenchBroomConfig, src_entity: &QuakeMapEntity, entity: &mut EntityWorldMut) -> anyhow::Result<()> {
        for base in self.info.base {
            let Some(class) = config.get_class(*base) else {
                return Err(anyhow::anyhow!("Class `{}` has invalid base class `{base}`, class does not exist.", self.info.name));
            };

            class.apply_insert_fn_recursive(config, src_entity, entity)?;
        }

        (self.insert_fn)(config, src_entity, entity)?;
        
        Ok(())
    }
}

#[cfg(feature = "auto_register")]
inventory::collect!(ErasedQuakeClass);

#[cfg(feature = "auto_register")]
pub static GLOBAL_CLASS_REGISTRY: Lazy<HashMap<&'static str, &'static ErasedQuakeClass>> = Lazy::new(|| {
    inventory::iter::<ErasedQuakeClass>.into_iter().map(|class| (class.info.name, class)).collect()
});

//////////////////////////////////////////////////////////////////////////////////
//// BASIC IMPLEMENTATIONS
//////////////////////////////////////////////////////////////////////////////////


impl QuakeClass for Transform {
    const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
        ty: QuakeClassType::Base,
        name: "Transform",
        description: None,
        base: &[],

        model: None,
        color: None,
        iconsprite: None,
        size: None, // TODO should this be Some("size")?
    };

    fn class_properties(_config: &TrenchBroomConfig, properties: &mut QuakeClassProperties) {
        // TODO what about brush entities?
        // TODO use fgd_type
        properties.values.insert("origin".s(), QuakeClassProperty {
            ty: Vec3::fgd_type(),
            title: Some("Translation".s()),
            description: None,
            default_value: Some("0 0 0".s()),
        });
        properties.values.insert("angles".s(), QuakeClassProperty {
            ty: Vec3::fgd_type(),
            title: Some("Rotation (pitch yaw roll) in degrees".s()),
            description: None,
            default_value: Some("0 0 0".s()),
        });
        properties.values.insert("scale".s(), QuakeClassProperty {
            ty: Vec3::fgd_type(),
            title: Some("Scale".s()),
            description: None,
            default_value: Some("1 1 1".s()),
        });
    }

    fn class_insert(config: &TrenchBroomConfig, src_entity: &QuakeMapEntity, entity: &mut EntityWorldMut) -> anyhow::Result<()> {
        let rotation = src_entity.get::<Vec3>("angles").map(angles_to_quat)
            .or_else(|_| src_entity.get::<Vec3>("mangle")
            // According to TrenchBroom docs https://trenchbroom.github.io/manual/latest/#editing-objects
            // “mangle” is interpreted as “yaw pitch roll” if the entity classnames begins with “light”, otherwise it’s a synonym for “angles”
            .map(if src_entity.classname().map(|s| s.starts_with("light")) == Ok(true) {mangle_to_quat} else {angles_to_quat}))
            .unwrap_or_else(|_| angle_to_quat(src_entity.get::<f32>("angle").unwrap_or_default()));

        entity.insert(Transform {
            translation: config.to_bevy_space(src_entity
                .get::<Vec3>("origin")
                .unwrap_or(Vec3::ZERO)),
            rotation,
            scale: match src_entity.get::<f32>("scale") {
                Ok(scale) => Vec3::splat(scale),
                Err(_) => match src_entity.get::<Vec3>("scale") {
                    Ok(scale) => scale.xzy(),
                    Err(_) => Vec3::ONE,
                },
            },
        });

        Ok(())
    }
}

impl QuakeClass for Visibility {
    const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
        ty: QuakeClassType::Base,
        name: "Visibility",
        description: None,
        base: &[],

        model: None,
        color: None,
        iconsprite: None,
        size: None,
    };

    fn class_properties(_config: &TrenchBroomConfig, properties: &mut QuakeClassProperties) {
        properties.values.insert("visibility".s(), QuakeClassProperty {
            ty: QuakeClassPropertyType::Choices(vec![
                ("inherited".fgd_to_string_quoted(), "inherited".s()),
                ("hidden".fgd_to_string_quoted(), "hidden".s()),
                ("visible".fgd_to_string_quoted(), "visible".s()),
            ]),
            title: Some("Visibility".s()),
            description: None,
            default_value: Some("inherited".s()),
        });
    }

    fn class_insert(_config: &TrenchBroomConfig, src_entity: &QuakeMapEntity, entity: &mut EntityWorldMut) -> anyhow::Result<()> {
        let visibility = match src_entity.properties.get("visibility").map(String::as_str) {
            Some("inherited") => Visibility::Inherited,
            Some("hidden") => Visibility::Hidden,
            Some("visible") => Visibility::Visible,
            None => Err(QuakeEntityError::RequiredPropertyNotFound { property: "visibility".s() })?,
            Some(_) => Err(QuakeEntityError::PropertyParseError {
                property: "visibility".s(),
                required_type: "Visibility",
                error: "Must be either `inherited`, `hidden`, or `visible`".s(),
            })?,
        };
        
        entity.insert(visibility);

        Ok(())
    }
}