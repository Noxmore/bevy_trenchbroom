use bevy::render::render_asset::RenderAssetUsages;

use crate::*;

// TODO look through here for things that should be able to be changed during gameplay.

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

    /// The default lightmap exposure for BSP loaded lightmaps.
    #[default(10000.)]
    pub default_lightmap_exposure: f32,
    #[default(500.)]
    pub default_irradiance_volume_intensity: f32,

    /// Whether to ignore map entity spawning errors for not having an entity definition for the map entity in question's classname. (Default: false)
    pub ignore_invalid_entity_definitions: bool,

    /// [TrenchBroomConfig::special_textures]
    #[builder(skip)]
    pub special_textures: Option<SpecialTexturesConfig>,

    /// How lightmaps atlas' are computed when loading BSP files.
    /// 
    /// It's worth noting that `wgpu` has a [texture size limit of 2048](https://github.com/gfx-rs/wgpu/discussions/2952), which can be expanded via [RenderPlugin](bevy::render::RenderPlugin) if needed.
    pub compute_lightmap_settings: ComputeLightmapSettings,

    pub entity_definitions: IndexMap<String, EntityDefinition>,

    /// Entity spawners that get run on every single entity (after the regular spawners), regardless of classname. (Default: [TrenchBroomConfig::default_global_spawner])
    #[default(vec![Self::default_global_spawner])]
    pub global_spawners: Vec<EntitySpawner>,

    /// Spawner that gets run after an entity spawns brushes, regardless of classname.
    pub global_brush_spawners: Vec<fn(&mut World, Entity, &mut BrushSpawnView)>,

    // TODO hook stack?
    /// Called to apply a material to spawned brush geometry. (Default: [TrenchBroomConfig::default_material_application_hook])
    #[default(Self::default_material_application_hook)]
    pub material_application_hook: fn(StandardMaterial, &BrushMeshView, &mut World, &BrushSpawnView),

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

    /// Adds an entity to `entity_definitions`. (deprecated, use `entity_definitions()` with the [entity_definitions!] macro instead)
    #[deprecated = "Use the `entity_definitions!` macro instead"]
    pub fn define_entity(mut self, id: impl Into<String>, definition: EntityDefinition) -> Self {
        self.entity_definitions.insert(id.into(), definition);
        self
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

    /// Adds transform via [MapEntityPropertiesView::get_transform], and names the entity based on the classname, and `targetname` if the property exists. (See documentation on [TrenchBroomConfig::global_spawner])
    /// 
    /// If the entity is a brush entity, no rotation is applied.
    pub fn default_global_spawner(
        world: &mut World,
        entity: Entity,
        view: EntitySpawnView,
    ) -> Result<(), MapEntitySpawnError> {
        let classname = view.map_entity.classname()?.s();

        let mut transform = view.get_transform();
        if view.server.config.get_definition(&classname)?.class_type == EntDefClassType::Solid {
            transform.rotation = Quat::IDENTITY;
        }
        
        world.entity_mut(entity).insert((
            Name::new(
                view.get::<String>("targetname")
                    .map(|name| format!("{classname} ({name})"))
                    .unwrap_or(classname),
            ),
            transform,
            GlobalTransform::default(),
        ));

        trenchbroom_gltf_rotation_fix(world, entity);

        Ok(())
    }

    /// Adds the [StandardMaterial] to the entity.
    pub fn default_material_application_hook(
        material: StandardMaterial,
        mesh_view: &BrushMeshView,
        world: &mut World,
        _view: &BrushSpawnView,
    ) {
        let handle = world.resource_mut::<Assets<StandardMaterial>>().add(material);
        world.entity_mut(mesh_view.entity).insert(handle);
    }

    /// Gets the default value for the specified entity definition's specified property accounting for entity class hierarchy.
    pub fn get_entity_property_default(&self, classname: &str, property: &str) -> Option<&String> {
        let definition = self.entity_definitions.get(classname)?;

        if let Some(prop) = definition.properties.get(property) {
            if let Some(default) = &prop.default_value {
                return Some(default);
            }
        }

        for base in &definition.base {
            if let Some(default) = self.get_entity_property_default(base, property) {
                return Some(default);
            }
        }

        None
    }

    /// Gets and entity definition from this config, or if none is found, returns [MapEntitySpawnError::DefinitionNotFound].
    pub fn get_definition(
        &self,
        classname: &str,
    ) -> Result<&EntityDefinition, MapEntitySpawnError> {
        self.entity_definitions.get(classname).ok_or_else(|| {
            MapEntitySpawnError::DefinitionNotFound {
                classname: classname.into(),
            }
        })
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
        //// ENTITY DEFINITIONS
        //////////////////////////////////////////////////////////////////////////////////

        fs::write(
            folder.join(format!("{}.fgd", self.name)),
            self.entity_definitions
                .iter()
                .map(|(name, def)| def.to_fgd(name, self))
                .join("\n\n"),
        )?;

        Ok(())
    }
}