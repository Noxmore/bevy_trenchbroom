flat! {
	hooks;
	main_impl;
	#[cfg(feature = "client")]
	set_sampler;
	tb_types;
	writing;
}

pub use crate::bevy_materialize::load::simple::SimpleGenericMaterialLoader;
use bevy::{
	asset::{AssetPath, LoadContext, RenderAssetUsages},
	image::ImageSampler,
	tasks::BoxedFuture,
};
use fgd::FgdType;
use qmap::QuakeMapEntities;
use util::{BevyTrenchbroomCoordinateConversions, ImageSamplerRepeatExt};

use crate::{class::QuakeClassSpawnView, *};

pub struct ConfigPlugin;
impl Plugin for ConfigPlugin {
	fn build(&self, #[allow(unused)] app: &mut App) {
		#[cfg(feature = "client")]
		app.add_systems(Update, Self::set_image_samplers);
	}
}

/// The main configuration structure of bevy_trenchbroom.
#[derive(Debug, Clone, SmartDefault, DefaultBuilder)]
pub struct TrenchBroomConfig {
	/// The format version of the TrenchBroom config file, you almost certainly should not change this.
	#[default(9)]
	pub tb_format_version: u16,

	/// How many units in the trenchbroom world take up 1 unit in the bevy world. (Default: ~40, 1 unit = 1 inch)
	#[default(39.37008)]
	pub scale: f32,

	/// The path to your game assets, should be the same as in your asset plugin. Probably does not support processed assets (I haven't tested). (Default: "assets")
	#[default("assets".into())]
	#[builder(into)]
	pub assets_path: PathBuf,

	/// The name of your game.
	#[builder(into)]
	pub name: String,

	/// Optional icon for the TrenchBroom UI. Contains the data of a PNG file. Should be 32x32 or it will look weird in the UI.
	/// By default, the Bevy logo is used.
	#[default(Some(include_bytes!("default_icon.png").into()))]
	pub icon: Option<Cow<'static, [u8]>>,
	/// Supported map file formats. Currently, only the loading of [`Valve`](MapFileFormat::Valve) is supported.
	///
	/// (Default: [`MapFileFormat::Valve`])
	#[default(vec![MapFileFormat::Valve])]
	#[builder(into)]
	pub file_formats: Vec<MapFileFormat>,
	/// The format for asset packages. If you are just using loose files, this probably doesn't matter to you, and you can leave it defaulted.
	#[builder(skip)] // bevy_trenchbroom currently *only* supports loose files
	package_format: AssetPackageFormat,

	/// The root directory to look for textures in the [`assets_path`](Self::assets_path). (Default: "textures")
	///
	/// NOTE: If you are using ericw-tools, this is currently hardcoded to only work with "textures". ([see issue](https://github.com/ericwa/ericw-tools/issues/451))
	#[default("textures".into())]
	#[builder(into)]
	pub material_root: PathBuf,
	/// The supported extensions of your texture files. This is also used for material loading as a fallback. (Default: ["png"])
	///
	/// Each one of these adds a filesystem call to check if the file exists when loading loose textures, so try to keep this to what you absolutely need.
	#[default(["png".s()].into())]
	#[builder(into)]
	pub texture_extensions: Vec<String>,
	/// The palette file path and data used for WADs. The path roots from your [`AssetServer`]'s assets folder.
	///
	/// For the default quake palette (what you most likely want to use), there is a [free download on the Quake wiki](https://quakewiki.org/wiki/File:quake_palette.zip),
	/// and a copy distributed by `qbsp` in the form of [`QUAKE_PALETTE`].
	///
	/// If TrenchBroom can't find this palette file, all WAD textures will be black. If `bevy_trenchbroom` can't find the file, it will default to [`QUAKE_PALETTE`].
	///
	/// (Default: "palette.lmp")
	#[cfg(feature = "bsp")]
	#[builder(into)]
	#[default("palette.lmp".into())]
	pub texture_pallette: PathBuf,
	/// For BSPs.
	#[cfg(not(feature = "bsp"))]
	#[builder(into)]
	#[default("palette.lmp".into())]
	pub texture_pallette: PathBuf,

	/// Patterns to match to exclude certain texture files from showing up in-editor. (Default: [`TrenchBroomConfig::default_texture_exclusions`]).
	#[builder(into)]
	#[default(Self::default_texture_exclusions())]
	pub texture_exclusions: Vec<String>,

	/// The default color for entities in RGBA. (Default: 0.6 0.6 0.6 1.0)
	#[default(vec4(0.6, 0.6, 0.6, 1.0))]
	#[builder(into)]
	pub entity_default_color: Vec4,
	/// An expression to evaluate how big entities' models are. Any instances of the string "%%scale%%" will be replaced wit with this config's scale. (Default: `{{ scale == undefined -> %%scale%%, scale }}``)
	#[default(Some("{{ scale == undefined -> %%scale%%, scale }}".s()))]
	#[builder(into)]
	pub entity_scale_expression: Option<String>,
	/// Whether to set property defaults into an entity on creation, or leave them to use the default value that is defined in entity definitions. It is not recommended to use this.
	pub entity_set_default_properties: bool,

