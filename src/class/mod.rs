pub mod builtin;
pub mod scene_hooks;

use bevy::{asset::LoadContext, platform::collections::HashSet};
use bevy_reflect::{FromType, GetTypeRegistration, TypeRegistry};
use qmap::QuakeMapEntity;

use crate::{geometry::MapGeometryTexture, util::MapFileType, *};

pub struct QuakeClassPlugin;
impl Plugin for QuakeClassPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.add_systems(Startup, Self::verify_classes)
		;
	}
}
impl QuakeClassPlugin {
	pub fn verify_classes(type_registry: Res<AppTypeRegistry>) {
		let type_registry = type_registry.read();

		let mut map: HashMap<&str, Vec<&str>> = HashMap::new();

		for (registration, reflected_class) in type_registry.iter_with_data::<ReflectQuakeClass>() {
			if !reflected_class.enabled {
				continue;
			}
			map.entry(reflected_class.erased_class.info.name)
				.or_default()
				.push(registration.type_info().type_path());
		}

		for (classname, registrations) in map {
			if registrations.len() > 1 {
				error!(
					"Class {classname:?} has been registered by more than one type: [{}] Did you forget to do `override_class::<T>()` instead of `register_type::<T>()`?",
					registrations.join(", ")
				);
			}
		}
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
	Solid,
}
impl fmt::Display for QuakeClassType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::Base => "Base",
			Self::Point => "Point",
			Self::Solid => "Solid",
		})
	}
}

/// A property for an entity definition. the property type (`ty`) doesn't have a set of different options, it more just tells users what kind of data you are expecting.
#[derive(Debug, Clone, Copy)]
pub struct QuakeClassProperty {
	pub ty: QuakeClassPropertyType,
	pub name: &'static str,
	pub title: Option<&'static str>,
	pub description: Option<&'static str>,
	pub default_value: Option<fn() -> String>,
}

#[derive(Debug, Clone, Copy)]
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
	pub decal: bool,

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
	/// #[point_class(
	///     model("models/my_class.glb"),
	/// )]
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

/// Generates a map of classnames to classes from a type registry.
pub fn generate_class_map(registry: &TypeRegistry) -> HashMap<&'static str, &'static ErasedQuakeClass> {
	registry
		.iter_with_data::<ReflectQuakeClass>()
		.filter(|(_, class)| class.enabled)
		.map(|(_, class)| (class.erased_class.info.name, class.erased_class))
		.collect()
}

/// Inputs provided when spawning an entity into the scene world of a loading map.
pub struct QuakeClassSpawnView<'l, 'w, 'sw> {
	// 'l: local, 'w: world, 'sw: scene world
	/// The file type of the map being loaded.
	pub file_type: MapFileType,
	pub tb_config: &'l TrenchBroomConfig,
	pub type_registry: &'l TypeRegistry,
	/// A map of classnames to classes.
	pub class_map: &'l HashMap<&'static str, &'static ErasedQuakeClass>,
	pub src_entity: &'l QuakeMapEntity,
	pub src_entity_idx: usize,
	/// The class of the entity that is being spawned. Not the class of the [`QuakeClass`] in which this view is passed to (if it is a base class).
	pub class: &'l ErasedQuakeClass,
	/// The scene world being written to.
	pub world: &'sw mut World,
	/// Entity in the scene world.
	pub entity: Entity,
	pub load_context: &'l mut LoadContext<'w>,

	/// Information about the mesh entities this entity contains.
	pub meshes: &'l mut Vec<QuakeClassMeshView<'l>>,
}
impl QuakeClassSpawnView<'_, '_, '_> {
	/// Store an asset that you wish to load, but not use for anything yet.
	pub fn preload_asset(&mut self, handle: UntypedHandle) {
		self.world
			.entity_mut(self.entity)
			.entry::<PreloadedAssets>()
			.or_default()
			.get_mut()
			.0
			.push(handle);
	}
}

/// Represents a mesh, its texture, and its associated entity under [`QuakeClassSpawnView`].
pub struct QuakeClassMeshView<'l> {
	pub entity: Entity,
	pub mesh: &'l mut Mesh,
	pub texture: &'l mut MapGeometryTexture,
}

pub trait QuakeClass: Component + Reflect + GetTypeRegistration + Sized {
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
}
impl ErasedQuakeClass {
	pub const fn of<T: QuakeClass>() -> Self {
		Self {
			type_id: TypeId::of::<T>,
			info: T::CLASS_INFO,
			spawn_fn: T::class_spawn,
		}
	}

