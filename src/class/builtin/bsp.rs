//! A collection of useful base classes when working with a BSP workflow.

use fgd::{IntBool, IntBoolOverride, Srgb};

use crate::*;

pub struct BspClassesPlugin;
impl Plugin for BspClassesPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.register_type::<BspSolidEntity>()
			.register_type::<BspWorldspawn>()
			.register_type::<BspLight>()
		;
	}
}

/// Contains properties used by the `ericw-tools` compiler for any entity with a brush model.
#[base_class(
	classname("__bsp_solid_entity"),
)]
#[derive(Debug, Clone, SmartDefault, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct BspSolidEntity {
	/// Generates an `LMSHIFT` BSPX lump for use by a light util. Note that both scaled and unscaled lighting will normally be used.
	pub _lmscale: Option<u32>,

	/// Set to 1 to save mirrored inside faces for brush models, so when the player view is inside the model, they will still see the faces. (e.g. for func_water, or func_illusionary)
	pub _mirrorinside: IntBool,

	/// Customize the brush order, which affects which brush “wins” in the CSG phase when there are multiple overlapping brushes,
	/// since most .map editors don’t directly expose the brush order.
	///
	/// Defaults to 0, brushes with higher values (equivalent to appearing later in the .map file) will clip away lower valued brushes.
	pub _chop_order: u32,

	/// Bitmap (“Flags” type in FGD) that selects for which hulls collision data will be generated.
	/// eg. a decimal value of 11 (0b1011) would generate hull 0, hull 1, and hull 3.
	/// Faces are computed using data from hull 0, not generating this hull will prevent a brush model from being rendered,
	/// acting as a CLIP brush only active for the specified hulls.
	///
	/// Defaults to 0 which will generate clipnodes for all hulls.
	pub _hulls: u32,

	/// `worldspawn`: Set a global minimum light level of this value across the whole map.
	/// This is an easy way to eliminate completely dark areas of the level, however you may lose some contrast as a result, so use with care. Default 0.
	///
	/// `model entity`: Set the minimum light level for any surface of the brush model. Default 0.
	pub _minlight: f32,

	/// Whether minlight should have a mottled pattern. Defaults to 0.
	pub _minlight_mottle: IntBool,

	/// Specify red(r), green(g) and blue(b) components for the colour of the minlight. RGB component values are between 0 and 255 (between 0 and 1 is also accepted).
	/// Default is white light ("255 255 255").
	#[default(Srgb::WHITE_255)]
	pub _minlight_color: Srgb,

	/// Faces with the given texture are excluded from receiving minlight on this brush model.
	pub _minlight_exclude: Option<String>,

	/// Faces with the given texture are excluded from receiving minlight on this brush model.
	pub _minlight_exclude2: Option<String>,

	/// Faces with the given texture are excluded from receiving minlight on this brush model.
	pub _minlight_exclude3: Option<String>,

	/// If set to 1, this model will cast shadows on other models and itself (i.e. "_shadow" implies "_shadowself").
	/// Note that this doesn’t magically give Quake dynamic lighting powers, so the shadows will not move if the model moves.
	/// Set to -1 on func_detail/func_group to prevent them from casting shadows. Default 0.
	pub _shadow: IntBool, // This is IntBool because func_detail and func_group get compiled into worldspawn, so will be removed

	/// If set to 1, this model will cast shadows on itself if one part of the model blocks the light from another model surface.
	/// This can be a better compromise for moving models than full shadowing. Default 0.
	pub _shadowself: IntBool,

	/// If set to 1, this model will cast shadows on the world only (not other brush models).
	pub _shadowworldonly: IntBool,

	/// If set to 1, this model casts a shadow that can be switched on/off using QuakeC.
	/// To make this work, a lightstyle is automatically assigned and stored in a key called "switchshadstyle", which the QuakeC will need to read and call the "lightstyle()" builtin with "a" or "m" to switch the shadow on or off.
	/// Entities sharing the same targetname, and with "_switchableshadow" set to 1, will share the same lightstyle.
	pub _switchableshadow: IntBool,

	/// `worldspawn`: 1 enables dirtmapping (ambient occlusion) on all lights, borrowed from q3map2. This adds shadows to corners and crevices.
	/// You can override the global setting for specific lights with the "_dirt" light entity key or "_sunlight_dirt", "_sunlight2_dirt", and "_minlight_dirt" worldspawn keys.
	/// Default is no dirtmapping (-1).
	///
	/// `model entity`: For brush models, -1 prevents dirtmapping on the brush model. Useful it the brush model touches or sticks into the world, and you want to those ares from turning black. Default 0.
	pub _dirt: IntBoolOverride,

	/// 1 enables phong shading on this model with a default _phong_angle of 89 (softens columns etc).
	pub _phong: IntBool,

	/// Enables phong shading on faces of this model with a custom angle. Adjacent faces with normals this many degrees apart (or less) will be smoothed.
	/// Consider setting "_anglescale" to "1" on lights or worldspawn to make the effect of phong shading more visible.
	/// Use the "-phongdebug" command-line flag to save the interpolated normals to the lightmap for previewing (use "r_lightmap 1" or "gl_lightmaps 1" in your engine to preview.)
	#[default(89.)]
	pub _phong_angle: f32,

	/// Optional key for setting a different angle threshold for concave joints.
	/// A pair of faces will either use "_phong_angle" or "_phong_angle_concave" as the smoothing threshold, depending on whether the joint between the faces is concave or not.
	/// "_phong_angle(_concave)" is the maximum angle (in degrees) between the face normals that will still cause the pair of faces to be smoothed.
	/// The minimum setting for "_phong_angle_concave" is 1, this should make all concave joints non-smoothed (unless they’re less than 1 degree apart, almost a flat plane.)
	/// If it’s 0 or unset, the same value as "_phong_angle" is used.
	pub _phong_angle_concave: Option<f32>,

	/// Integer specifying a “smoothing group ID” for phong shading. Default 0, faces with a _phong_group will only smooth with faces with a matching _phong_group.
	///
	/// Equivalent to the Q2 .map format’s “value” field.
	pub _phong_group: u32,

	/// 1 makes a model receive minlight only, ignoring all lights / sunlight. Could be useful on rotators / trains.
	pub _lightignore: IntBool,

	/// Set to 1 to enable receiving light from either side.
	///
	/// Default is 0 execept on liquids (Q1 *, Q2 contents LAVA/SLIME/WATER), where it defaults to 1.
	pub _light_twosided: Option<IntBool>,

	/// Float, range 0-1. Allows customizing the opacity of this face when it’s acting as “stained glass”.
	///
	/// `ericw-tools todo` Document default, and which conditions cause a face to be “stained glass”
	pub _light_alpha: Option<f32>,

	/// Overrides the worldspawn/command line option qbsp -litwater for these specific brushes.
	pub _litwater: Option<IntBool>,

	/// Overrides the worldspawn key _surflight_atten for these brushes.
	pub _surflight_atten: Option<f32>,

	/// Integer, 0 or 1.
	///
	/// If 1, rescales any surface light emitted by these brushes to emit 50% light at 90 degrees from the surface normal. Otherwise, use a more natural angle falloff of 0% at 90 degrees.
	///
	/// Default is 0 on sky faces, otherwise 1.
	pub _surflight_rescale: Option<IntBool>,

	/// Customize the emissive color of a surface light.
	///
	/// Default is to use the average texture color.
	pub _surflight_color: Option<Srgb>,

	/// Override the surface light lightstyle number for light emitted from these brushes.
	pub _surflight_style: Option<LightmapStyle>,

	/// Override the surface light targetname for light emitted from these brushes.
	pub _surflight_targetname: Option<String>,

	/// Overrides the worldspawn setting `_surflight_minlight_scale`.
	pub _surflight_minlight_scale: Option<f32>,

	/// Set to -1 to prevent this model from bouncing light (i.e. prevents its brushes from emitting bounced light they receive from elsewhere.)
	/// Only has an effect if “_bounce” is enabled in worldspawn.
	pub _bounce: IntBoolOverride,

	/// “Autominlight” is a feature for automatically choosing a suitable minlight color for a solid entity (e.g. a func_door),
	/// by averaging incoming light at the center of the model bounding box.
	///
	/// Default behaviour is to apply autominlight on occluded luxels only
	/// (e.g., for a door that opens vertically upwards, it would apply to the bottom face of the door, which is initially pressed against the ground).
	///
	/// A value of “-1” disables the feature (occluded luxels will be solid black), and “1” enables it as a minlight color even on non-occluded luxels.
	pub _autominlight: IntBoolOverride,

	/// For autominlight, instead of using the center of the model bounds as the sample point,
	/// searches for an entity with its “targetname” key set to “name”, and use that entity’s origin (typically you’d use an “info_null” for this).
	pub _autominlight_target: Option<String>,

	/// When -world_units_per_luxel is in use, customizes the lightmap scale on this entity.
	pub _world_units_per_luxel: Option<f32>,

	/// Integer. Default 0.
	///
	/// Can be set to a nonzero value to make these brushes emit as surface lights only from a light template with a matching _surflight_group value.
	pub _surflight_group: u32,

	/// Saturation control as a postprocessing step on these specific faces’ lightmaps.
	///
	/// Default 1.0, 0.0 is fully desaturated to greyscale.
	#[default(1.)]
	pub _lightcolorscale: f32,

	/// Mask of lighting channels that this bmodel receives light on, blocks light on, and tests for AO on.
	///
	/// Default 1.
	///
	/// NOTE: Changing this from 1 will disable bouncing light off of this bmodel.
	///
	/// NOTE: Changing this from 1 implicitly enables _shadow.
	///
	/// NOTE: Changing to 2, for example, will cause the bmodel to initially be solid black. You’ll need to add minlight or lights with _light_channel_mask 2.
	#[default(1)]
	pub _object_channel_mask: u32,
}

