use fgd::FgdType;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuakeClassProperty {
    pub ty: QuakeClassPropertyType,
    pub name: &'static str,
    pub title: Option<&'static str>,
    pub description: Option<&'static str>,
    pub default_value: Option<fn() -> String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuakeClassPropertyType {
    Value(&'static str),
    Choices(&'static [(&'static str, &'static str)]),
}

impl Default for QuakeClassPropertyType {
    fn default() -> Self {
        Self::Value("string".into())
    }
}


#[derive(Debug, Clone, Copy)]
pub struct QuakeClassInfo {
    pub ty: QuakeClassType,
    /// The name of the class, this is usually the snake_case version of the type's name.
    pub name: &'static str,
    pub description: Option<&'static str>,
    pub base: &'static [&'static ErasedQuakeClass],
    
    /// A model that the entity shows up as in the editor. See the page on the [TrenchBroom docs](https://trenchbroom.github.io/manual/latest/#display-models-for-entities) for more info.
    pub model: Option<&'static str>,
    pub color: Option<&'static str>,
    /// An icon that the entity appears as in the editor. Takes a single value representing the path to the image to show.
    pub iconsprite: Option<&'static str>,
    /// The size of the bounding box of the entity in the editor.
    pub size: Option<&'static str>,

    pub properties: &'static [QuakeClassProperty],
}

pub trait QuakeClass: Component {
    const ERASED_CLASS_INSTANCE: &ErasedQuakeClass;
    const CLASS_INFO: QuakeClassInfo;

    fn class_spawn(server: &TrenchBroomConfig, src_entity: &QuakeMapEntity, entity: &mut EntityWorldMut) -> anyhow::Result<()>; // TODO more specific error?
    fn geometry_provider(src_entity: &QuakeMapEntity) -> Option<GeometryProvider> {
        let _ = src_entity;
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ErasedQuakeClass {
    pub info: QuakeClassInfo,
    pub spawn_fn: fn(&TrenchBroomConfig, &QuakeMapEntity, &mut EntityWorldMut) -> anyhow::Result<()>,
    pub geometry_provider_fn: fn(&QuakeMapEntity) -> Option<GeometryProvider>,
}
impl ErasedQuakeClass {
    pub const fn of<T: QuakeClass>() -> Self {
        Self {
            info: T::CLASS_INFO,
            spawn_fn: T::class_spawn,
            geometry_provider_fn: T::geometry_provider,
        }
    }

    pub fn apply_spawn_fn_recursive(&self, config: &TrenchBroomConfig, src_entity: &QuakeMapEntity, entity: &mut EntityWorldMut) -> anyhow::Result<()> {
        for base in self.info.base {
            base.apply_spawn_fn_recursive(config, src_entity, entity)?;
        }

        (self.spawn_fn)(config, src_entity, entity)?;
        
        Ok(())
    }
}

#[cfg(feature = "auto_register")]
inventory::collect!(&'static ErasedQuakeClass);

#[cfg(feature = "auto_register")]
pub static GLOBAL_CLASS_REGISTRY: Lazy<HashMap<&'static str, &'static ErasedQuakeClass>> = Lazy::new(|| {
    inventory::iter::<&'static ErasedQuakeClass>.into_iter().copied().map(|class| (class.info.name, class)).collect()
});

//////////////////////////////////////////////////////////////////////////////////
//// BASIC IMPLEMENTATIONS
//////////////////////////////////////////////////////////////////////////////////


impl QuakeClass for Transform {
    const ERASED_CLASS_INSTANCE: &ErasedQuakeClass = &ErasedQuakeClass::of::<Self>();
    const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
        ty: QuakeClassType::Base,
        name: "transform",
        description: None,
        base: &[],

        model: None,
        color: None,
        iconsprite: None,
        size: None, // TODO should this be Some("size")?

        properties: &[
            QuakeClassProperty {
                ty: Vec3::PROPERTY_TYPE,
                name: "origin",
                title: Some("Translation/Origin"),
                description: None,
                default_value: Some(|| Vec3::ZERO.fgd_to_string()),
            },
            QuakeClassProperty {
                ty: Vec3::PROPERTY_TYPE,
                name: "angles",
                title: Some("Rotation (pitch yaw roll) in degrees"),
                description: None,
                default_value: Some(|| Vec3::ZERO.fgd_to_string()),
            },
            QuakeClassProperty {
                ty: Vec3::PROPERTY_TYPE,
                name: "scale",
                title: Some("Scale"),
                description: None,
                default_value: Some(|| Vec3::ONE.fgd_to_string()),
            },
        ],
    };

    fn class_spawn(config: &TrenchBroomConfig, src_entity: &QuakeMapEntity, entity: &mut EntityWorldMut) -> anyhow::Result<()> {
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
#[cfg(feature = "auto_register")]
inventory::submit! { Transform::ERASED_CLASS_INSTANCE }

impl QuakeClass for Visibility {
    const ERASED_CLASS_INSTANCE: &ErasedQuakeClass = &ErasedQuakeClass::of::<Self>();
    const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
        ty: QuakeClassType::Base,
        name: "visibility",
        description: None,
        base: &[],

        model: None,
        color: None,
        iconsprite: None,
        size: None,

        properties: &[
            QuakeClassProperty {
                ty: QuakeClassPropertyType::Choices(&[
                    ("\"Inherited\"", "Inherited"),
                    ("\"Hidden\"", "Hidden"),
                    ("\"Visible\"", "Visible"),
                ]),
                name: "visibility",
                title: Some("Visibility"),
                description: None,
                default_value: Some(|| "Inherited".s()),
            },
        ],
    };

    fn class_spawn(_config: &TrenchBroomConfig, src_entity: &QuakeMapEntity, entity: &mut EntityWorldMut) -> anyhow::Result<()> {
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
#[cfg(feature = "auto_register")]
inventory::submit! { Visibility::ERASED_CLASS_INSTANCE }