	/// Calls [`Self::spawn_fn`] recursively for all base classes. For almost all cases, you should use the [`spawn_quake_entity_into_scene`] function instead of this.
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

/// Fully spawns a Quake entity into a scene through a [`QuakeClassSpawnView`], calling [`ErasedQuakeClass::spawn_fn`] recursively for all base classes, as well as pre and post scene hooks.
pub fn spawn_quake_entity_into_scene(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
	// We use string formatting because I am not a fan of anyhow's context adding system
	(view.tb_config.pre_spawn_hook)(view).map_err(|err| anyhow!("pre_spawn_hook: {err}"))?;
	view.class.apply_spawn_fn_recursive(view)?;
	(view.tb_config.post_spawn_hook)(view).map_err(|err| anyhow!("post_spawn_hook: {err}"))
}

/// Reflects [`QuakeClass::ERASED_CLASS`]. Any type with this data in the type registry will be considered a registered [`QuakeClass`], unless not [`enabled`](Self::enabled).
#[derive(Clone)]
pub struct ReflectQuakeClass {
	pub erased_class: &'static ErasedQuakeClass,
	pub enabled: bool,
}
impl<T: QuakeClass> FromType<T> for ReflectQuakeClass {
	fn from_type() -> Self {
		Self {
			erased_class: T::ERASED_CLASS,
			enabled: true,
		}
	}
}

pub trait QuakeClassAppExt {
	/// Stops a specific [`QuakeClass`] from being considered when spawning or writing an fgd, effectively unregistering it.
	///
	/// We can't do this by just unregistering the type because at the time of writing, that isn't public API.
	fn disable_class<T: QuakeClass>(&mut self) -> &mut Self;
	/// Registers a class after disabling all other classes with the same classname.
	fn override_class<T: QuakeClass>(&mut self) -> &mut Self;
}
impl QuakeClassAppExt for App {
	#[track_caller]
	fn disable_class<T: QuakeClass>(&mut self) -> &mut Self {
		let mut type_registry = self.world().resource::<AppTypeRegistry>().write();

		type_registry
			.get_mut(TypeId::of::<T>())
			.expect("Class not registered!")
			.data_mut::<ReflectQuakeClass>()
			.expect("Class not reflected, did you forget to add #[reflect(QuakeClass)]?")
			.enabled = false;

		drop(type_registry);
		self
	}
	fn override_class<T: QuakeClass>(&mut self) -> &mut Self {
		let mut type_registry = self.world().resource::<AppTypeRegistry>().write();

		for registration in type_registry.iter_mut() {
			let type_id = registration.type_info().type_id();
			let Some(reflected_class) = registration.data_mut::<ReflectQuakeClass>() else { continue };

			if type_id == TypeId::of::<T>() {
				reflected_class.enabled = true;
			} else if reflected_class.erased_class.info.name == T::CLASS_INFO.name {
				reflected_class.enabled = false;
			}
		}

		drop(type_registry);
		self.register_type::<T>()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[cfg(all(feature = "client", feature = "bsp"))]
	#[test]
	fn derives_from() {
		use builtin::*;

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

		#[base_class(
			hooks(SceneHooks::new().push(|_| {
				assert!(unsafe { !BASE_CALLED });
				unsafe { BASE_CALLED = true; }
				Ok(())
			}))
		)]
		#[reflect(no_auto_register)]
		struct Base;

		#[point_class(
			base(Base, Base),
			hooks(SceneHooks::new().push(|_| {
				assert!(unsafe { !CLASS_CALLED });
				unsafe { CLASS_CALLED = true; }
				Ok(())
			}))
		)]
		#[reflect(no_auto_register)]
		struct Class;

		let asset_server = create_test_asset_server();
		let mut load_context = create_load_context(&asset_server, "".into(), false, false);
		let mut world = World::new();
		let entity = world.spawn_empty().id();

		Class::ERASED_CLASS
			.apply_spawn_fn_recursive(&mut QuakeClassSpawnView {
				file_type: MapFileType::Map,
				tb_config: &default(),
				type_registry: &default(),
				class_map: &default(),
				src_entity: &default(),
				src_entity_idx: 0,
				class: Class::ERASED_CLASS,
				world: &mut world,
				entity,
				load_context: &mut load_context,
				meshes: &mut Vec::new(),
			})
			.unwrap();

		// They should've been called exactly once.
		assert!(unsafe { BASE_CALLED });
		assert!(unsafe { CLASS_CALLED });
	}
}
