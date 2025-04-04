use bevy_reflect::{GetTypeRegistration, TypeRegistration, TypeRegistry};
use bsp::base_classes::*;
use fgd::FgdType;
use geometry::GeometryProvider;
use qmap::QuakeMapEntity;
use util::{angle_to_quat, angles_to_quat, mangle_to_quat};

use crate::*;

pub struct QuakeClassPlugin;
impl Plugin for QuakeClassPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.register_type::<Target>()
			.register_type::<Targetable>()
		;
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, strum::EnumIs)]
pub enum QuakeClassType {
	/// Cannot be spawned in TrenchBroom, works like a base class in any object-oriented language.
	#[default]
	Base,
	/// An entity that revolves around a single point.
	Point,
	/// An entity that contains brushes.
	Solid(fn() -> GeometryProvider),
}
impl fmt::Display for QuakeClassType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::Base => "Base",
			Self::Point => "Point",
			Self::Solid(_) => "Solid",
		})
	}
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
	Choices(&'static [(ChoicesKey, &'static str)]),
	/// Will show up in editor as a bunch of checkboxes, each defined flag has its own name.
	///
	/// API is different than other variants because of integration with [`enumflags2`].
	Flags(fn() -> Box<dyn Iterator<Item = (u32, &'static str)>>),
}

impl Default for QuakeClassPropertyType {
	fn default() -> Self {
		Self::Value("string")
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoicesKey {
	String(&'static str),
	Integer(i32),
}
impl fmt::Display for ChoicesKey {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::String(s) => write!(f, "\"{s}\""),
			Self::Integer(v) => write!(f, "{v}"),
		}
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
impl QuakeClassInfo {
	/// Recursively checks if this class uses a class by the name of `classname` as a base class. Does not return `true` if this class *is* `classname`.
	///
	/// You should probably use [`Self::derives_from`] instead.
	pub fn derives_from_name(&self, classname: &str) -> bool {
		self.base
			.iter()
			.any(|class| class.info.name == classname || class.info.derives_from_name(classname))
	}

	/// Recursively checks if this class is a subclass of `T`. Does not return `true` if this class *is* `T`.
	pub fn derives_from<T: QuakeClass>(&self) -> bool {
		self.derives_from_name(T::CLASS_INFO.name)
	}
}

pub trait QuakeClass: Component + GetTypeRegistration + Sized {
	/// A global [`ErasedQuakeClass`] of this type. Used for base classes and registration.
	///
	/// NOTE: Everything i've read seems a little vague on this situation, but in testing it seems like this acts like a static.
	const ERASED_CLASS: &ErasedQuakeClass = &ErasedQuakeClass::of::<Self>();
	const CLASS_INFO: QuakeClassInfo;

	fn class_spawn(server: &TrenchBroomConfig, src_entity: &QuakeMapEntity, entity: &mut EntityWorldMut) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Copy)]
pub struct ErasedQuakeClass {
	pub info: QuakeClassInfo,
	pub spawn_fn: fn(&TrenchBroomConfig, &QuakeMapEntity, &mut EntityWorldMut) -> anyhow::Result<()>,
	pub get_type_registration: fn() -> TypeRegistration,
	pub register_type_dependencies: fn(&mut TypeRegistry),
}
impl ErasedQuakeClass {
	pub const fn of<T: QuakeClass>() -> Self {
		Self {
			info: T::CLASS_INFO,
			spawn_fn: T::class_spawn,
			get_type_registration: T::get_type_registration,
			register_type_dependencies: T::register_type_dependencies,
		}
	}

	pub fn apply_spawn_fn_recursive(
		&self,
		config: &TrenchBroomConfig,
		src_entity: &QuakeMapEntity,
		entity: &mut EntityWorldMut,
	) -> anyhow::Result<()> {
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
	inventory::iter::<&'static ErasedQuakeClass>
		.into_iter()
		.copied()
		.map(|class| (class.info.name, class))
		.collect()
});

// ////////////////////////////////////////////////////////////////////////////////
// // BASIC IMPLEMENTATIONS
// ////////////////////////////////////////////////////////////////////////////////

/// Returns the default registry used in [`TrenchBroomConfig`], containing a bunch of useful foundational and utility classes to greatly reduce boilerplate.
pub fn default_quake_class_registry() -> HashMap<&'static str, Cow<'static, ErasedQuakeClass>> {
	// TODO would it be better if we use inventory instead?
	macro_rules! registry {
		{$($(#[$($attrs:meta)*])? $ty:ident),* $(,)?} => {
			[
				$(
					$(#[$($attrs)*])?
					($ty::CLASS_INFO.name, Cow::Borrowed($ty::ERASED_CLASS)
				)),*
			].into()
		};
	}

	registry! {
		Transform,
		#[cfg(feature = "client")]
		Visibility,

		Target,
		Targetable,

		BspSolidEntity,
		BspWorldspawn,
		BspLight,
		BspExternalMap,
	}
}

impl QuakeClass for Transform {
	const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
		ty: QuakeClassType::Base,
		name: "transform",
		description: None,
		base: &[],

		model: None,
		color: None,
		iconsprite: None,
		size: None,

		properties: &[
			QuakeClassProperty {
				ty: Vec3::PROPERTY_TYPE,
				name: "origin",
				title: Some("Translation/Origin"),
				description: None,
				default_value: Some(|| Vec3::ZERO.fgd_to_string_quoted()),
			},
			QuakeClassProperty {
				ty: Vec3::PROPERTY_TYPE,
				name: "angles",
				title: Some("Rotation (pitch yaw roll) in degrees"),
				description: None,
				default_value: Some(|| Vec3::ZERO.fgd_to_string_quoted()),
			},
			QuakeClassProperty {
				ty: Vec3::PROPERTY_TYPE,
				name: "scale",
				title: Some("Scale"),
				description: None,
				default_value: Some(|| Vec3::ONE.fgd_to_string_quoted()),
			},
		],
	};

	fn class_spawn(config: &TrenchBroomConfig, src_entity: &QuakeMapEntity, entity: &mut EntityWorldMut) -> anyhow::Result<()> {
		let rotation = src_entity
			.get::<Vec3>("mangle")
			// According to TrenchBroom docs https://trenchbroom.github.io/manual/latest/#editing-objects
			// “mangle” is interpreted as “yaw pitch roll” if the entity classnames begins with “light”, otherwise it’s a synonym for “angles”
			.map(if src_entity.classname().map(|s| s.starts_with("light")) == Ok(true) {
				mangle_to_quat
			} else {
				angles_to_quat
			})
			.or_else(|_| src_entity.get::<Vec3>("angles").map(angles_to_quat))
			.unwrap_or_else(|_| angle_to_quat(src_entity.get::<f32>("angle").unwrap_or_default()));

		entity.insert(Transform {
			translation: config.to_bevy_space(src_entity.get::<Vec3>("origin").unwrap_or(Vec3::ZERO)),
			rotation,
			scale: match src_entity.get::<f32>("scale") {
				Ok(scale) => Vec3::splat(scale),
				Err(_) => match src_entity.get::<Vec3>("scale") {
					Ok(scale) => scale.xzy(),
					Err(_) => Vec3::ONE,
				},
			},
		});
		println!("insert transform");

		Ok(())
	}
}

#[cfg(feature = "client")]
impl QuakeClass for Visibility {
	const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
		ty: QuakeClassType::Base,
		name: "visibility",
		description: None,
		base: &[],

		model: None,
		color: None,
		iconsprite: None,
		size: None,

		properties: &[QuakeClassProperty {
			#[rustfmt::skip]
			ty: QuakeClassPropertyType::Choices(&[
				(ChoicesKey::String("Inherited"), "Uses the visibility of its parents. If its a root-level entity, it will be visible."),
				(ChoicesKey::String("Hidden"), "Always not rendered, regardless of its parent's visibility."),
				(ChoicesKey::String("Visible"), "Always rendered, regardless of its parent's visibility."),
			]),
			name: "visibility",
			title: Some("Visibility"),
			description: None,
			default_value: Some(|| "\"Inherited\"".s()),
		}],
	};

	fn class_spawn(_config: &TrenchBroomConfig, src_entity: &QuakeMapEntity, entity: &mut EntityWorldMut) -> anyhow::Result<()> {
		let visibility = match src_entity.properties.get("visibility").map(String::as_str) {
			Some("Inherited") => Visibility::Inherited,
			Some("Hidden") => Visibility::Hidden,
			Some("Visible") => Visibility::Visible,
			None => Visibility::default(),
			Some(_) => Err(qmap::QuakeEntityError::PropertyParseError {
				property: "visibility".s(),
				required_type: "Visibility",
				error: "Must be either `Inherited`, `Hidden`, or `Visible`".s(),
			})?,
		};

		entity.insert(visibility);

		Ok(())
	}
}

/// Quake entity IO - Able to target entities with the [`Targetable`] component.
///
/// TODO: this is currently just a skeleton struct, first-class entity IO hasn't been added yet.
#[derive(BaseClass, Component, Reflect, Debug, Clone, SmartDefault, Serialize, Deserialize)]
#[reflect(Component, Default, Serialize, Deserialize)]
#[no_register]
pub struct Target {
	/// If [`Some`], when this entity's IO fires, it will activate all entities with its [`Targetable::targetname`] set to this, with whatever input that functionality that entity has set up.
	pub target: Option<String>,
	/// If [`Some`], when this entity's IO fires, it will kill all entities with its [`Targetable::targetname`] set to this.
	pub killtarget: Option<String>,
}

/// Quake entity IO - Able to be targeted from a [`Target`] component.
///
/// TODO: this is currently just a skeleton struct, first-class entity IO hasn't been added yet.
#[derive(BaseClass, Component, Reflect, Debug, Clone, SmartDefault, Serialize, Deserialize)]
#[reflect(Component, Default, Serialize, Deserialize)]
#[no_register]
pub struct Targetable {
	/// The name for entities with [`Target`] components to point to.
	pub targetname: Option<String>,
}
