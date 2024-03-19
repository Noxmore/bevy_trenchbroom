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

#[derive(Clone, Copy)]
pub struct MapEntityPropertiesView<'w> {
    pub entity: &'w MapEntity,
    pub tb_config: &'w TrenchBroomConfig,
}

impl<'w> MapEntityPropertiesView<'w> {
    /// Gets a property from this entity accounting for entity class hierarchy.
    /// If the property is not defined, it attempts to get its default.
    pub fn get<T: TrenchBroomValue>(&self, key: &str) -> Result<T, MapEntityInsertionError> {
        let Some(value_str) = self.entity.properties.get(key).or(self
            .tb_config
            .get_entity_property_default(self.entity.classname()?, key))
        else {
            return Err(MapEntityInsertionError::RequiredPropertyNotFound {
                property: key.into(),
            });
        };
        T::tb_parse(value_str.trim_matches('"')).map_err(|err| {
            MapEntityInsertionError::PropertyParseError {
                property: key.into(),
                required_type: std::any::type_name::<T>(),
                error: err.to_string(),
            }
        })
    }

    /// Extracts a transform from this entity using the properties `angles`, `origin`, and `scale`.
    /// If you are not using those for your transform, you probably shouldn't use this function.
    pub fn get_transform(&self) -> Transform {
        let rotation = match self.get::<Vec3>("angles")/* .map(Vec3::z_up_to_y_up) */ {
            Ok(rot) => Quat::from_euler(
                // Honestly, i don't know why this works, i got here through hours of trial and error
                EulerRot::default(),
                (rot.y - 90.).to_radians(),
                -rot.x.to_radians(),
                -rot.z.to_radians(),
            ),
            Err(_) => Quat::default(),
        };

        Transform {
            translation: self
                .get::<Vec3>("origin")
                .unwrap_or(Vec3::ZERO)
                .trenchbroom_to_bevy_space(),
            rotation,
            scale: match self.get::<f32>("scale") {
                Ok(scale) => Vec3::splat(scale),
                Err(_) => match self.get::<Vec3>("scale") {
                    Ok(scale) => scale.trenchbroom_to_bevy_space(),
                    Err(_) => Vec3::ONE,
                },
            },
        }
    }
}