	/// Tags to apply to brushes.
	#[builder(into)]
	pub brush_tags: Vec<TrenchBroomTag>,
	/// Tags to apply to brush faces. The default is defined by [`TrenchBroomConfig::empty_face_tag`], and all it does is make `__TB_empty` transparent.
	#[default(vec![Self::empty_face_tag()])]
	#[builder(into)]
	pub face_tags: Vec<TrenchBroomTag>,

	/// Game-defined flags per face.
	///
	/// TODO: Currently, these do not save in the map file, or are able to be loaded, as it requires [MapFileFormat::Quake2] or higher, which isn't supported yet.
	#[builder(into)]
	pub surface_flags: Vec<BitFlag>,
	/// Game-defined flags per face.
	/// According to [TrenchBroom docs](https://trenchbroom.github.io/manual/latest/#game_configuration_files), unlike [`Self::surface_flags`], this is "generally affecting the behavior of the brush containing the face".
	///
	/// TODO: Currently, these do not save in the map file, or are able to be loaded, as it requires [MapFileFormat::Quake2] or higher, which isn't supported yet.
	#[builder(into)]
	pub content_flags: Vec<BitFlag>,

	pub default_face_attributes: DefaultFaceAttributes,

	/// The optional bounding box enclosing the map to draw in the 2d viewports.
	///
	/// The two values are the bounding box min, and max respectively.
	///
	/// NOTE: This bounding box is in TrenchBroom space (Z up).
	pub soft_map_bounds: Option<[Vec3; 2]>,

	/// The file extension used when loading [`GenericMaterial`]s.
	///
	/// With the default loose texture loader, if a file with this asset doesn't exist,
	/// it tries to load it with [`SimpleGenericMaterialLoader`]
	/// with this config's [`texture_extensions`](Self::texture_extensions)
	///
	/// Each one of these adds a filesystem call to check if the file exists when loading loose textures, so try to keep this to what you absolutely need.
	///
	/// (Default: "toml" because the default material deserializer is toml)
	#[default(["toml".s()].into())]
	#[builder(into)]
	pub generic_material_extensions: Vec<String>,

	/// If `Some`, sets the lightmap exposure on any `StandardMaterial` loaded. (Default: Some(10,000))
	#[cfg(all(feature = "client", feature = "bsp"))]
	#[default(Some(10_000.))]
	pub lightmap_exposure: Option<f32>,
	#[cfg(all(feature = "client", feature = "bsp"))]
	#[default(500.)]
	pub default_irradiance_volume_intensity: f32,
	/// Multipliers to the colors of BSP loaded irradiance volumes depending on direction.
	///
	/// This is because light-grid-loaded irradiance volumes don't have any directionality.
	/// This fakes it, making objects within look a little nicer.
	///
	/// (Default: [`IrradianceVolumeMultipliers::SLIGHT_SHADOW`])
	#[cfg(all(feature = "client", feature = "bsp"))]
	#[default(IrradianceVolumeMultipliers::SLIGHT_SHADOW)]
	pub irradiance_volume_multipliers: IrradianceVolumeMultipliers,

	/// Whether to ignore map entity spawning errors for not having an entity definition for the map entity in question's classname. (Default: false)
	pub suppress_invalid_entity_definitions: bool,

	/// Whether to disable bsp lighting (lightmaps and irradiance volumes). This is for rendering backends where these aren't supported like OpenGL.
	#[cfg(feature = "bsp")]
	pub no_bsp_lighting: bool,

	#[cfg(feature = "bsp")]
	#[builder(skip)]
	#[default(Hook(Arc::new(Self::default_load_embedded_texture)))]
	pub load_embedded_texture: Hook<LoadEmbeddedTextureFn>,
	#[builder(skip)]
	#[default(Hook(Arc::new(Self::default_load_loose_texture)))]
	pub load_loose_texture: Hook<LoadLooseTextureFn>,

	/// Default frames per second for embedded animated textures.
	///
	/// If [`Some`], embedded textures with names starting with `+<0..9>` will become animated, going to the next number in the range, and if it doesn't exist, looping back around to 0.
	///
	/// (Default: `Some(5)`)
	#[cfg(feature = "bsp")]
	#[default(Some(5.))]
	pub embedded_texture_animation_fps: Option<f32>,

	/// If [`Some`], embedded textures with names that start with "sky" will be split in two.
	///
	/// Using the material provided by the contained function, the left side being the foreground, and right side the background.
	///
	/// (Default: `Some(QuakeSkyMaterial::default)`)
	#[cfg(all(feature = "client", feature = "bsp"))]
	#[default(Some(default))]
	pub embedded_quake_sky_material: Option<fn() -> QuakeSkyMaterial>,

