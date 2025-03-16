use bevy::{
	asset::{io::AssetReaderError, AssetLoadError, LoadContext},
	image::{ImageLoaderSettings, ImageSampler},
	render::render_asset::RenderAssetUsages,
	utils::BoxedFuture,
};
use bsp::GENERIC_MATERIAL_PREFIX;
use class::{default_quake_class_registry, ErasedQuakeClass, QuakeClass};
use fgd::FgdType;
use geometry::{GeometryProviderFn, GeometryProviderView};
use qmap::{QuakeMapEntities, QuakeMapEntity};
use util::{trenchbroom_gltf_rotation_fix, BevyTrenchbroomCoordinateConversions, ImageSamplerRepeatExt};

use crate::*;

pub type LoadEmbeddedTextureFn = dyn for<'a, 'b> Fn(EmbeddedTextureLoadView<'a, 'b>) -> BoxedFuture<'a, Handle<GenericMaterial>> + Send + Sync;
pub type LoadLooseTextureFn = dyn for<'a, 'b> Fn(TextureLoadView<'a, 'b>) -> BoxedFuture<'a, Handle<GenericMaterial>> + Send + Sync;
pub type SpawnFn = dyn Fn(&TrenchBroomConfig, &QuakeMapEntity, &mut EntityWorldMut) -> anyhow::Result<()> + Send + Sync;

/// The main configuration structure of bevy_trenchbroom.
#[derive(Debug, Clone, SmartDefault, DefaultBuilder)]
pub struct TrenchBroomConfig {
	/// The format version of the TrenchBroom config file, you almost certainly should not change this.
	#[default(9)]
	pub tb_format_version: u16,

	/// How many units in the trenchbroom world take up 1 unit in the bevy world. (Default: ~40, 1 unit = 1 inch)
	#[default(39.37008)]
	pub scale: f32,

	/// Whether the current instance of this application is a server, if true, this will disable unnecessary features such as brush mesh rendering.
	pub is_server: bool,

	/// The path to your game assets, should be the same as in your asset plugin. Probably does not support processed assets (I haven't tested). (Default: "assets")
	#[default("assets".into())]
	#[builder(into)]
	pub assets_path: PathBuf,

	/// The name of your game.
	#[builder(into)]
	pub name: String,

	/// Optional icon for the TrenchBroom UI. Contains the data of a PNG file. Should be 32x32 or it will look weird in the UI.
	pub icon: Option<Vec<u8>>,
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
	/// The extension of your texture files. This is also used for material loading as a fallback. (Default: "png")
	#[default("png".into())]
	#[builder(into)]
	pub texture_extension: String,
	/// The palette file path and data used for WADs. The path roots from your [`AssetServer`]'s assets folder.
	///
	/// For the default quake palette (what you most likely want to use), there is a [free download on the Quake wiki](https://quakewiki.org/wiki/File:quake_palette.zip),
	/// and a copy distributed by `qbsp` in the form of [`QUAKE_PALETTE`].
	///
	/// If TrenchBroom can't find this palette file, all WAD textures will be black. If `bevy_trenchbroom` can't find the file, it will default to [`QUAKE_PALETTE`].
	///
	/// (Default: "palette.lmp")
	#[builder(into)]
	#[default("palette.lmp".into())] // TODO
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
	/// it tries to load it with [`SimpleGenericMaterialLoader`](bevy_materialize::load::SimpleGenericMaterialLoader)
	/// with this config's [`texture_extension`](Self::texture_extension)
	///
	/// (Default: "material")
	#[default("material".s())]
	#[builder(into)]
	pub generic_material_extension: String,

	/// If `Some`, sets the lightmap exposure on any `StandardMaterial` loaded. (Default: Some(10,000))
	#[cfg(feature = "bevy_pbr")]
	#[default(Some(10_000.))]
	#[builder(into)]
	pub lightmap_exposure: Option<f32>,
	#[cfg(feature = "bevy_pbr")]
	#[default(500.)]
	pub default_irradiance_volume_intensity: f32,
	/// Multipliers to the colors of BSP loaded irradiance volumes depending on direction.
	///
	/// This is because light-grid-loaded irradiance volumes don't have any directionality.
	/// This fakes it, making objects within look a little nicer.
	///
	/// (Default: [`IrradianceVolumeMultipliers::SLIGHT_SHADOW`])
	#[cfg(feature = "bevy_pbr")]
	#[default(IrradianceVolumeMultipliers::SLIGHT_SHADOW)]
	pub irradiance_volume_multipliers: IrradianceVolumeMultipliers,

