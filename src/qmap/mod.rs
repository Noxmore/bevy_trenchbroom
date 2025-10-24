use brush::Brush;
use fgd::FgdType;
use geometry::BrushesAsset;

use crate::*;

pub mod loader;

pub struct QuakeMapPlugin;
impl Plugin for QuakeMapPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.init_asset::<QuakeMap>()
			.init_asset_loader::<loader::QuakeMapLoader>()
		;
	}
}

/// Quake map loaded from a .map file.
#[derive(Reflect, Asset, Debug, Clone)]
pub struct QuakeMap {
	pub scene: Handle<Scene>,
	pub meshes: Vec<Handle<Mesh>>,
	/// Maps from entity indexes to brush lists.
	pub brush_lists: HashMap<usize, Handle<BrushesAsset>>,
	pub entities: QuakeMapEntities,
}

/// All the entities stored in a quake map, whether `.map` or `.bsp`.
#[derive(Reflect, Debug, Clone, Default, Deref, DerefMut)]
pub struct QuakeMapEntities(pub Vec<QuakeMapEntity>);
impl QuakeMapEntities {
	pub fn from_quake_util(qmap: quake_util::qmap::QuakeMap, config: &TrenchBroomConfig) -> Self {
		let mut entities = Self::default();
		entities.reserve(qmap.entities.len());

		for entity in qmap.entities {
			let properties = entity
				.edict
				.into_iter()
				.map(|(k, v)| (k.to_string_lossy().into(), v.to_string_lossy().into()))
				.collect::<HashMap<String, String>>();

			entities.push(QuakeMapEntity {
				properties,
				brushes: entity.brushes.iter().map(|brush| Brush::from_quake_util(brush, config)).collect(),
			});
		}

		entities
	}

	/// Gets the worldspawn of this map, this will return `Some` on any valid map.
	///
	/// worldspawn should be the first entity, so normally this will be an `O(1)` operation
	pub fn worldspawn(&self) -> Option<&QuakeMapEntity> {
		self.iter().find(|ent| ent.classname() == Ok("worldspawn"))
	}
}

/// A single entity from a quake map, containing the entities property map, and optionally, brushes.
#[derive(Reflect, Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuakeMapEntity {
	/// The properties defined in this entity instance.
	pub properties: HashMap<String, String>,
	/// If the map entity is a [`Solid`](crate::class::QuakeClassType::Solid) entity, this will contain the brushes making it up.
	///
	/// NOTE: If loading from a BSP, this will always be empty. Instead, use the `BRUSHLIST` BSPX lump stored within [`BspModel`](crate::bsp::BspModel).
	#[cfg(feature = "bsp")]
	pub brushes: Vec<Brush>,
	/// If the map entity is a [`Solid`](crate::class::QuakeClassType::Solid) entity, this will contain the brushes making it up.
	#[cfg(not(feature = "bsp"))]
	pub brushes: Vec<Brush>,
}

impl QuakeMapEntity {
	/// Gets the classname of the entity, on any valid entity, this will return `Ok`. Otherwise it will return [`QuakeEntityError::RequiredPropertyNotFound`].
	pub fn classname(&self) -> Result<&str, QuakeEntityError> {
		self.properties
			.get("classname")
			.map(String::as_str)
			.ok_or_else(|| QuakeEntityError::RequiredPropertyNotFound {
				property: "classname".into(),
			})
	}

	/// Helper function to try to parse an [`FgdType`] property from this map entity.
	pub fn get<T: FgdType>(&self, key: &str) -> Result<T, QuakeEntityError> {
		let s = self
			.properties
			.get(key)
			.ok_or_else(|| QuakeEntityError::RequiredPropertyNotFound { property: key.s() })?;

		T::fgd_parse(s).map_err(|err| QuakeEntityError::PropertyParseError {
			property: key.s(),
			value: s.s(),
			required_type: type_name::<T>(),
			error: format!("{err}"),
		})
	}
}

#[derive(Error, Debug, Clone, PartialEq)]
pub enum QuakeEntityError {
	#[error("required property `{property}` not found")]
	RequiredPropertyNotFound { property: String },
	#[error("requires property `{property}` to be a valid `{required_type}` (got `{value}`). Error: {error}")]
	PropertyParseError {
		property: String,
		value: String,
		required_type: &'static str,
		error: String,
	},
	#[error("definition for \"{classname}\" not found")]
	DefinitionNotFound { classname: String },
	#[error("Entity class {classname} has a base of {base_name}, but that class does not exist")]
	InvalidBase { classname: String, base_name: String },
}

pub trait QuakeEntityErrorResultExt {
	type Value;

	/// If this result is a [`RequiredPropertyNotFound`](QuakeEntityError::RequiredPropertyNotFound) error,
	/// returns [`Ok`] with the specified default value, otherwise simply returns `self`.
	fn with_default(self, default: Self::Value) -> Self;
}

impl<T> QuakeEntityErrorResultExt for Result<T, QuakeEntityError> {
	type Value = T;

	fn with_default(self, default: Self::Value) -> Self {
		match self {
			Err(QuakeEntityError::RequiredPropertyNotFound { property: _ }) => Ok(default),
			res => res,
		}
	}
}
