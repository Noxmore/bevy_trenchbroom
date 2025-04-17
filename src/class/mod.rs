pub mod builtin;
pub mod spawn_util;

use bevy::asset::LoadContext;
use bevy_reflect::{GetTypeRegistration, TypeRegistration, TypeRegistry};
use geometry::GeometryProvider;
use qmap::QuakeMapEntity;

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

/// Inputs provided when spawning an entity into the scene world of a loading map.
pub struct QuakeClassSpawnView<'l, 'w, 'sw> {
	pub config: &'l TrenchBroomConfig,
	pub src_entity: &'l QuakeMapEntity,
	/// Entity in the scene world.
	pub entity: &'l mut EntityWorldMut<'sw>,
	pub load_context: &'l mut LoadContext<'w>,
}

pub trait QuakeClass: Component + GetTypeRegistration + Sized {
	/// A global [`ErasedQuakeClass`] of this type. Used for base classes and registration.
	///
	/// Everything i've read seems a little vague on this situation, but in testing it seems like this acts like a static.
	const ERASED_CLASS: &ErasedQuakeClass = &ErasedQuakeClass::of::<Self>();
	const CLASS_INFO: QuakeClassInfo;

	/// Spawns into the scene world when the map is loaded.
	fn class_spawn(view: &mut QuakeClassSpawnView) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Copy)]
pub struct ErasedQuakeClass {
	pub info: QuakeClassInfo,
	pub spawn_fn: fn(&mut QuakeClassSpawnView) -> anyhow::Result<()>,
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

	pub fn apply_spawn_fn_recursive(&self, view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		for base in self.info.base {
			base.apply_spawn_fn_recursive(view)?;
		}

		(self.spawn_fn)(view)?;

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