	/// Whether to ignore map entity spawning errors for not having an entity definition for the map entity in question's classname. (Default: false)
	pub suppress_invalid_entity_definitions: bool,

	/// Whether to disable bsp lighting (lightmaps and irradiance volumes). This is for rendering backends where these aren't supported like OpenGL.
	pub no_bsp_lighting: bool,

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
	#[default(Some(5.))]
	pub embedded_texture_animation_fps: Option<f32>,

	/// If [`Some`], embedded textures with names that start with "sky" will be split in two.
	///
	/// Using the material provided by the contained function, the left side being the foreground, and right side the background.
	///
	/// (Default: `Some(QuakeSkyMaterial::default)`)
	#[cfg(feature = "bevy_pbr")]
	#[default(Some(default))]
	pub embedded_quake_sky_material: Option<fn() -> QuakeSkyMaterial>,

	/// If [`Some`], embedded textures with names that start with `*` will use [`LiquidMaterial`], and will abide by the `water_alpha` worldspawn key.
	///
	/// (Default: `Some(QuakeSkyMaterial::default)`)
	#[cfg(feature = "bevy_pbr")]
	#[default(Some(default))]
	pub embedded_liquid_material: Option<fn() -> LiquidMaterialExt>,

	/// If `true`, embedded textures with names starting with `{` will be given the alpha mode [`Mask(0.5)`](AlphaMode::Mask), and pixels with the index value `255` will be turned transparent.
	///
	/// (Default: `true`)
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
	/// NOTE: Entities that support this need to have [`Transform`] as a base class, or they will appear at the world origin.
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
	#[default(ComputeLightmapSettings { special_lighting_color: [75; 3], ..default() })]
	pub compute_lightmap_settings: ComputeLightmapSettings,

	/// `qbsp` settings when parsing BSP files.
	pub bsp_parse_settings: BspParseSettings,

	/// Registered entity classes to be outputted in the fgd file, and used when spawning into scenes. (Default: [`default_quake_class_registry`])
	#[default(default_quake_class_registry())]
	#[builder(skip)]
	pub entity_classes: HashMap<&'static str, Cow<'static, ErasedQuakeClass>>,

	/// Entity spawners that get run on every single entity (after the regular spawners), regardless of classname. (Default: [`TrenchBroomConfig::default_global_spawner`])
	#[builder(skip)]
	#[default(Hook(Arc::new(Self::default_global_spawner)))]
	pub global_spawner: Hook<SpawnFn>,

	/// Geometry provider run after all others for all entities regardless of classname. (Default: [`TrenchBroomConfig::default_global_geometry_provider`])
	#[builder(skip)]
	#[default(Hook(Arc::new(Self::default_global_geometry_provider)))]
	pub global_geometry_provider: Hook<GeometryProviderFn>,

	/// The image sampler used with textures loaded from maps. (Default: [`Self::default_texture_sampler`] (repeating nearest neighbor))
	#[default(Self::default_texture_sampler())]
	pub texture_sampler: ImageSampler,

	/// Whether brush meshes are kept around in memory after they're sent to the GPU. Default: [`RenderAssetUsages::all`] (kept around)
	#[default(RenderAssetUsages::all())]
	pub brush_mesh_asset_usages: RenderAssetUsages,

	/// Whether BSP loaded textures and lightmaps are kept around in memory after they're sent to the GPU. Default: [`RenderAssetUsages::RENDER_WORLD`] (not kept around)
	#[default(RenderAssetUsages::RENDER_WORLD)]
	pub bsp_textures_asset_usages: RenderAssetUsages,
}

impl TrenchBroomConfig {
	/// Creates a new TrenchBroom config. It is recommended to use this over [`TrenchBroomConfig::default`]
	pub fn new(name: impl Into<String>) -> Self {
		Self::default().name(name)
	}

	/// Inserts a new texture to auto-remove.
	pub fn auto_remove_texture(mut self, texture: impl ToString) -> Self {
		self.auto_remove_textures.insert(texture.to_string());
		self
	}

