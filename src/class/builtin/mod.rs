//! Builtin [`QuakeClass`] implementations.

// These modules aren't feature locked so that the plugins within can still be referenced,
// removing the need for the user to feature lock disabling them.
flat! {
	bsp;
	light;
	solid;
}

use fgd::FgdType;
use qmap::{QuakeEntityError, QuakeEntityErrorResultExt};
use util::{angle_to_quat, angles_to_quat, mangle_to_quat};

use super::*;

/// The prefix used by base classes provided by bevy_trenchbroom.
///
/// You should not use this prefix in your base classes to avoid conflicts.
pub const BUILTIN_BASE_CLASS_PREFIX: &str = "__";

pub struct BasicClassesPlugin;
impl Plugin for BasicClassesPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.register_type_data::<Transform, ReflectQuakeClass>()

			.register_type::<Target>()
			.register_type::<Targetable>()
		;

		#[cfg(feature = "client")]
		#[rustfmt::skip]
		app
			.register_type_data::<Visibility, ReflectQuakeClass>()
		;
	}
}

/// Reads the `origin` property, converting it to Bevy's coordinate space. Defaults to [`Vec3::ZERO`].
pub fn read_translation_from_entity(src_entity: &QuakeMapEntity, tb_config: &TrenchBroomConfig) -> Result<Vec3, QuakeEntityError> {
	Ok(tb_config.to_bevy_space(src_entity.get::<Vec3>("origin").with_default(Vec3::ZERO)?))
}

/// Tries to read `mangle`, `angles`, and `angle` in that order to produce a quaternion. Defaults to [`Quat::IDENTITY`].
pub fn read_rotation_from_entity(src_entity: &QuakeMapEntity) -> Result<Quat, QuakeEntityError> {
	Ok(match src_entity.get::<Vec3>("mangle") {
		// According to TrenchBroom docs https://trenchbroom.github.io/manual/latest/#editing-objects
		// “mangle” is interpreted as “yaw pitch roll” if the entity classnames begins with “light”, otherwise it’s a synonym for “angles”
		Ok(x) => {
			if src_entity.classname().map(|s| s.starts_with("light")) == Ok(true) {
				mangle_to_quat(x)
			} else {
				angles_to_quat(x)
			}
		}
		Err(QuakeEntityError::RequiredPropertyNotFound { .. }) => match src_entity.get::<Vec3>("angles") {
			Ok(x) => angles_to_quat(x),
			Err(QuakeEntityError::RequiredPropertyNotFound { .. }) => angle_to_quat(src_entity.get::<f32>("angle").with_default(0.)?),
			Err(err) => return Err(err),
		},
		Err(err) => return Err(err),
	})
}

impl QuakeClass for Transform {
	const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
		ty: QuakeClassType::Base,
		name: "__transform",
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

	fn class_spawn(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		view.world.entity_mut(view.entity).insert(Transform {
			translation: read_translation_from_entity(view.src_entity, view.config)?,
			rotation: read_rotation_from_entity(view.src_entity)?,
			scale: match view.src_entity.get::<f32>("scale") {
				Ok(scale) => Vec3::splat(scale),
				Err(QuakeEntityError::RequiredPropertyNotFound { .. }) => Vec3::ONE,
				Err(_) => view.src_entity.get::<Vec3>("scale")?.xzy(),
			},
		});
		Ok(())
	}
}

#[cfg(feature = "client")]
impl QuakeClass for Visibility {
	const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
		ty: QuakeClassType::Base,
		name: "__visibility",
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

	fn class_spawn(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		let visibility = match view.src_entity.properties.get("visibility").map(String::as_str) {
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

		view.world.entity_mut(view.entity).insert(visibility);

		Ok(())
	}
}

/// Quake entity IO - Able to target entities with the [`Targetable`] component.
///
/// TODO: this is currently just a skeleton struct, first-class entity IO hasn't been added yet.
#[base_class(classname("__target"))]
#[derive(Debug, Clone, SmartDefault, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct Target {
	/// If [`Some`], when this entity's IO fires, it will activate all entities with its [`Targetable::targetname`] set to this, with whatever input that functionality that entity has set up.
	pub target: Option<String>,
	/// If [`Some`], when this entity's IO fires, it will kill all entities with its [`Targetable::targetname`] set to this.
	pub killtarget: Option<String>,
}

/// Quake entity IO - Able to be targeted from a [`Target`] component.
///
/// TODO: this is currently just a skeleton struct, first-class entity IO hasn't been added yet.
#[base_class(classname("__targetable"))]
#[derive(Debug, Clone, SmartDefault, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct Targetable {
	/// The name for entities with [`Target`] components to point to.
	pub targetname: Option<String>,
}

#[cfg(test)]
mod tests {
	#[allow(unused)]
	use super::*;

	#[cfg(feature = "bsp")]
	#[test]
	fn builtin_base_class_prefix() {
		let mut app = App::new();

		app.init_resource::<AppTypeRegistry>()
			.register_type::<Transform>()
			.register_type::<PointLight>()
			.register_type::<SpotLight>()
			.register_type::<DirectionalLight>()
			.register_type::<Visibility>()
			.add_plugins((BuiltinClassesPlugin, BspClassesPlugin));

		for (_, ReflectQuakeClass { erased_class: class, .. }) in
			app.world().resource::<AppTypeRegistry>().read().iter_with_data::<ReflectQuakeClass>()
		{
			if class.info.ty.is_base() {
				assert!(
					class.info.name.starts_with(BUILTIN_BASE_CLASS_PREFIX),
					"class {:?} does not start with prefix {BUILTIN_BASE_CLASS_PREFIX:?}",
					class.info.name
				);
			}
		}
	}
}
