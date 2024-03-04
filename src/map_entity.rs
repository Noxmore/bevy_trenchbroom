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
		self.properties.get("classname")
			.map(String::as_str)
			.ok_or_else(|| MapEntityInsertionError::RequiredPropertyNotFound { ent_index: self.ent_index, property: "classname".into() })
	}
}



#[derive(Error, Debug, Clone, PartialEq)]
pub enum MapEntityInsertionError
{
	#[error("Entity {ent_index} requires property `{property}` to be created")]
	RequiredPropertyNotFound {
		ent_index: usize,
		property: String,
	},
	#[error("Entity {ent_index} requires property `{property}` to be a valid `{required_type}`. Error: ")]
	PropertyParseError {
		ent_index: usize,
		property: String,
		required_type: &'static str,
		error: String,
	},
	#[error("Entity definition for \"{classname}\" not found")]
	DefinitionNotFound {
		classname: String,
	},
	#[error("Entity class {classname} has a base of {base_name}, but that class does not exist")]
	InvalidBase {
		classname: String,
		base_name: String,
	},
}



#[derive(Clone, Copy)]
pub struct MapEntityPropertiesView<'w> {
	pub entity: &'w MapEntity,
	pub tb_config: &'w TrenchBroomConfig,
}

impl<'w> MapEntityPropertiesView<'w>
{
	/// TODO: document
	pub fn require<T: TrenchBroomValue>(&self, key: &str) -> Result<T, MapEntityInsertionError> {
		let Some(value_str) = self.entity.properties.get(key).or(self.tb_config.get_entity_property_default(self.entity.classname()?, key)) else { 
			return Err(MapEntityInsertionError::RequiredPropertyNotFound { ent_index: self.entity.ent_index, property: key.into() });
		};
		T::tb_parse(value_str).map_err(|err| MapEntityInsertionError::PropertyParseError {
			ent_index: self.entity.ent_index, property: key.into(), required_type: std::any::type_name::<T>(), error: err.to_string()
		})
	}

	/// Extracts a transform from this entity using the properties `angles`, `origin`, and `scale`.
	/// If you are not using those for your transform, you probably shouldn't use this function.
	pub fn get_transform(&self) -> Transform
	{
		let rotation = match self.require::<Vec3>("angles") {
			// TODO i think something is wrong here
			Ok(rot) => Quat::from_euler(EulerRot::YXZ, (rot.y - 90.).to_radians(), -rot.x.to_radians(), rot.z.to_radians()),
			Err(_) => Quat::IDENTITY,
		};

		Transform {
			translation: self.require::<Vec3>("origin").unwrap_or(Vec3::ZERO).trenchbroom_to_bevy_space(),
			rotation,
			scale: match self.require::<Vec3>("scale") { Ok(scale) => scale.trenchbroom_to_bevy_space(), Err(_) => Vec3::ONE },
		}
	}
}