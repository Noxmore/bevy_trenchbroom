use bevy::{asset::LoadContext, render::render_asset::RenderAssetUsages};
use class::{ErasedQuakeClass, QuakeClassType, GLOBAL_CLASS_REGISTRY};
use fgd::FgdType;
use geometry::{GeometryProviderFn, GeometryProviderView};
use qmap::{QuakeMap, QuakeMapEntity};
use bsp::{util::IrradianceVolumeMultipliers, GENERIC_MATERIAL_PREFIX};
use special_textures::load_special_texture;
use util::{trenchbroom_gltf_rotation_fix, ZUpToYUp};

use crate::*;

// TODO look through here for things that should be able to be changed during gameplay.

pub type LoadEmbeddedTextureFn = dyn Fn(EmbeddedTextureLoadView) -> Handle<GenericMaterial> + Send + Sync;
pub type LoadLooseTextureFn = dyn Fn(TextureLoadView) -> Handle<GenericMaterial> + Send + Sync;
pub type SpawnFn = dyn Fn(&TrenchBroomConfig, &QuakeMapEntity, &mut EntityWorldMut) -> anyhow::Result<()> + Send + Sync;

/// The main configuration structure of bevy_trenchbroom.
#[derive(Debug, Clone, SmartDefault, DefaultBuilder)]
pub struct TrenchBroomConfig {
    /// The format version of the TrenchBroom config file, you almost certainly should not change this.
    #[default(8)]
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
    /// Supported map file formats, it is recommended to leave this at its default (Valve)
    #[default(vec![MapFileFormat::Valve])]
    pub file_formats: Vec<MapFileFormat>,
    /// The format for asset packages. If you are just using loose files, this probably doesn't matter to you, and you can leave it defaulted.
    #[builder(skip)] // bevy_trenchbroom currently *only* supports loose files
    package_format: AssetPackageFormat,