	/// Excludes "\*_normal", "\*_mr" (Metallic and roughness), "\*_emissive", and "\*_depth".
	pub fn default_texture_exclusions() -> Vec<String> {
		vec!["*_normal".into(), "*_mr".into(), "*_emissive".into(), "*_depth".into()]
	}

	/// (See documentation on [`TrenchBroomConfig::face_tags`])
	pub fn empty_face_tag() -> TrenchBroomTag {
		TrenchBroomTag::new("empty", "__TB_empty").attributes([TrenchBroomTagAttribute::Transparent])
	}

	/// A repeating, nearest-neighbor sampler.
	pub fn default_texture_sampler() -> ImageSampler {
		ImageSampler::nearest().repeat()
	}

	/// Switches to using linear (smooth) filtering on textures.
	pub fn linear_filtering(self) -> Self {
		self.texture_sampler(ImageSampler::linear().repeat())
	}

	/// Names the entity based on the classname, and `targetname` if the property exists. (See documentation on [`TrenchBroomConfig::global_spawner`])
	///
	/// If the entity is a brush entity, rotation is reset.
	pub fn default_global_spawner(config: &TrenchBroomConfig, src_entity: &QuakeMapEntity, entity: &mut EntityWorldMut) -> anyhow::Result<()> {
		let classname = src_entity.classname()?.s();

		// For things like doors where the `angles` property means open direction.
		if let Some(mut transform) = entity.get_mut::<Transform>() {
			if config.get_class(&classname).map(|class| class.info.ty.is_solid()) == Some(true) {
				transform.rotation = Quat::IDENTITY;
			}
		}

		entity.insert(Name::new(
			src_entity
				.get::<String>("targetname")
				.map(|name| format!("{classname} ({name})"))
				.unwrap_or(classname),
		));

		trenchbroom_gltf_rotation_fix(entity);

		Ok(())
	}

	/// Adds [`Visibility`] and [`Transform`] components if they aren't in the entity, as it is needed to clear up warnings for child meshes.
	pub fn default_global_geometry_provider(view: &mut GeometryProviderView) {
		let mut ent = view.world.entity_mut(view.entity);

		if !ent.contains::<Visibility>() {
			ent.insert(Visibility::default());
		}
		if !ent.contains::<Transform>() {
			ent.insert(Transform::default());
		}
	}

