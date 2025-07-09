//! Builtin [`QuakeClass`] implementations.

// These modules aren't feature locked so that the plugins within can still be referenced,
// removing the need for the user to feature lock disabling them.
flat! {
	base;
	light;
	point;
	solid;
}

use bevy::app::plugin_group;
use fgd::FgdType;
use qmap::{QuakeEntityError, QuakeEntityErrorResultExt};
use util::{angle_to_quat, angles_to_quat, mangle_to_quat};

use super::*;

plugin_group! {
	#[derive(Debug)]
	pub struct BasicClassesPlugins {
		:BaseClassesPlugin,
		:LightingClassesPlugin,
		:PointClassesPlugin,
		:SolidClassesPlugin,
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
			.add_plugins(BasicClassesPlugins);

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