    /// The root directory to look for textures in the [assets_path](Self::assets_path). (Default: "textures")
    #[default("textures".into())]
    #[builder(into)]
    pub texture_root: PathBuf,
    /// The extension of your texture files. (Default: "png")
    #[default("png".into())]
    #[builder(into)]
    pub texture_extension: String,
    /// The palette file path and data used for WADs. The path roots from your assets folder.
    /// 
    /// For the default quake palette (what you most likely want to use), there is a [free download on the Quake wiki](https://quakewiki.org/wiki/File:quake_palette.zip),
    /// and a copy distributed by this library in the form of [QUAKE_PALETTE].
    /// 
    /// If TrenchBroom can't find this palette file, all WAD textures will be black.
    /// (Default: ("palette.lmp", &QUAKE_PALETTE))
    #[builder(into)]
    #[default(("palette.lmp".into(), &QUAKE_PALETTE))] // TODO
    pub texture_pallette: (PathBuf, &'static Palette),
    /// Patterns to match to exclude certain texture files from showing up in-editor. (Default: [TrenchBroomConfig::default_texture_exclusions]).
    #[builder(into)]
    #[default(Self::default_texture_exclusions())]
    pub texture_exclusions: Vec<String>,

    /// The default color for entities in RGBA. (Default: 0.6 0.6 0.6 1.0)
    #[default(vec4(0.6, 0.6, 0.6, 1.0))]
    #[builder(into)]
    pub entity_default_color: Vec4,
    /// An expression to evaluate how big entities' models are. Any instances of the string `$tb_scale$` will be replaced with the scale configured in this struct.
    #[builder(into)]
    pub entity_scale_expression: Option<String>,
    /// Whether to set property defaults into an entity on creation, or leave them to use the default value that is defined in entity definitions. It is not recommended to use this.
    pub entity_set_default_properties: bool,

    /// Tags to apply to brushes.
    #[builder(into)]
    pub brush_tags: Vec<TrenchBroomTag>,
    /// Tags to apply to brush faces. The default is defined by [TrenchBroomConfig::empty_face_tag], and all it does is make `__TB_empty` transparent.
    #[default(vec![Self::empty_face_tag()])]
    #[builder(into)]
    pub face_tags: Vec<TrenchBroomTag>,

    /// The optional bounding box enclosing the map to draw in the 2d viewports.
    ///
    /// The two values are the bounding box min, and max respectively.
    ///
    /// NOTE: This bounding box is in TrenchBroom space (Z up).
    pub soft_map_bounds: Option<[Vec3; 2]>,

    /// If `Some`, sets the lightmap exposure on any `StandardMaterial` loaded. (Default: Some(10,000))
    #[default(Some(10_000.))]
    #[builder(into)]
    pub lightmap_exposure: Option<f32>,
    #[default(500.)]
    pub default_irradiance_volume_intensity: f32,
    /// Multipliers to the colors of BSP loaded irradiance volumes depending on direction.
    /// 
    /// This is because light-grid-loaded irradiance volumes don't have any directionality.
    /// This fakes it, making objects within look a little nicer.
    /// 
    /// (Default: IrradianceVolumeMultipliers::SLIGHT_SHADOW)
    #[default(IrradianceVolumeMultipliers::SLIGHT_SHADOW)]
    pub irradiance_volume_multipliers: IrradianceVolumeMultipliers,

    // TODO rename
    /// Whether to ignore map entity spawning errors for not having an entity definition for the map entity in question's classname. (Default: false)
    pub ignore_invalid_entity_definitions: bool,

    /// An optional configuration for supporting [Quake special textures](https://quakewiki.org/wiki/Textures),
    /// such as animated textures, skies, liquids, and invisible textures like clip and skip.
    #[builder(into)]
    pub special_textures: Option<SpecialTexturesConfig>,

    #[builder(skip)]
    #[default(Hook(Arc::new(Self::default_load_embedded_texture)))]
    pub load_embedded_texture: Hook<LoadEmbeddedTextureFn>,
    #[builder(skip)]
    #[default(Hook(Arc::new(Self::default_load_loose_texture)))]
    pub load_loose_texture: Hook<LoadLooseTextureFn>,

    /// How lightmaps atlas' are computed when loading BSP files.
    /// 
    /// It's worth noting that `wgpu` has a [texture size limit of 2048](https://github.com/gfx-rs/wgpu/discussions/2952), which can be expanded via [RenderPlugin](bevy::render::RenderPlugin) if needed.
    /// 
    /// NOTE: `special_lighting_color` is set to gray (`75`) by default instead of white (`255`), because otherwise all textures with it look way too bright and washed out, not sure why.
    #[default(ComputeLightmapSettings { special_lighting_color: [75; 3], ..default() })]
    pub compute_lightmap_settings: ComputeLightmapSettings,

    entity_classes: HashMap<String, ErasedQuakeClass>,

    /// Entity spawners that get run on every single entity (after the regular spawners), regardless of classname. (Default: [TrenchBroomConfig::default_global_spawner])
    #[builder(skip)]
    #[default(Hook(Arc::new(Self::default_global_spawner)))]
    pub global_spawner: Hook<SpawnFn>,

    /// Geometry provider run after all others for all entities regardless of classname. (Default: [TrenchBroomConfig::default_global_geometry_provider])
    #[builder(skip)]
    #[default(Hook(Arc::new(Self::default_global_geometry_provider)))]
    pub global_geometry_provider: Hook<GeometryProviderFn>,

    /// Whether brush meshes are kept around in memory after they're sent to the GPU. Default: [RenderAssetUsages::RENDER_WORLD] (not kept around)
    #[default(RenderAssetUsages::RENDER_WORLD)]
    pub brush_mesh_asset_usages: RenderAssetUsages,

    /// Whether BSP loaded textures and lightmaps are kept around in memory after they're sent to the GPU. Default: [RenderAssetUsages::RENDER_WORLD] (not kept around)
    #[default(RenderAssetUsages::RENDER_WORLD)]
    pub bsp_textures_asset_usages: RenderAssetUsages,
}

impl TrenchBroomConfig {
    /// Creates a new TrenchBroom config. It is recommended to use this over [TrenchBroomConfig::default]
    pub fn new(name: impl Into<String>) -> Self {
        Self::default().name(name)
    }

    /// Excludes "\*_normal", "\*_mr" (Metallic and roughness), "\*_emissive", and "\*_depth".
    pub fn default_texture_exclusions() -> Vec<String> {
        vec![
            "*_normal".into(),
            "*_mr".into(),
            "*_emissive".into(),
            "*_depth".into(),
        ]
    }

    /// (See documentation on [TrenchBroomConfig::face_tags])
    pub fn empty_face_tag() -> TrenchBroomTag {
        TrenchBroomTag::new("empty", "__TB_empty")
            .attributes([TrenchBroomTagAttribute::Transparent])
    }