	/// If [`Some`], embedded textures with names that start with `*` will use [`LiquidMaterial`], and will abide by the `water_alpha` worldspawn key.
	///
	/// (Default: `Some(QuakeSkyMaterial::default)`)
	#[cfg(all(feature = "client", feature = "bsp"))]
	#[default(Some(default))]
	pub embedded_liquid_material: Option<fn() -> LiquidMaterialExt>,

	/// If `true`, embedded textures with names starting with `{` will be given the alpha mode [`Mask(0.5)`](AlphaMode::Mask), and pixels with the index value `255` will be turned transparent.
	///
	/// (Default: `true`)
	#[cfg(feature = "bsp")]
	#[default(true)]
	pub embedded_texture_cutouts: bool,

	/// Set of textures to skip meshes of on map load. (Default: `["clip", "skip", "__TB_empty"]`)
	#[default(["clip".s(), "skip".s(), "__TB_empty".s()].into())]
	#[builder(into)]
	pub auto_remove_textures: HashSet<String>,

	/// If a brush is fully textured with the name of one of these when loading a `.map` file, it will set the transformation origin of the entity to which it belongs to the center of the brush, removing the origin brush after.
	///
	/// This allows, for example, your `func_rotate` entity to easily rotate around a specific point.
	///
	/// (Default: `["origin"]`)
	#[default(["origin".s()].into())]
	#[builder(into)]
	pub origin_textures: HashSet<String>,

	/// How lightmaps atlas' are computed when loading BSP files.
	///
	/// It's worth noting that `wgpu` has a [texture size limit of 2048](https://github.com/gfx-rs/wgpu/discussions/2952), which can be expanded via [`RenderPlugin`](bevy::render::RenderPlugin) if needed.
	///
	/// NOTE: `special_lighting_color` is set to gray (`75`) by default instead of white (`255`), because otherwise all textures with it look way too bright and washed out, not sure why.
	///
	/// (Default: [`Self::default_compute_lightmap_settings`])
	#[cfg(feature = "bsp")]
	#[default(Self::default_compute_lightmap_settings())]
	pub compute_lightmap_settings: ComputeLightmapSettings,

	/// `qbsp` settings when parsing BSP files.
	#[cfg(feature = "bsp")]
	pub bsp_parse_settings: BspParseSettings,

	/// Entity spawners that get run on every single entity (after the regular spawners), regardless of classname. (Default: [`TrenchBroomConfig::default_global_spawner`])
	#[builder(skip)]
	#[default(Hook(Arc::new(Self::default_global_spawner)))]
	pub global_spawner: Hook<SpawnFn>,

	/// Spawn hooks to run on solid classes unless overridden.
	#[default(SpawnHooks::new)]
	pub default_solid_spawn_hooks: fn() -> SpawnHooks,
	/// Spawn hooks to run on point classes unless overridden.
	#[default(SpawnHooks::new)]
	pub default_point_spawn_hooks: fn() -> SpawnHooks,
	/// Spawn hooks to run on base classes unless overridden.
	#[default(SpawnHooks::new)]
	pub default_base_spawn_hooks: fn() -> SpawnHooks,

	/// Whether to apply the translation and rotation fields regardless of having [`Transform`] as a base class.
	/// Adds convenance and removes a foot-gun, but is less flexible, hence why it is disableable.
	///
	/// NOTE: This doesn't use the scale field as it is not hardcoded unlike the others. If you want to use it, then you should use [`Transform`] as a base class.
	///
	/// (Default: `true`)
	#[default(true)]
	pub global_transform_application: bool,

	/// The image sampler used with textures loaded from maps.
	/// Use [`Self::linear_filtering`] to easily switch to smooth texture interpolation.
	/// (Default: [`Self::default_texture_sampler`] (repeating nearest neighbor))
	#[default(Self::default_texture_sampler())]
	pub texture_sampler: ImageSampler,

	/// If `true`, lightmaps spawned from BSPs will use bicubic filtering. (Default: `false`)
	///
	/// NOTE: It's recommended you add a pixel of padding in [`compute_lightmap_settings`](Self::compute_lightmap_settings), otherwise there will be obvious lightmap leaking.
	#[cfg(feature = "bsp")]
	pub bicubic_lightmap_filtering: bool,

	/// Whether brush meshes are kept around in memory after they're sent to the GPU. Default: [`RenderAssetUsages::all`] (kept around)
	#[default(RenderAssetUsages::all())]
	pub brush_mesh_asset_usages: RenderAssetUsages,

	/// Whether BSP loaded textures and lightmaps are kept around in memory after they're sent to the GPU. Default: [`RenderAssetUsages::RENDER_WORLD`] (not kept around)
	#[cfg(feature = "bsp")]
	#[default(RenderAssetUsages::RENDER_WORLD)]
	pub bsp_textures_asset_usages: RenderAssetUsages,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn coordinate_conversions() {
		let config = TrenchBroomConfig::default();

		let input = vec3(20.6, 1.72, 9.0);
		assert_eq!(config.from_bevy_space(config.to_bevy_space(input)), input);
	}
}