/// Contains properties used by the `ericw-tools` compiler for the `worldspawn` entity.
#[base_class(
	base(BspSolidEntity),
	classname("__bsp_worldspawn"),
)]
#[derive(Debug, Clone, SmartDefault, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct BspWorldspawn {
	/// (Not documented, but hopefully self-explanatory.)
	pub _maxlight: Option<f32>,

	/// Scales the fade distance of all lights by a factor of n. If n > 1 lights fade more quickly with distance and if n < 1, lights fade more slowly with distance and light reaches further.
	#[default(1.)]
	pub _dist: f32,

	/// Scales the brightness range of all lights without affecting their fade discance. Values of n > 0.5 makes lights brighter and n < 0.5 makes lights less bright. The same effect can be achieved on individual lights by adjusting both the "light" and "wait" attributes.
	#[default(0.5)]
	pub _range: f32,

	/// Set the brightness of the sunlight coming from an unseen sun in the sky. Sky brushes (or more accurately bsp leafs with sky contents) will emit sunlight at an angle specified by the "_sun_mangle" key. Default 0.
	pub _sunlight: f32,

	/// Set the scaling of sunlight brightness due to the angle of incidence with a surface (more detailed explanation in the "_anglescale" light entity key).
	#[default(0.5)]
	pub _anglescale: f32,

	/// Specifies the direction of sunlight using yaw, pitch and roll in degrees. Yaw specifies the angle around the Z-axis from 0 to 359 degrees and pitch specifies the angle from 90 (shining straight up) to -90 (shining straight down from above). Roll has no effect, so use any value (e.g. 0). Default is straight down ("0 -90 0").
	#[default(vec3(0., -90., 0.))]
	pub _sunlight_mangle: Vec3,

	/// (Not documented.)
	pub _sun2: f32,

	/// (Not documented, default is an educated guess.)
	#[default(Srgb::WHITE)]
	pub _sun2_color: Srgb,

	/// (Not documented, default is an educated guess.)
	#[default(vec3(0., -90., 0.))]
	pub _sun2_mangle: Vec3,

	/// Specifies the penumbra width, in degrees, of sunlight. Useful values are 3-4 for a gentle soft edge, or 10-20+ for more diffuse sunlight. Default is 0.
	pub _sunlight_penumbra: f32,

	/// Specify red(r), green(g) and blue(b) components for the color of the sunlight. RGB component values are between 0 and 255 (between 0 and 1 is also accepted). Default is white light ("255 255 255").
	#[default(Srgb::WHITE_255)]
	pub _sunlight_color: Srgb,

	/// Set the brightness of a dome of lights arranged around the upper hemisphere. (i.e. ambient light, coming from above the horizon). Default 0.
	pub _sunlight2: f32,

	/// Specifies the colour of _sunlight2, same format as "_sunlight_color". Default is white light ("255 255 255").
	#[default(Srgb::WHITE_255)]
	pub _sunlight2_color: Srgb,

	/// Same as "_sunlight2", but for the bottom hemisphere (i.e. ambient light, coming from below the horizon). Combine "_sunlight2" and "_sunlight3" to have light coming equally from all directions, e.g. for levels floating in the clouds. Default 0.
	pub _sunlight3: f32,

	/// Specifies the colour of "_sunlight3". Default is white light ("255 255 255").
	#[default(Srgb::WHITE_255)]
	pub _sunlight3_color: Srgb,

	/// 1 enables dirtmapping (ambient occlusion) on sunlight, -1 to disable (making it illuminate the dirtmapping shadows). Default is to use the value of "_dirt".
	pub _sunlight_dirt: IntBoolOverride,

	/// 1 enables dirtmapping (ambient occlusion) on sunlight2/3, -1 to disable. Default is to use the value of "_dirt".
	pub _sunlight2_dirt: IntBoolOverride,

	/// 1 enables dirtmapping (ambient occlusion) on minlight, -1 to disable. Default is to use the value of "_dirt".
	pub _minlight_dirt: IntBoolOverride,

	/// Choose between ordered (0, default) and randomized (1) dirtmapping.
	pub _dirtmode: DirtMode,

	/// Maximum depth of occlusion checking for dirtmapping, default 128.
	#[default(128.)]
	pub _dirtdepth: f32,

	/// Scale factor used in dirt calculations, default 1. Lower values (e.g. 0.5) make the dirt fainter, 2.0 would create much darker shadows.
	#[default(1.)]
	pub _dirtscale: f32,

	/// Exponent used in dirt calculation, default 1. Lower values (e.g. 0.5) make the shadows darker and stretch further away from corners.
	#[default(1.)]
	pub _dirtgain: f32,

	/// Cone angle in degrees for occlusion testing, default 88. Allowed range 1-90. Lower values can avoid unwanted dirt on arches, pipe interiors, etc.
	#[default(88.)]
	pub _dirtangle: f32,

	/// Adjust brightness of final lightmap. Default 1, >1 is brighter, <1 is darker.
	#[default(1.)]
	pub _gamma: f32,

	/// Forces all surfaces+submodels to use this specific lightmap scale. Removes "LMSHIFT" field.
	pub _lightmap_scale: Option<f32>,

	/// 1 enables bounce lighting, disabled by default.
	pub _bounce: IntBool,

	/// Scales brightness of bounce lighting, default 1.
	#[default(1.)]
	pub _bouncescale: f32,

	/// Weight for bounce lighting to use texture colors from the map: 0=ignore map textures (default), 1=multiply bounce light color by texture color.
	pub _bouncecolorscale: f32,

	/// (Not documented.)
	pub _bouncelightsubdivision: Option<f32>,

	/// Scales the surface light emission from Q2 surface lights (excluding sky faces) by this amount.
	#[default(1.)]
	pub _surflightscale: f32,

	/// (Not documented.)
	#[default(1.)]
	pub _surflight_atten: f32,

	/// Scales the surface light emission from Q2 sky faces by this amount.
	#[default(1.)]
	pub _surflightskyscale: f32,

	/// (Not documented.)
	pub _surflightsubdivision: Option<f32>,

	/// (Not documented.)
	pub _choplight: Option<f32>,

	/// 1 makes styled lights bounce (e.g. flickering or switchable lights), default is 0, they do not bounce.
	pub _bouncestyled: IntBool,

	/// When set to 1, spotlight falloff is calculated from the distance to the targeted info_null. Ignored when "_falloff" is not 0. Default 0.
	pub _spotlightautofalloff: IntBool,

	/// Whether to use Q1-style surface subdivision (0) or Q2-style surface radiosity.
	pub _surflight_radiosity: IntBool,

	/// (Not documented.)
	pub _sky_surface: Option<Vec3>,

	/// (Not documented.)
	pub _sun_surface: Option<Vec3>,

	/// (Not documented.)
	pub _compilerstyle_start: Option<f32>,

	/// (Not documented.)
	pub _compilerstyle_max: Option<f32>,

	/// Scale factor for automatic minlight on an emissive face, derived from the light color being emitted.
	///
	/// This is intended to prevent, e.g., a light fixture texture which is configured as a surface light, from being completely black.
	///
	/// Default 1.0, can set to 0.0 to disable minlight.
	#[default(1.)]
	pub _surflight_minlight_scale: f32,
}