    /// Names the entity based on the classname, and `targetname` if the property exists. (See documentation on [TrenchBroomConfig::global_spawner])
    /// 
    /// If the entity is a brush entity, rotation is reset.
    pub fn default_global_spawner(
        config: &TrenchBroomConfig,
        src_entity: &QuakeMapEntity,
        entity: &mut EntityWorldMut,
    ) -> anyhow::Result<()> {
        let classname = src_entity.classname()?.s();

        if let Some(mut transform) = entity.get_mut::<Transform>() {
            if config.get_class(&classname).map(|class| class.info.ty) == Some(QuakeClassType::Solid) {
                transform.rotation = Quat::IDENTITY;
            }
        }
        
        entity.insert(Name::new(
            src_entity.get::<String>("targetname")
                .map(|name| format!("{classname} ({name})"))
                .unwrap_or(classname),
        ));

        trenchbroom_gltf_rotation_fix(entity);

        Ok(())
    }

    /// Adds [Visibility] and [Transform] components if they aren't in the entity, as it is needed to clear up warnings for child meshes.
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
    pub fn default_load_embedded_texture(mut view: EmbeddedTextureLoadView) -> Handle<GenericMaterial> {
        let mut material = StandardMaterial {
            base_color_texture: Some(view.image_handle.clone()),
            perceptual_roughness: 1.,
            ..default()
        };

        if let Some(alpha_mode) = view.alpha_mode {
            material.alpha_mode = alpha_mode;
        }

        let generic_material = match load_special_texture(&mut view, &mut material) {
            Some(v) => v,
            None => GenericMaterial {
                handle: view.add_material(material).into(),
                properties: default(),
            }
        };
        
        view.parent_view.load_context.add_labeled_asset(format!("{GENERIC_MATERIAL_PREFIX}{}", view.name), generic_material)
    }

    pub fn load_loose_texture_fn(mut self, provider: impl FnOnce(Arc<LoadLooseTextureFn>) -> Arc<LoadLooseTextureFn>) -> Self {
        self.load_loose_texture.set(provider);
        self
    }
    pub fn default_load_loose_texture(view: TextureLoadView) -> Handle<GenericMaterial> {
        view.load_context.load(view.tb_config.texture_root.join(format!("{}.material", view.name)))
    }

    /// Retrieves the entity class of `classname` from this config. If none is found and the `auto_register` feature is enabled, it'll try to find it in [GLOBAL_CLASS_REGISTRY].
    pub fn get_class(&self, classname: &str) -> Option<&ErasedQuakeClass> {
        #[cfg(not(feature = "auto_register"))] {
            self.entity_classes.get(classname)
        }

        #[cfg(feature = "auto_register")] {
            self.entity_classes.get(classname).or_else(|| GLOBAL_CLASS_REGISTRY.get(classname).copied())
        }
    }

    /// A list of all registered classes. If the `auto_register` feature is enabled, also includes [GLOBAL_CLASS_REGISTRY].
    pub fn class_iter(&self) -> impl Iterator<Item = &ErasedQuakeClass> {
        #[cfg(not(feature = "auto_register"))] {
            self.entity_classes.values()
        }

        #[cfg(feature = "auto_register")] {
            self.entity_classes.values().chain(GLOBAL_CLASS_REGISTRY.values().copied())
        }
    }

    /// Converts from a z-up coordinate space to a y-up coordinate space, and scales everything down by this config's scale.
    pub fn to_bevy_space(&self, vec: Vec3) -> Vec3 {
        vec.z_up_to_y_up() / self.scale
    }

    /// Converts from a z-up coordinate space to a y-up coordinate space, and scales everything down by this config's scale.
    pub fn to_bevy_space_f64(&self, vec: DVec3) -> DVec3 {
        vec.z_up_to_y_up() / self.scale as f64
    }
}

