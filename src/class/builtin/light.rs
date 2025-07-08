use crate::fgd::{IntBool, IntBoolOverride, Srgb};

use super::*;

#[derive(Debug, Clone, Copy, Default)]
pub enum LightingWorkflow {
	#[cfg_attr(not(feature = "bsp"), default)]
	DynamicOnly,
	#[cfg(feature = "bsp")]
	BakedOnly,
	#[cfg_attr(feature = "bsp", default)]
	#[cfg(feature = "bsp")]
	MapDynamicBspBaked,
	#[cfg(feature = "bsp")]
	DynamicAndBakedCombined,
	#[cfg(feature = "bsp")]
	DynamicAndBakedSeparate,
	Custom,
}

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

		#[cfg(feature = "client")]
		match self.0 {
			// TODO:
			LightingWorkflow::DynamicOnly => {},
			LightingWorkflow::BakedOnly => {},
			LightingWorkflow::MapDynamicBspBaked => {},
			LightingWorkflow::DynamicAndBakedCombined => {},
			LightingWorkflow::DynamicAndBakedSeparate => {},
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

/// Contains properties used by the `ericw-tools` compiler for any entity with a classname starting with the first five letters "light". E.g. "light", "light_spot", "light_flame_small_yellow", etc.
/// 
/// This is a combined class instead of split into point, spot, and directional lights because `ericw-tools` makes no distinction based on entity type. You have to specify per-entity what kind of light it is.
#[cfg(feature = "bsp")]
#[base_class(
	classname("__bsp_combined_light"),
)]
#[derive(Debug, Clone, SmartDefault, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct BspLight {
	/// Set the light intensity. Negative values are also allowed and will cause the entity to subtract light cast by other entities. Default 300.
	#[default(300.)]
	pub light: f32,

	/// Scale the fade distance of the light by the value specified. Values of n > 1 make the light fade more quickly with distance, and values < 1 make the light fade more slowly (and thus reach further). Default 1.
	#[default(1.)]
	pub wait: f32,

	/// The attenuation formula for the light.
	pub delay: BspLightAttenuation,

	/// Sets the distance at which the light drops to 0, in map units.
	///
	/// In this mode, "wait" is ignored and "light" only controls the brightness at the center of the light, and no longer affects the falloff distance.
	///
	/// Only supported on linear attenuation (delay 0) lights currently.
	pub _falloff: Option<f32>,

	/// Specify red(r), green(g) and blue(b) components for the colour of the light. RGB component values are between 0 and 255 (between 0 and 1 is also accepted). Default is white light ("255 255 255").
	#[default(Srgb::WHITE_255)]
	pub _color: Srgb,

	/// Turns the light into a switchable light, toggled by another entity targeting it’s name.
	pub targetname: Option<String>,

	/// Set the animated light style. Default 0.
	#[default(LightmapStyle::NORMAL)]
	pub style: LightmapStyle,

	/// Sets a scaling factor for how much influence the angle of incidence of light on a surface has on the brightness of the surface
	/// Value must be between 0.0 and 1.0. Smaller values mean less attenuation, with zero meaning that angle of incidence has no effect at all on the brightness.
	/// Default 0.5.
	#[default(0.5)]
	pub _anglescale: f32,

	/// Override the global "_dirtscale" setting to change how this light is affected by dirtmapping (ambient occlusion). See descriptions of this key in the worldspawn section.
	pub _dirtscale: Option<f32>,

	/// Override the global "_dirtgain" setting to change how this light is affected by dirtmapping (ambient occlusion). See descriptions of this key in the worldspawn section.
	pub _dirtgain: Option<f32>,

	/// Overrides the worldspawn setting of "_dirt" for this particular light.
	/// -1 to disable dirtmapping (ambient occlusion) for this light, making it illuminate the dirtmapping shadows.
	/// 1 to enable ambient occlusion for this light. Default is to defer to the worldspawn setting.
	pub _dirt: IntBoolOverride,

	/// Split up the light into a sphere of randomly positioned lights within the radius denoted by this value (in world units).
	/// Useful to give shadows a wider penumbra. "_samples" specifies the number of lights in the sphere.
	/// The "light" value is automatically scaled down for most lighting formulas (except linear and non-additive minlight) to attempt to keep the brightness equal.
	/// Default is 0, do not split up lights.
	pub _deviance: f32,

	/// Number of lights to use for "_deviance". Default 16 (only used if "_deviance" is set).
	#[default(16)]
	pub _samples: u32,

	/// Scales the amount of light that is contributed by bounces. Default is 1.0, 0.0 disables bounce lighting for this light.
	#[default(1.)]
	pub _bouncescale: f32,

	/// Set to 1 to make the light compiler ignore this entity (prevents it from casting any light). e.g. could be useful with rtlights.
	pub _nostaticlight: IntBool,

	/// Calculate lighting with and without brush models with a “targetname” equal to this value, and stores the resulting switchable shadow data in a light style which is stored in this light entity’s “style” key.
	///
	/// You should give this light a targetname and typically set “spawnflags” “1” (start off).
	///
	/// Implies `_nostaticlight` (this entity itself does not cast any light).
	pub _switchableshadow_target: Option<String>,

	/// Turns the light into a spotlight (or sun light if `_sun` if 1), with the direction of light being towards another entity with it’s "targetname" key set to this value.
	///
	/// NOTE: Docs may imply that sun lights have to target `info_null` entities? I haven't tested it though.
	pub target: Option<String>,

	/// Turns the light into a spotlight and specifies the direction of light using yaw, pitch and roll in degrees.
	/// Yaw specifies the angle around the Z-axis from 0 to 359 degrees and pitch specifies the angle from 90 (straight up) to -90 (straight down).
	/// Roll has no effect, so use any value (e.g. 0). Often easier than the "target" method.
	pub mangle: Vec3,

	/// Specifies the angle in degrees for a spotlight cone. Default 40.
	#[default(40.)]
	pub angle: f32,

	/// Specifies the angle in degrees for an inner spotlight cone (must be less than the "angle" cone. Creates a softer transition between the full brightness of the inner cone to the edge of the outer cone. Default 0 (disabled).
	pub _softangle: f32,

	/// Makes surfaces with the given texture name emit light, by using this light as a template which is copied across those surfaces.
	/// Lights are spaced about 128 units (though possibly closer due to bsp splitting) apart and positioned 2 units above the surfaces.
	pub _surface: Option<String>,

	/// Controls the offset lights are placed above surfaces for "_surface" (world units). Default 2.
	#[default(2.)]
	pub _surface_offset: f32,

	/// For a surface light template (i.e. a light with "_surface" set), setting this to "1" makes each instance into a spotlight,
	/// with the direction of light pointing along the surface normal. In other words, it automatically sets "mangle" on each of the generated lights.
	pub _surface_spotlight: IntBool,

	/// Whether to use Q1-style surface subdivision (0) or Q2-style surface radiosity (1) on this light specifically.
	///
	/// Use in conjunction with `_surface`.
	///
	/// The default can be changed for all surface lights in a map with worldspawn key `_surflight_radiosity`.
	pub _surface_radiosity: Option<IntBool>,

	/// Integer, default 0.
	///
	/// For use with `_surface` lights.
	///
	/// Can be set to a nonzero value to restrict this surface light template to only emit from brushes with a matching `_surflight_group` value.
	pub _surflight_group: u32,

	/// Specifies that a light should project this texture. The texture must be used in the map somewhere.
	pub _project_texture: Option<String>,

	/// Specifies the yaw/pitch/roll angles for a texture projection (overriding mangle).
	pub _project_mangle: Option<Vec3>,

	/// Specifies the fov angle for a texture projection. Default 90.
	#[default(90.)]
	pub _project_fov: f32,

	/// Set to 1 to make this entity a sun, as an alternative to using the sunlight worldspawn keys.
	/// If the light targets an info_null entity, the direction towards that entity sets sun direction.
	/// The light itself is disabled, so it can be placed anywhere in the map.
	///
	///
	/// The following light properties correspond to these sunlight settings:
	/// - light => _sunlight
	/// - mangle => _sunlight_mangle
	/// - deviance => _sunlight_penumbra
	/// - _color => _sunlight_color
	/// - _dirt => _sunlight_dirt
	/// - _anglescale => _anglescale
	pub _sun: IntBool,

	/// This sunlight is only emitted from faces with this texture name. Default is to be emitted from all sky textures.
	pub _suntexture: Option<String>,

	/// Set to 1 to make this entity control the upper dome lighting emitted from sky faces, as an alternative to the worldspawn key `_sunlight2`.
	/// The light entity itself is disabled, so it can be placed anywhere in the map.
	pub _sunlight2: IntBool,

	/// Same as `_sunlight2`, but makes this sky light come from the lower hemisphere.
	pub _sunlight3: IntBool,

	/// Mask of lighting channels that the light casts on.
	///
	/// In order for this light to cast light on a bmodel, there needs to be a least 1 bit in common between `_light_channel_mask` and the receiving bmodel’s `_object_channel_mask` (i.e. the bitwise AND must be nonzero).
	///
	/// Default 1.
	#[default(1)]
	pub _light_channel_mask: u32,

	/// This is the mask of lighting channels that will block this entity’s light rays. If the the bitwise AND of this and another bmodel’s `_object_channel_mask` is nonzero, the light ray is stopped.
	///
	/// This is an advanced option, for making bmodels only cast shadows for specific lights (but not others).
	///
	/// Defaults to `_light_channel_mask`
	pub _shadow_channel_mask: Option<u32>,
}

/// How light fades over distance. Used in the `delay` property of light entities.
#[cfg(feature = "bsp")]
#[derive(FgdType, Reflect, Debug, Clone, Default, Serialize, Deserialize)]
#[number_key]
pub enum BspLightAttenuation {
	/// Linear attenuation (default)
	#[default]
	Linear = 0,
	/// 1/x attenuation
	Reciprocal = 1,
	/// 1/(x^2) attenuation
	ReciprocalSquare = 2,
	/// No attenuation (same brightness at any distance)
	None = 3,
	/// No attenuation, and like minlight
	/// it won’t raise the lighting above it’s light value.
	/// Unlike minlight, it will only affect surfaces within
	/// line of sight of the entity.
	LocalMinLight = 4,
	/// 1/(x^2) attenuation, but slightly more attenuated and
	/// without the extra bright effect that [`ReciprocalSquare`](BspLightAttenuation::ReciprocalSquare) has
	/// near the source.
	ReciprocalSquareTweaked = 5,
}