	pub fn load_embedded_texture_fn(mut self, provider: impl FnOnce(Arc<LoadEmbeddedTextureFn>) -> Arc<LoadEmbeddedTextureFn>) -> Self {
		self.load_embedded_texture.set(provider);
		self
	}
	pub fn default_load_embedded_texture<'a>(
		#[allow(unused_mut)] mut view: EmbeddedTextureLoadView<'a, '_>,
	) -> BoxedFuture<'a, Handle<GenericMaterial>> {
		Box::pin(async move {
			#[cfg(feature = "bevy_pbr")]
			let mut material = StandardMaterial {
				base_color_texture: Some(view.image_handle.clone()),
				perceptual_roughness: 1.,
				..default()
			};

			#[cfg(feature = "bevy_pbr")]
			if let Some(alpha_mode) = view.alpha_mode {
				material.alpha_mode = alpha_mode;
			}

			#[cfg(feature = "bevy_pbr")]
			let generic_material = match special_textures::load_special_texture(&mut view, &material) {
				Some(v) => v,
				None => GenericMaterial {
					handle: view.add_material(material).into(),
					properties: default(),
				},
			};

			#[cfg(not(feature = "bevy_pbr"))]
			let generic_material = GenericMaterial::default();

			view.parent_view
				.load_context
				.add_labeled_asset(format!("{GENERIC_MATERIAL_PREFIX}{}", view.name), generic_material)
		})
	}

	pub fn load_loose_texture_fn(mut self, provider: impl FnOnce(Arc<LoadLooseTextureFn>) -> Arc<LoadLooseTextureFn>) -> Self {
		self.load_loose_texture.set(provider);
		self
	}
	/// Tries to load a [`GenericMaterial`] with the [`generic_material_extension`](Self::generic_material_extension), as a fallback tries [`texture_extension`](Self::texture_extension).
	pub fn default_load_loose_texture<'a>(view: TextureLoadView<'a, '_>) -> BoxedFuture<'a, Handle<GenericMaterial>> {
		Box::pin(async move {
			let path = view
				.tb_config
				.material_root
				.join(format!("{}.{}", view.name, view.tb_config.generic_material_extension));
			// Because i can't just check if an asset exists, i have to load it twice.
			match view.load_context.loader().immediate().load::<GenericMaterial>(path.clone()).await {
				Ok(_) => {
					let texture_sampler = view.tb_config.texture_sampler.clone();
					view.load_context
						.loader()
						.with_settings(move |s: &mut ImageLoaderSettings| s.sampler = texture_sampler.clone())
						.load(path)
				}
				Err(err) => match err.error {
					AssetLoadError::AssetReaderError(AssetReaderError::NotFound(_)) => {
						let texture_sampler = view.tb_config.texture_sampler.clone();
						view.load_context
							.loader()
							.with_settings(move |s: &mut ImageLoaderSettings| s.sampler = texture_sampler.clone())
							.load(
								view.tb_config
									.material_root
									.join(format!("{}.{}", view.name, view.tb_config.texture_extension)),
							)
					}

					err => {
						error!("Loading map {}: {err}", view.load_context.asset_path());
						Handle::default()
					}
				},
			}
		})
	}

	/// Returns a copy of [`Self::entity_scale_expression`], replacing all instances of "%%scale%%" with this config's scale.
	pub fn get_entity_scale_expression(&self) -> Option<String> {
		self.entity_scale_expression
			.as_ref()
			.map(|s| s.replace("%%scale%%", &self.scale.to_string()))
	}

	/// Retrieves the entity class of `classname` from this config. If none is found and the `auto_register` feature is enabled, it'll try to find it in [`GLOBAL_CLASS_REGISTRY`](crate::class::GLOBAL_CLASS_REGISTRY).
	pub fn get_class(&self, classname: &str) -> Option<&ErasedQuakeClass> {
		#[cfg(not(feature = "auto_register"))]
		{
			self.entity_classes.get(classname).map(Cow::as_ref)
		}

		#[cfg(feature = "auto_register")]
		{
			self.entity_classes
				.get(classname)
				.map(Cow::as_ref)
				.or_else(|| class::GLOBAL_CLASS_REGISTRY.get(classname).copied())
		}
	}

	/// A list of all registered classes. If the `auto_register` feature is enabled, also includes [`GLOBAL_CLASS_REGISTRY`](crate::class::GLOBAL_CLASS_REGISTRY).
	pub fn class_iter(&self) -> impl Iterator<Item = &ErasedQuakeClass> {
		#[cfg(not(feature = "auto_register"))]
		{
			self.entity_classes
				.values()
				.map(Cow::as_ref)
				.sorted_by(|a, b| a.info.name.cmp(b.info.name))
		}

		#[cfg(feature = "auto_register")]
		{
			self.entity_classes
				.values()
				.map(Cow::as_ref)
				.chain(class::GLOBAL_CLASS_REGISTRY.values().copied())
				.sorted_by(|a, b| a.info.name.cmp(b.info.name))
		}
	}

	/// Registers a [`QuakeClass`] into this config. It will be outputted into the fgd, and will be used when loading entities into scenes.
	///
	/// If the `auto_register` feature is enabled, you don't have to do this, as it automatically puts classes into a global registry when [`QuakeClass`] is derived.
	pub fn register_class<T: QuakeClass>(mut self) -> Self {
		self.entity_classes.insert(T::CLASS_INFO.name, Cow::Borrowed(T::ERASED_CLASS));
		self
	}

	/// Register an owned [`ErasedQuakeClass`] directly for dynamic classes. You almost always want to be using [`Self::register_class`] instead.
	pub fn register_class_dynamic(mut self, class: ErasedQuakeClass) -> Self {
		self.entity_classes.insert(class.info.name, Cow::Owned(class));
		self
	}

	/// Converts from a z-up coordinate space to a y-up coordinate space, and scales everything down by this config's scale.
	pub fn to_bevy_space(&self, vec: Vec3) -> Vec3 {
		vec.z_up_to_y_up() / self.scale
	}

	/// Converts from a z-up coordinate space to a y-up coordinate space, and scales everything down by this config's scale.
	pub fn to_bevy_space_f64(&self, vec: DVec3) -> DVec3 {
		vec.z_up_to_y_up() / self.scale as f64
	}

	/// The opposite of [`Self::to_bevy_space`], converts from a y-up coordinate space to z-up, and scales everything up by this config's scale.
	pub fn from_bevy_space(&self, vec: Vec3) -> Vec3 {
		vec.y_up_to_z_up() * self.scale
	}

	/// The opposite of [`Self::to_bevy_space_f64`], converts from a y-up coordinate space to z-up, and scales everything up by this config's scale.
	pub fn from_bevy_space_f64(&self, vec: DVec3) -> DVec3 {
		vec.y_up_to_z_up() * self.scale as f64
	}
}