/// Various inputs available when loading textures.
pub struct TextureLoadView<'a, 'b> {
    pub name: &'a str,
    pub tb_config: &'a TrenchBroomConfig,
    pub load_context: &'a mut LoadContext<'b>,
    pub map: &'a QuakeMap,
    /// `Some` if it is determined that a specific alpha mode should be used for a material, such as in some embedded textures.
    pub alpha_mode: Option<AlphaMode>,
    /// If the map contains embedded textures, this will be a map of texture names to image handles.
    /// This is useful for things like animated textures.
    pub embedded_textures: Option<&'a HashMap<&'a str, (Image, Handle<Image>)>>,
}
impl TextureLoadView<'_, '_> {
    /// Shorthand for adding a material asset with the correct label.
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

// TODO I wish this was bit more encapsulated and ergonomic
/// Wrapper for storing a stack of dynamic functions. Use [Hook::push] to push a new function onto the stack.
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

/* /// Macro that produces a Hook type with less boilerplate.
#[macro_export]
macro_rules! Hook {(($($arg:ty),* $(,)?) $(-> $return:ty)?) => {
    Hook<dyn Fn($($arg),*) $(-> $return)? + Send + Sync>
};} */

/* impl<F: ?Sized + HookClone<F = F>> Clone for Hook<F> {
    fn clone(&self) -> Self {
        F::clone_hook(self)
    }
} */

/* pub trait DynClone<T: ?Sized> {
    fn dyn_clone(&self) -> Box<T>;
}
impl<T: Clone> DynClone<T> for T {
    fn dyn_clone(&self) -> Box<T> {
        Box::new(self.clone())
    }
} */

/* pub trait HookClone {
    type F: ?Sized;
    fn clone_hook(&self) -> Hook<Self::F>;
}
impl<F: Clone> HookClone for F {
    type F = F;
    fn clone_hook(&self) -> Hook<Self::F> {
        Hook(Box::new(self.clone()))
    }
} */

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
            Self::Other {
                extension,
                format: _,
            } => extension,
        }
    }

    /// The format id used by this package format.
    pub fn format(&self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::IdPack => "idpak",
            Self::DkPack => "dkpak",
            Self::Other {
                extension: _,
                format,
            } => format,
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

/// Tag for applying attributes to certain brushes/faces, for example, making a `trigger` texture transparent.
#[derive(Debug, Clone, Default, DefaultBuilder)]
pub struct TrenchBroomTag {
    /// Name of the tag.
    #[builder(skip)]
    pub name: String,
    /// The attributes applied to the brushes/faces the tag targets.
    #[builder(into)]
    pub attributes: Vec<TrenchBroomTagAttribute>,
    /// The pattern to match for, if this is a brush tag, it will match against the `classname`, if it is a face tag, it will match against the texture.
    #[builder(skip)]
    pub pattern: String,
    /// Only used if this is a brush tag. When this tag is applied by the use of its keyboard shortcut, then the selected brushes will receive this texture if it is specified.
    #[builder(into)]
    pub texture: Option<String>,
}

impl TrenchBroomTag {
    /// Creates a new tag.
    ///
    /// The name is a simple name to identify the tag. The pattern is a pattern to match for allowing wildcards.
    /// If this is a brush tag, it will match against the `classname`, if it is a face tag, it will match against the texture.
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

        if let Some(texture) = &self.texture {
            json.insert("texture", texture.clone()).unwrap();
        }

        json
    }
}

/// Attribute for [TrenchBroomTag], currently the only option is `Transparent`.
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
            "textures": {
                "root": self.texture_root.s(),
                // .D is required for WADs to work
                "extensions": [".D", self.texture_extension.clone()],
                "palette": self.texture_pallette.0.s(),
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
                "brushface": self.face_tags.iter().map(|tag| tag.to_json("texture")).collect::<Vec<_>>()
            }
        };
        

        if let Some(icon) = &self.icon {
            fs::write(folder.join("Icon.png"), icon)?;
            json.insert("icon", "Icon.png").unwrap();
        }

        // if let Some(palette) = &self.texture_pallette {
        //     json["textures"].insert("palette", palette.clone()).unwrap();
        // }

        if let Some(bounds) = self.soft_map_bounds {
            json.insert("softMapBounds", bounds.fgd_to_string()).unwrap();
        }

        let mut buf = json.pretty(4);

        if let Some(expression) = &self.entity_scale_expression {
            buf = buf.replace(
                "\"$$scale$$\"",
                &expression.replace("$tb_scale$", &self.scale.to_string()),
            );
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
    let mut hook: Hook<dyn Fn() -> i32> = Hook(Arc::new(|| 2));
    assert_eq!(hook(), 2);
    hook.set(|prev| Arc::new(move || prev() + 1));
    assert_eq!(hook(), 3);
}