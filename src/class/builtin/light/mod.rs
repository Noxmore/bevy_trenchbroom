use super::*;

flat! {
	#[cfg(feature = "bsp")]
	bsp;
	basic_impls;
}

pub const QUAKE_LIGHT_TO_LUM_MULTIPLIER: f32 = 1000.;
/// Quake light (such as the `light` property used in light entities) conversion to lumens.
///
/// NOTE: This is only a rough estimation, based on what i've personally found looks right.
#[inline]
pub fn quake_light_to_lum(light: f32) -> f32 {
	light * QUAKE_LIGHT_TO_LUM_MULTIPLIER
}

pub const QUAKE_LIGHT_TO_LUX_DIVISOR: f32 = 50_000.;
/// Quake light (such as the `light` property used in light entities) conversion to lux (lumens per square meter).
///
/// NOTE: This is only a rough estimation, based on what i've personally found looks right.
#[inline]
pub fn quake_light_to_lux(light: f32) -> f32 {
	light / QUAKE_LIGHT_TO_LUX_DIVISOR
}

/// Commonly used workflows with Quake maps and BSPs. Used with [`LightingClassesPlugin`] to set up a lighting workflow with minimal boilerplate.
///
/// If the `bsp` feature is enabled, the default value is [`MapDynamicBspBaked`](LightingWorkflow::MapDynamicBspBaked), otherwise [`DynamicOnly`](LightingWorkflow::DynamicOnly) if only `.map`s are supported.
#[derive(Debug, Clone, Copy, Default)]
pub enum LightingWorkflow {
	/// Only use dynamic lighting, no BSP baked lighting.
	/// Adds point, spot, and directional lights as separate entities, with Bevy-only properties.
	#[cfg_attr(not(feature = "bsp"), default)]
	DynamicOnly,
	/// Only use BSP baked lighting, no Bevy dynamic lighting.
	/// Adds a single `light` entity, which by default represents a point light, and by setting properties can become a spot, or directional light. No Bevy light settings are included.
	#[cfg(feature = "bsp")]
	BakedOnly,
	/// Use BSP baked lighting only if loading a BSP, else use dynamic lighting.
	/// Adds a single `light` entity, containing all the same settings as [`BakedOnly`](LightingWorkflow::BakedOnly), along with Bevy-specific overrides under the `dynamic_` prefix.
	/// For non-set overrides, the Quake-specific properties will be converted into Bevy lights when spawning dynamic lighting.
	#[cfg_attr(feature = "bsp", default)]
	#[cfg(feature = "bsp")]
	MapDynamicBspBaked,
	/// Uses the same entity setup as [`MapDynamicBspBaked`](LightingWorkflow::MapDynamicBspBaked), but doesn't make a distinction based on what type of map is being loaded.
	/// Lights will always spawn real-time dynamic lights over the baked ones unless their `dynamic_enabled` property is set to `false`.
	#[cfg(feature = "bsp")]
	DynamicAndBakedCombined,
	/// Use both baked and dynamic lighting, with baked lights being a separate `light` entity from real-time dynamic lights which are under the `dynamiclight_` prefix.
	#[cfg(feature = "bsp")]
	DynamicAndBakedSeparate,
	/// Don't register any ready-to-go light types, you're on your own.
	Custom,
}

#[derive(Default)]
pub struct LightingClassesPlugin(pub LightingWorkflow);
impl Plugin for LightingClassesPlugin {
	fn build(&self, #[allow(unused)] app: &mut App) {
		#[cfg(feature = "client")]
		#[rustfmt::skip]
		app
			.register_type_data::<PointLight, ReflectQuakeClass>()
			.register_type_data::<SpotLight, ReflectQuakeClass>()
			.register_type_data::<DirectionalLight, ReflectQuakeClass>()
		;
		
		match self.0 {
			LightingWorkflow::DynamicOnly => {
				app.register_type::<DynamicOnlyPointLight>().register_type::<DynamicOnlySpotLight>().register_type::<DynamicOnlyDirectionalLight>();
			},
			#[cfg(feature = "bsp")]
			LightingWorkflow::BakedOnly => {
				app.register_type::<BakedOnlyLight>();
			},
			#[cfg(feature = "bsp")]
			LightingWorkflow::MapDynamicBspBaked => {
				app.register_type::<MapDynamicBspBakedLight>();
			},
			#[cfg(feature = "bsp")]
			LightingWorkflow::DynamicAndBakedCombined => {
				app.register_type::<CombinedLight>();
			},
			#[cfg(feature = "bsp")]
			LightingWorkflow::DynamicAndBakedSeparate => {
				app.register_type::<BakedOnlyLight>().register_type::<DynamicPointLight>().register_type::<DynamicSpotLight>().register_type::<DynamicDirectionalLight>();
			},
			LightingWorkflow::Custom => {},
		}
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
		decal: false,

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
				description: Some("Simulates a light source coming from a spherical volume with the given radius. This affects the size of specular highlights created by this light."),
				default_value: Some(|| PointLight::default().radius.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: bool::PROPERTY_TYPE,
				name: "shadows_enabled",
				title: Some("Enable Shadows"),
				description: None,
				default_value: Some(|| PointLight::default().shadows_enabled.fgd_to_string()),
			},
			QuakeClassProperty {
				ty: bool::PROPERTY_TYPE,
				name: "affects_lightmapped_mesh_diffuse",
				title: Some("Affects Lightmapped Mesh Diffuse"),
				description: Some("Whether this light contributes diffuse lighting to meshes with lightmaps.\nNote that the specular portion of the light is always considered, because Bevy currently has no means to bake specular light."),
				default_value: Some(|| PointLight::default().affects_lightmapped_mesh_diffuse.fgd_to_string()),
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
			affects_lightmapped_mesh_diffuse: view.src_entity.get("affects_lightmapped_mesh_diffuse").with_default(default.affects_lightmapped_mesh_diffuse)?,
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
		decal: false,

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
			QuakeClassProperty {
				ty: bool::PROPERTY_TYPE,
				name: "affects_lightmapped_mesh_diffuse",
				title: Some("Affects Lightmapped Mesh Diffuse"),
				description: Some("Whether this light contributes diffuse lighting to meshes with lightmaps.\nNote that the specular portion of the light is always considered, because Bevy currently has no means to bake specular light."),
				default_value: Some(|| SpotLight::default().affects_lightmapped_mesh_diffuse.fgd_to_string()),
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
			affects_lightmapped_mesh_diffuse: view.src_entity.get("affects_lightmapped_mesh_diffuse").with_default(default.affects_lightmapped_mesh_diffuse)?,
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
		decal: false,

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
			QuakeClassProperty {
				ty: bool::PROPERTY_TYPE,
				name: "affects_lightmapped_mesh_diffuse",
				title: Some("Affects Lightmapped Mesh Diffuse"),
				description: Some("Whether this light contributes diffuse lighting to meshes with lightmaps.\nNote that the specular portion of the light is always considered, because Bevy currently has no means to bake specular light."),
				default_value: Some(|| DirectionalLight::default().affects_lightmapped_mesh_diffuse.fgd_to_string()),
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
			affects_lightmapped_mesh_diffuse: view.src_entity.get("affects_lightmapped_mesh_diffuse").with_default(default.affects_lightmapped_mesh_diffuse)?,
			shadow_depth_bias: view.src_entity.get("shadow_depth_bias").with_default(default.shadow_depth_bias)?,
			shadow_normal_bias: view.src_entity.get("shadow_normal_bias").with_default(default.shadow_normal_bias)?,
			// For soft shadows
			..default
		});

		Ok(())
	}
}