#[derive(FgdType, Reflect, Debug, Clone, Default, Serialize, Deserialize)]
#[number_key]
pub enum DirtMode {
	#[default]
	Ordered = 0,
	Randomized = 1,
}

/// Contains properties used by the `ericw-tools` compiler for any entity with a classname starting with the first five letters "light". E.g. "light", "light_globe", "light_flame_small_yellow", etc.
#[base_class(
	classname("__bsp_light"),
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

/// `ericw-tools` qbsp has a prefab system using a point entity named “misc_external_map”.
/// The idea is, each “misc_external_map” imports brushes from an external .map file,
/// applies rotations specified by the “_external_map_angles” key,
/// then translates them to the “origin” key of the “misc_external_map” entity.
/// Finally, the classname of the “misc_external_map” is switched to the one provided by the mapper in the “_external_map_classname” key.
/// (The “origin” key is also cleared to “0 0 0” before saving the .bsp).
///
/// The external .map file should consist of worldspawn brushes only, although you can use func_group for editing convenience.
/// Brush entities are merged with the worldspawn brushes during import.
/// All worldspawn keys, and any point entities are ignored.
/// Currently, this means that the “wad” key is not handled,
/// so you need to add any texture wads required by the external .map file to your main map.
///
/// Note that you can set other entity keys on the “misc_external_map” to configure the final entity type.
/// e.g. if you set “_external_map_classname” to “func_door”,
/// you can also set a “targetname” key on the “misc_external_map”, or any other keys for “func_door”.
#[base_class(
	classname("__bsp_external_map"),
)]
#[derive(Debug, Clone, SmartDefault, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct BspExternalMap {
	/// Specifies the filename of the .map to import.
	#[must_set]
	pub _external_map: String,

	/// What entity you want the external map to turn in to.
	/// You can use internal qbsp entity types such as func_detail,
	/// or a regular solid entity classname like “func_wall” or “func_door”.
	pub _external_map_classname: Option<String>,

	/// Rotation for the prefab, “pitch yaw roll” format.
	/// Assuming the exernal map is facing the +X axis, positive pitch is down.
	/// Yaw of 180, for example, would rotate it to face -X.
	pub _external_map_angles: Option<Vec3>,

	/// Short version of `_external_map_angles` for when you want to specify just a yaw rotation.
	pub _external_map_angle: Option<Vec3>,

	/// Scale factor for the prefab, defaults to 1. Either specify a single value or three scales, “x y z”.
	#[default(Vec3::ONE)]
	pub _external_map_scale: Vec3,
}