/// Various inputs available when loading textures.
pub struct TextureLoadView<'a, 'b> {
	pub name: &'a str,
	pub tb_config: &'a TrenchBroomConfig,
	pub load_context: &'a mut LoadContext<'b>,
	pub entities: &'a QuakeMapEntities,
	/// `Some` if it is determined that a specific alpha mode should be used for a material, such as in some embedded textures.
	pub alpha_mode: Option<AlphaMode>,
	/// If the map contains embedded textures, this will be a map of texture names to image handles.
	/// This is useful for things like animated textures.
	pub embedded_textures: Option<&'a HashMap<&'a str, (Image, Handle<Image>)>>,
}
impl TextureLoadView<'_, '_> {
	/// Shorthand for adding a material asset with the correct label.
	#[cfg(feature = "bevy_pbr")]
	pub fn add_material<M: Material>(&mut self, material: M) -> Handle<M> {
		self.load_context.add_labeled_asset(format!("Material_{}", self.name), material)
	}
}

#[derive(Deref, DerefMut)]
pub struct EmbeddedTextureLoadView<'a, 'b> {
	#[deref]
	pub parent_view: TextureLoadView<'a, 'b>,

	/// The handle of the image of this embedded texture.
	pub image_handle: &'a Handle<Image>,
	/// The actual image data behind the texture.
	pub image: &'a Image,
}

/// Wrapper for storing a stack of dynamic functions. Use [`Hook::set`] to push a new function onto the stack.
#[derive(Deref)]
pub struct Hook<F: ?Sized>(pub Arc<F>);
impl<F: ?Sized + Send + Sync> fmt::Debug for Hook<F> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Hook<{}>", type_name::<F>())
	}
}
impl<F: ?Sized> Clone for Hook<F> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}
impl<F: ?Sized> Hook<F> {
	/// Sets the function in the hook using a function that takes the hook's previous function for the new function to optionally call.
	pub fn set(&mut self, provider: impl FnOnce(Arc<F>) -> Arc<F>) {
		self.0 = provider(self.0.clone());
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AssetPackageFormat {
	/// Simple ZIP file, uses the .zip extension, if you want to use another extension like pk3, `Other`
	#[default]
	Zip,
	/// Id pak file
	IdPack,
	/// Daikatana pak file
	DkPack,
	Other {
		extension: &'static str,
		format: &'static str,
	},
}

impl AssetPackageFormat {
	/// The extension used by this package format.
	pub fn extension(&self) -> &'static str {
		match self {
			Self::Zip => "zip",
			Self::IdPack => "pak",
			Self::DkPack => "pak",
			Self::Other { extension, format: _ } => extension,
		}
	}

	/// The format id used by this package format.
	pub fn format(&self) -> &'static str {
		match self {
			Self::Zip => "zip",
			Self::IdPack => "idpak",
			Self::DkPack => "dkpak",
			Self::Other { extension: _, format } => format,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MapFileFormat {
	Standard,
	#[default]
	Valve,
	Quake2,
	Quake2Valve,
	Quake3Legacy,
	Quake3Valve,
	Hexen2,
}

impl MapFileFormat {
	/// How this format is referred to in the config file.
	pub fn config_str(self) -> &'static str {
		match self {
			Self::Standard => "Standard",
			Self::Valve => "Valve",
			Self::Quake2 => "Quake2",
			Self::Quake2Valve => "Quake2 (Valve)",
			Self::Quake3Legacy => "Quake3 (Legacy)",
			Self::Quake3Valve => "Quake3 (Valve)",
			Self::Hexen2 => "Hexen2",
		}
	}
}

/// Tag for applying attributes to certain brushes/faces, for example, making a `trigger` material transparent.
#[derive(Debug, Clone, Default, DefaultBuilder)]
pub struct TrenchBroomTag {
	/// Name of the tag.
	#[builder(skip)]
	pub name: String,
	/// The attributes applied to the brushes/faces the tag targets.
	#[builder(into)]
	pub attributes: Vec<TrenchBroomTagAttribute>,
	/// The pattern to match for, if this is a brush tag, it will match against the `classname`, if it is a face tag, it will match against the material.
	#[builder(skip)]
	pub pattern: String,
	/// Only used if this is a brush tag. When this tag is applied by the use of its keyboard shortcut, then the selected brushes will receive this material if it is specified.
	#[builder(into)]
	pub material: Option<String>,
}

impl TrenchBroomTag {
	/// Creates a new tag.
	///
	/// The name is a simple name to identify the tag. The pattern is a pattern to match for allowing wildcards.
	/// If this is a brush tag, it will match against the `classname`, if it is a face tag, it will match against the material.
	pub fn new(name: impl Into<String>, pattern: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			pattern: pattern.into(),
			..default()
		}
	}

	pub(crate) fn to_json(&self, match_type: &str) -> json::JsonValue {
		let mut json = json::object! {
			"name": self.name.clone(),
			"attribs": self.attributes.iter().copied().map(TrenchBroomTagAttribute::config_str).collect::<Vec<_>>(),
			"match": match_type,
			"pattern": self.pattern.clone(),
		};

		if let Some(material) = &self.material {
			json.insert("material", material.clone()).unwrap();
		}

		json
	}
}

/// Attribute for [`TrenchBroomTag`], currently the only option is `Transparent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrenchBroomTagAttribute {
	Transparent,
}

