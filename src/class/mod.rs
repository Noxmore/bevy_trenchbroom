pub mod builtin;
pub mod spawn_util;

use bevy::{asset::LoadContext, platform::collections::HashSet};
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
			.register_type::<PreloadedAssets>()
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
	/// Recursively checks if this class is a subclass of `T`. Does not return `true` if this class *is* `T`.
	pub fn derives_from<T: QuakeClass>(&self) -> bool {
		self.base
			.iter()
			.any(|class| class.id() == TypeId::of::<T>() || class.info.derives_from::<T>())
	}

	/// Returns the path of the in-editor model of this class.
	///
	/// TODO: This currently only works for classes with the syntax `#[model("path/to/model")]`, anything more complex will produce `None`.
	///
	/// # Examples
	/// ```
	/// # use bevy::prelude::*;
	/// # use bevy_trenchbroom::prelude::*;
	/// #[derive(PointClass, Reflect, Component)]
	/// #[reflect(Component)]
	/// #[model("models/my_class.glb")]
	/// struct MyClass;
	///
	/// assert_eq!(MyClass::CLASS_INFO.model_path(), Some("models/my_class.glb"));
	/// ```
	pub fn model_path(&self) -> Option<&str> {
		let model = self.model?;
		if !model.starts_with('"') || !model.ends_with('"') {
			return None;
		}
		Some(model.trim_matches('"'))
	}
}

/// Inputs provided when spawning an entity into the scene world of a loading map.
pub struct QuakeClassSpawnView<'l, 'w, 'sw> {
	pub config: &'l TrenchBroomConfig,
	pub src_entity: &'l QuakeMapEntity,
	/// The class of the entity that is being spawned. Not the class of the [`QuakeClass`] in which this view is passed to (if it is a base class).
	pub class: &'l ErasedQuakeClass,
	/// Entity in the scene world.
	pub entity: &'l mut EntityWorldMut<'sw>,
	pub load_context: &'l mut LoadContext<'w>,
}
impl QuakeClassSpawnView<'_, '_, '_> {
	/// Store an asset that you wish to load, but not use for anything yet.
	pub fn preload_asset(&mut self, handle: UntypedHandle) {
		self.entity.entry::<PreloadedAssets>().or_default().get_mut().0.push(handle);
	}
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

/// Function that spawns a [`QuakeClass`] into a scene world. Also used for spawning hooks.
pub type QuakeClassSpawnFn = fn(&mut QuakeClassSpawnView) -> anyhow::Result<()>;

#[derive(Debug, Clone, Copy)]
pub struct ErasedQuakeClass {
	/// The Rust type of this class. Is a function because `TypeId::of` is not yet stable as a const fn.
	pub type_id: fn() -> TypeId,
	pub info: QuakeClassInfo,
	pub spawn_fn: QuakeClassSpawnFn,
	pub get_type_registration: fn() -> TypeRegistration,
	pub register_type_dependencies: fn(&mut TypeRegistry),
}
impl ErasedQuakeClass {
	pub const fn of<T: QuakeClass>() -> Self {
		Self {
			type_id: TypeId::of::<T>,
			info: T::CLASS_INFO,
			spawn_fn: T::class_spawn,
			get_type_registration: T::get_type_registration,
			register_type_dependencies: T::register_type_dependencies,
		}
	}

	pub fn apply_spawn_fn_recursive(&self, view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		self.apply_spawn_fn_recursive_internal(view, &mut default())
	}

	fn apply_spawn_fn_recursive_internal(&self, view: &mut QuakeClassSpawnView, spawned_classes: &mut HashSet<TypeId>) -> anyhow::Result<()> {
		for base in self.info.base {
			if spawned_classes.contains(&base.id()) {
				continue;
			}
			base.apply_spawn_fn_recursive_internal(view, spawned_classes)?;
			spawned_classes.insert(base.id());
		}

		(self.spawn_fn)(view)?;

		Ok(())
	}

	/// The Rust type of this [`QuakeClass`]. Not called `type_id` as to not shadow [`Any`].
	#[inline]
	pub fn id(&self) -> TypeId {
		(self.type_id)()
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

#[cfg(feature = "client")]
#[test]
fn derives_from() {
	use crate::bsp::base_classes::*;

	assert!(PointLight::CLASS_INFO.derives_from::<Transform>());
	assert!(PointLight::CLASS_INFO.derives_from::<Visibility>());
	assert!(!PointLight::CLASS_INFO.derives_from::<BspLight>());
	assert!(!PointLight::CLASS_INFO.derives_from::<BspWorldspawn>());

	assert!(!Transform::CLASS_INFO.derives_from::<Transform>());
}

#[test]
fn spawn_deduplication() {
	use crate::util::*;

	static mut BASE_CALLED: bool = false;
	static mut CLASS_CALLED: bool = false;

	#[derive(BaseClass, Component, Reflect)]
	#[no_register]
	#[spawn_hook(|_| {
		assert!(unsafe { !BASE_CALLED });
		unsafe { BASE_CALLED = true; }
		Ok(())
	})]
	struct Base;

	#[allow(clippy::duplicated_attributes)]
	#[derive(PointClass, Component, Reflect)]
	#[base(Base, Base)]
	#[no_register]
	#[spawn_hook(|_| {
		assert!(unsafe { !CLASS_CALLED });
		unsafe { CLASS_CALLED = true; }
		Ok(())
	})]
	struct Class;

	let asset_server = create_test_asset_server();
	let mut load_context = create_load_context(&asset_server, "".into(), false, false);

	Class::ERASED_CLASS
		.apply_spawn_fn_recursive(&mut QuakeClassSpawnView {
			config: &default(),
			src_entity: &default(),
			class: Class::ERASED_CLASS,
			entity: &mut World::new().spawn_empty(),
			load_context: &mut load_context,
		})
		.unwrap();
}
