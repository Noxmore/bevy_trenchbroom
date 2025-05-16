//! Builtin [`QuakeClass`] implementations.

flat! {
	#[cfg(feature = "bsp")]
	bsp;
}

use fgd::FgdType;
use qmap::{QuakeEntityError, QuakeEntityErrorResultExt};
use util::{angle_to_quat, angles_to_quat, mangle_to_quat};

use super::*;
use crate::*;

/// The prefix used by base classes provided by bevy_trenchbroom.
///
/// You should not use this prefix in your base classes to avoid conflicts.
pub const BUILTIN_BASE_CLASS_PREFIX: &str = "__";

pub struct BuiltinClassesPlugin;
impl Plugin for BuiltinClassesPlugin {
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
			.register_type_data::<PointLight, ReflectQuakeClass>()
			.register_type_data::<SpotLight, ReflectQuakeClass>()
			.register_type_data::<DirectionalLight, ReflectQuakeClass>()
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

#[cfg(feature = "client")]
impl QuakeClass for PointLight {
	const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
		ty: QuakeClassType::Base,
		name: "__point_light",
		description: None,
		base: &[Transform::ERASED_CLASS, Visibility::ERASED_CLASS],

		model: None,
		color: None,
		iconsprite: None,
		size: None,

		properties: &[
			QuakeClassProperty {
				ty: Color::PROPERTY_TYPE,
				name: "color",
				title: Some("Light Color"),
				description: None,
				default_value: Some(|| "\"1 1 1\"".s()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "intensity",
				title: Some("Light Intensity"),
				description: Some("Luminous power in lumens, representing the amount of light emitted by this source in all directions."),
				default_value: Some(|| PointLight::default().intensity.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "range",
				title: Some("Light Range"),
				description: Some(
					"Cut-off for the light's area-of-effect. Fragments outside this range will not be affected by this light at all, so it's important to tune this together with `intensity` to prevent hard lighting cut-offs.",
				),
				default_value: Some(|| PointLight::default().range.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "radius",
				title: Some("Light Radius"),
				description: Some("Simulates a light source coming from a spherical volume with the given radius."),
				default_value: Some(|| PointLight::default().radius.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: bool::PROPERTY_TYPE,
				name: "shadows_enabled",
				title: Some("Enable Shadows"),
				description: None,
				default_value: Some(|| PointLight::default().shadows_enabled.fgd_to_string()),
			},
			// Soft shadows can't be included because it's locked behind a feature
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "shadow_depth_bias",
				title: Some("Shadow Depth Bias"),
				description: Some(
					"A bias used when sampling shadow maps to avoid 'shadow-acne', or false shadow occlusions that happen as a result of shadow-map fragments not mapping 1:1 to screen-space fragments.",
				),
				default_value: Some(|| PointLight::DEFAULT_SHADOW_DEPTH_BIAS.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "shadow_normal_bias",
				title: Some("Shadow Normal Bias"),
				description: Some(
					"A bias applied along the direction of the fragment's surface normal. It is scaled to the shadow map's texel size so that it can be small close to the camera and gets larger further away.",
				),
				default_value: Some(|| PointLight::DEFAULT_SHADOW_NORMAL_BIAS.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "shadow_map_near_z",
				title: Some("Shadow Map Near Z"),
				description: Some("The distance from the light to near Z plane in the shadow map."),
				default_value: Some(|| PointLight::DEFAULT_SHADOW_MAP_NEAR_Z.fgd_to_string()),
			},
		],
	};

	fn class_spawn(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		let default = PointLight::default();

		#[allow(clippy::needless_update)]
		view.world.entity_mut(view.entity).insert(PointLight {
			color: view.src_entity.get("color").with_default(default.color)?,
			intensity: view.src_entity.get("intensity").with_default(default.intensity)?,
			range: view.src_entity.get("range").with_default(default.range)?,
			radius: view.src_entity.get("radius").with_default(default.radius)?,
			shadows_enabled: view.src_entity.get("shadows_enabled").with_default(default.shadows_enabled)?,
			shadow_depth_bias: view.src_entity.get("shadow_depth_bias").with_default(default.shadow_depth_bias)?,
			shadow_normal_bias: view.src_entity.get("shadow_normal_bias").with_default(default.shadow_normal_bias)?,
			shadow_map_near_z: view.src_entity.get("shadow_map_near_z").with_default(default.shadow_map_near_z)?,
			// For soft shadows
			..default
		});

		Ok(())
	}
}

#[cfg(feature = "client")]
impl QuakeClass for SpotLight {
	const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
		ty: QuakeClassType::Base,
		name: "__spot_light",
		description: None,
		base: &[Transform::ERASED_CLASS, Visibility::ERASED_CLASS],

		model: None,
		color: None,
		iconsprite: None,
		size: None,

		properties: &[
			QuakeClassProperty {
				ty: Color::PROPERTY_TYPE,
				name: "color",
				title: Some("Light Color"),
				description: None,
				default_value: Some(|| "\"1 1 1\"".s()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "intensity",
				title: Some("Light Intensity"),
				description: Some("Luminous power in lumens, representing the amount of light emitted by this source in all directions."),
				default_value: Some(|| SpotLight::default().intensity.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "range",
				title: Some("Light Range"),
				description: Some(
					"Range in meters that this light illuminates. Note that this value affects resolution of the shadow maps; generally, the higher you set it, the lower-resolution your shadow maps will be.",
				),
				default_value: Some(|| SpotLight::default().range.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "radius",
				title: Some("Light Radius"),
				description: Some("Simulates a light source coming from a spherical volume with the given radius."),
				default_value: Some(|| SpotLight::default().radius.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: bool::PROPERTY_TYPE,
				name: "shadows_enabled",
				title: Some("Enable Shadows"),
				description: None,
				default_value: Some(|| SpotLight::default().shadows_enabled.fgd_to_string()),
			},
			// Soft shadows can't be included because it's locked behind a feature
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "shadow_depth_bias",
				title: Some("Shadow Depth Bias"),
				description: Some(
					"A value that adjusts the tradeoff between self-shadowing artifacts and proximity of shadows to their casters. This value frequently must be tuned to the specific scene; this is normal and a well-known part of the shadow mapping workflow.",
				),
				default_value: Some(|| SpotLight::DEFAULT_SHADOW_DEPTH_BIAS.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "shadow_normal_bias",
				title: Some("Shadow Normal Bias"),
				description: Some(
					"A bias applied along the direction of the fragment's surface normal. It is scaled to the shadow map's texel size so that it can be small close to the camera and gets larger further away.",
				),
				default_value: Some(|| SpotLight::DEFAULT_SHADOW_NORMAL_BIAS.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "shadow_map_near_z",
				title: Some("Shadow Map Near Z"),
				description: Some("The distance from the light to near Z plane in the shadow map."),
				default_value: Some(|| SpotLight::DEFAULT_SHADOW_MAP_NEAR_Z.fgd_to_string()),
			},
			// We use degrees instead of radians here because it's easier to edit and visualize to an average person.
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "outer_angle",
				title: Some("Light Cone Outer Angle"),
				description: Some(
					"Angle defining the distance from the spot light direction to the outer limit of the light's cone of effect in degrees.",
				),
				default_value: Some(|| "\"45\"".s()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "inner_angle",
				title: Some("Light Cone Inner Angle"),
				description: Some(
					"Angle defining the distance from the spot light direction to the inner limit of the light's cone of effect in degrees.",
				),
				default_value: Some(|| "\"0\"".s()),
			},
		],
	};

	fn class_spawn(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		let default = SpotLight::default();

		#[allow(clippy::needless_update)]
		view.world.entity_mut(view.entity).insert(SpotLight {
			color: view.src_entity.get("color").with_default(default.color)?,
			intensity: view.src_entity.get("intensity").with_default(default.intensity)?,
			range: view.src_entity.get("range").with_default(default.range)?,
			radius: view.src_entity.get("radius").with_default(default.radius)?,
			shadows_enabled: view.src_entity.get("shadows_enabled").with_default(default.shadows_enabled)?,
			shadow_depth_bias: view.src_entity.get("shadow_depth_bias").with_default(default.shadow_depth_bias)?,
			shadow_normal_bias: view.src_entity.get("shadow_normal_bias").with_default(default.shadow_normal_bias)?,
			shadow_map_near_z: view.src_entity.get("shadow_map_near_z").with_default(default.shadow_map_near_z)?,
			outer_angle: view
				.src_entity
				.get("outer_angle")
				.map(f32::to_radians)
				.with_default(default.outer_angle)?,
			inner_angle: view
				.src_entity
				.get("inner_angle")
				.map(f32::to_radians)
				.with_default(default.inner_angle)?,
			// For soft shadows
			..default
		});

		Ok(())
	}
}

#[cfg(feature = "client")]
impl QuakeClass for DirectionalLight {
	const CLASS_INFO: QuakeClassInfo = QuakeClassInfo {
		ty: QuakeClassType::Base,
		name: "__directional_light",
		description: None,
		base: &[Transform::ERASED_CLASS, Visibility::ERASED_CLASS],

		model: None,
		color: None,
		iconsprite: None,
		size: None,

		properties: &[
			QuakeClassProperty {
				ty: Color::PROPERTY_TYPE,
				name: "color",
				title: Some("Light Color"),
				description: None,
				default_value: Some(|| "\"1 1 1\"".s()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "illuminance",
				title: Some("Light Illuminance"),
				description: Some(
					"Illuminance in lux (lumens per square meter), representing the amount of light projected onto surfaces by this light source.",
				),
				default_value: Some(|| light_consts::lux::AMBIENT_DAYLIGHT.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: bool::PROPERTY_TYPE,
				name: "shadows_enabled",
				title: Some("Enable Shadows"),
				description: None,
				default_value: Some(|| DirectionalLight::default().shadows_enabled.fgd_to_string()),
			},
			// Soft shadows can't be included because it's locked behind a feature
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "shadow_depth_bias",
				title: Some("Shadow Depth Bias"),
				description: Some(
					"A value that adjusts the tradeoff between self-shadowing artifacts and proximity of shadows to their casters. This value frequently must be tuned to the specific scene; this is normal and a well-known part of the shadow mapping workflow.",
				),
				default_value: Some(|| DirectionalLight::DEFAULT_SHADOW_DEPTH_BIAS.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: f32::PROPERTY_TYPE,
				name: "shadow_normal_bias",
				title: Some("Shadow Normal Bias"),
				description: Some(
					"A bias applied along the direction of the fragment's surface normal. It is scaled to the shadow map's texel size so that it is automatically adjusted to the orthographic projection.",
				),
				default_value: Some(|| DirectionalLight::DEFAULT_SHADOW_NORMAL_BIAS.fgd_to_string()),
			},
		],
	};

	fn class_spawn(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		let default = DirectionalLight::default();

		#[allow(clippy::needless_update)]
		view.world.entity_mut(view.entity).insert(DirectionalLight {
			color: view.src_entity.get("color").with_default(default.color)?,
			illuminance: view.src_entity.get("illuminance").with_default(default.illuminance)?,
			shadows_enabled: view.src_entity.get("shadows_enabled").with_default(default.shadows_enabled)?,
			shadow_depth_bias: view.src_entity.get("shadow_depth_bias").with_default(default.shadow_depth_bias)?,
			shadow_normal_bias: view.src_entity.get("shadow_normal_bias").with_default(default.shadow_normal_bias)?,
			// For soft shadows
			..default
		});

		Ok(())
	}
}

/// Quake entity IO - Able to target entities with the [`Targetable`] component.
///
/// TODO: this is currently just a skeleton struct, first-class entity IO hasn't been added yet.
#[derive(BaseClass, Component, Reflect, Debug, Clone, SmartDefault, Serialize, Deserialize)]
#[reflect(QuakeClass, Component, Default, Serialize, Deserialize)]
#[classname("__target")]
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
#[reflect(QuakeClass, Component, Default, Serialize, Deserialize)]
#[classname("__targetable")]
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