impl TrenchBroomTagAttribute {
	/// How this attribute is referred to in the config file.
	pub fn config_str(self) -> &'static str {
		match self {
			Self::Transparent => "transparent",
		}
	}
}

/// Definition of a bit flag for [`TrenchBroomConfig`]. The position of the flag definition determines the position of the bit.
#[derive(Debug, Clone, Default)]
pub enum BitFlag {
	#[default]
	Unused,
	/// Shows up in-editor with the specified name and optional description.
	Used { name: String, description: Option<String> },
}
impl From<BitFlag> for json::JsonValue {
	fn from(value: BitFlag) -> Self {
		match value {
			BitFlag::Unused => json::object! { "unused": true },
			BitFlag::Used { name, description } => {
				let mut json = json::object! {
					"name": name.clone(),
				};

				if let Some(description) = description {
					json.insert("description", description.clone()).unwrap();
				}

				json
			}
		}
	}
}

#[derive(Debug, Clone, Default)]
pub struct DefaultFaceAttributes {
	/// If [`Some`], overrides the default x and y texture offset.
	pub offset: Option<Vec2>,
	/// If [`Some`], overrides the default texture scale.
	pub scale: Option<Vec2>,
	/// If [`Some`], overrides the default texture rotation.
	pub rotation: Option<f32>,
	/// Number specifying the default surface value (only applicable if surfaceflags exist)
	pub surface_value: Option<u32>,
	/// List of strings naming the default surface flags
	pub surface_flags: Vec<String>,
	/// List of strings naming the default content flags
	pub content_flags: Vec<String>,
	/// The default surface color (only applicable for Daikatana)
	pub color: Option<Srgba>,
}
impl DefaultFaceAttributes {
	/// Returns `true` if any attribute is set to something not the default, else `false`.
	pub fn is_any_set(&self) -> bool {
		self.offset.is_some()
			|| self.scale.is_some()
			|| self.rotation.is_some()
			|| self.surface_value.is_some()
			|| !self.surface_flags.is_empty()
			|| !self.content_flags.is_empty()
			|| self.color.is_some()
	}
}
impl From<&DefaultFaceAttributes> for json::JsonValue {
	fn from(value: &DefaultFaceAttributes) -> Self {
		let mut json = json::JsonValue::new_object();

		if let Some(value) = value.offset {
			json.insert("offset", value.to_array().as_ref()).unwrap();
		}
		if let Some(value) = value.scale {
			json.insert("scale", value.to_array().as_ref()).unwrap();
		}
		if let Some(value) = value.rotation {
			json.insert("rotation", value).unwrap();
		}
		if let Some(value) = value.surface_value {
			json.insert("surfaceValue", value).unwrap();
		}
		if !value.surface_flags.is_empty() {
			json.insert::<&[_]>("surfaceFlags", &value.surface_flags).unwrap();
		}
		if !value.content_flags.is_empty() {
			json.insert::<&[_]>("surfaceContents", &value.content_flags).unwrap();
		}
		if let Some(value) = value.color {
			json.insert("scale", value.to_f32_array().as_ref()).unwrap();
		}

		json
	}
}

impl TrenchBroomConfig {
	/// Writes the configuration into a folder, it is your choice when to do this in your application, and where you want to save the config to.
	pub fn write_folder(&self, folder: impl AsRef<Path>) -> io::Result<()> {
		if self.name.is_empty() {
			return Err(io::Error::new(
				io::ErrorKind::InvalidData,
				"Please set a name for your TrenchBroom config. \
				If you have, make sure you call `write_into` after the app is built. (e.g. In a startup system)",
			));
		}

		let folder = folder.as_ref();

		//////////////////////////////////////////////////////////////////////////////////
		//// GAME CONFIGURATION && ICON
		//////////////////////////////////////////////////////////////////////////////////

		// The game config file is basically json, so we can get 99% of the way there by just creating a json object.
		let mut json = json::object! {
			"version": self.tb_format_version,
			"name": self.name.clone(),
			"fileformats": self.file_formats.iter().map(|format| json::object! { "format": format.config_str() }).collect::<Vec<_>>(),
			"filesystem": {
				"searchpath": self.assets_path.s(),
				"packageformat": { "extension": self.package_format.extension(), "format": self.package_format.format() }
			},
			"materials": {
				"root": self.material_root.s(),
				// .D is required for WADs to work
				"extensions": [".D", self.texture_extension.clone()],
				"palette": self.texture_pallette.s(),
				"attribute": "wad",
				"excludes": self.texture_exclusions.clone(),
			},
			"entities": {
				"definitions": [ format!("{}.fgd", self.name) ],
				"defaultcolor": format!("{} {} {} {}", self.entity_default_color.x, self.entity_default_color.y, self.entity_default_color.z, self.entity_default_color.w),
				"scale": "$$scale$$", // Placeholder
				"setDefaultProperties": self.entity_set_default_properties,
			},
			"tags": {
				"brush": self.brush_tags.iter().map(|tag| tag.to_json("classname")).collect::<Vec<_>>(),
				"brushface": self.face_tags.iter().map(|tag| tag.to_json("material")).collect::<Vec<_>>()
			},
		};

		if let Some(icon) = &self.icon {
			fs::write(folder.join("Icon.png"), icon)?;
			json.insert("icon", "Icon.png").unwrap();
		}

		let insert_defaults = self.default_face_attributes.is_any_set();
		if insert_defaults || !self.surface_flags.is_empty() || !self.content_flags.is_empty() {
			let mut face_attributes = json::object! {
				"surfaceflags": self.surface_flags.as_slice(),
				"contentflags": self.content_flags.as_slice(),
			};

			if insert_defaults {
				face_attributes.insert("defaults", &self.default_face_attributes).unwrap();
			}

			json.insert("faceattribs", face_attributes).unwrap();
		}

		if let Some(bounds) = self.soft_map_bounds {
			json.insert("softMapBounds", bounds.fgd_to_string()).unwrap();
		}

		let mut buf = json.pretty(4);

		if let Some(expression) = &self.get_entity_scale_expression() {
			buf = buf.replace("\"$$scale$$\"", expression);
		}

		fs::write(folder.join("GameConfig.cfg"), buf)?;

		//////////////////////////////////////////////////////////////////////////////////
		//// FGD
		//////////////////////////////////////////////////////////////////////////////////

		fs::write(folder.join(format!("{}.fgd", self.name)), self.to_fgd())?;

		Ok(())
	}
}

#[test]
fn hook_stack() {
	let mut hook: Hook<dyn Fn() -> i32 + Send + Sync> = Hook(Arc::new(|| 2));
	assert_eq!(hook(), 2);
	hook.set(|prev| Arc::new(move || prev() + 1));
	assert_eq!(hook(), 3);
}

#[test]
fn coordinate_conversions() {
	let config = TrenchBroomConfig::default();

	let input = vec3(20.6, 1.72, 9.0);
	assert_eq!(config.from_bevy_space(config.to_bevy_space(input)), input);
}
