use crate::*;

/// The complete TrenchBroom configuration, it is recommended to set this in the plugin, where it will be put into [CURRENT_CONFIG], and to not change it afterwards.
#[derive(Resource, Debug, Clone, SmartDefault, DefaultBuilder)]
pub struct TrenchBroomConfig {
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

	/// Optional icon for the TrenchBroom UI. Contains the data of a PNG file.
	pub icon: Option<Vec<u8>>,
	/// Supported map file formats, it is recommended to leave this at its default (Valve)
	#[default(vec![MapFileFormat::Valve])]
	pub file_formats: Vec<MapFileFormat>,
	/// The format for asset packages. If you are just using loose files, this probably doesn't matter to you, and you can leave it defaulted.
	pub package_format: AssetPackageFormat,

	/// The root directory to look for textures. (Default: "textures")
	#[default("textures".into())]
	#[builder(into)]
	pub texture_root: String,
	/// The extension of your texture files. (Default: "png")
	#[default("png".into())]
	#[builder(into)]
	pub texture_extension: String,
	/// An optional pallette file to use for textures.
	#[builder(into)]
	pub texture_pallette: Option<String>,
	/// Patterns to match to exclude certain texture files from showing up in-editor. (Default: [TrenchBroomConfig::default_texture_exclusions]).
	#[builder(into)]
	pub texture_exclusions: Vec<String>,

	/// The default color for entities in RGBA. (Default: 0.6 0.6 0.6 1.0)
	#[default(vec4(0.6, 0.6, 0.6, 1.0))]
	#[builder(into)]
	pub entity_default_color: Vec4,
	/// An expression to evaluate how big entities' models are. Any instances of the string `$tb_scale$` will be replaced with the scale configured in this struct. (Default: "$tb_scale$")
	#[default(Some("$tb_scale$".into()))]
	#[builder(into)]
	pub entity_scale_expression: Option<String>,
	/// Whether to set property defaults into an entity on creation, or leave them to use the default value that is defined in entity definitions. It is not recommended to use this.
	pub entity_set_default_properties: bool,

	/// Tags to apply to brushes.
	#[builder(into)]
	pub brush_tags: Vec<TrenchBroomTag>,
	/// Tags to apply to brush faces. The default is defined by [TrenchBroomConfig::default_face_tags], and all it does is make `__TB_empty` transparent.
	#[default(Self::default_face_tags())]
	#[builder(into)]
	pub face_tags: Vec<TrenchBroomTag>,

	/// The optional bounding box enclosing the map to draw in the 2d viewports.
	/// This doesn't use [Aabb] because at the time of writing it does not implement `Serialize`.
	///
	/// The two values are the bounding box min, and max respectively.
	///
	/// NOTE: This bounding box is in TrenchBroom space (Z up).
	pub soft_map_bounds: Option<[Vec3; 2]>,
	
	#[builder(skip)]
	pub entity_definitions: IndexMap<String, EntityDefinition>,

	/// Entity Inserter that gets run on every single entity (after the regular inserters), regardless of classname. (Default: [TrenchBroomConfig::default_global_inserter])
	#[default(Some(Self::default_global_inserter))]
	pub global_inserter: Option<EntityInserter>,
}

impl TrenchBroomConfig {
	/// Creates a new TrenchBroom config. It is recommended to use this over [TrenchBroomConfig::default]
	pub fn new(name: impl Into<String>) -> Self {
		Self::default().name(name)
	}
	
	/// Adds an entity to `entity_definitions`.
	pub fn define_entity(mut self, id: impl Into<String>, definition: EntityDefinition) -> Self {
		self.entity_definitions.insert(id.into(), definition);
		self
	}

	/// Excludes "\*_normal", "\*_mr" (Metallic and roughness), "\*_emissive", and "\*_depth".
	pub fn default_texture_exclusions() -> Vec<String> {
		vec!["*_normal".into(), "*_mr".into(), "*_emissive".into(), "*_depth".into()]
	}
	
	/// (See documentation on [TrenchBroomConfig::face_tags])
	pub fn default_face_tags() -> Vec<TrenchBroomTag> {
		vec![TrenchBroomTag {
			name: "empty".into(),
			attributes: vec![TrenchBroomTagAttribute::Transparent],
			pattern: "__TB_empty".into(),
			..default()
		}]
	}

	/// Adds transform via [MapEntityPropertiesView::get_transform], the [MapEntity] itself, and names the entity based on the classname, and `targetname` if the property exists. (See documentation on [TrenchBroomConfig::global_inserter])
	pub fn default_global_inserter(commands: &mut Commands, entity: Entity, view: EntityInsertionView) -> Result<(), MapEntityInsertionError> {
		let classname = view.properties.entity.classname()?.to_string();
		commands.entity(entity).insert((
			Name::new(view.properties.require::<String>("targetname").map(|name| format!("{classname} ({name})")).unwrap_or(classname)),
			view.properties.get_transform(),
			view.properties.entity.clone(),
		));
		Ok(())
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

	pub fn validate(&self) -> Result<(), String> {
		Ok(())
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
	Other { extension: &'static str, format: &'static str },
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

/// Tag for applying attributes to certain brushes/faces, for example, making a `trigger` texture transparent.
#[derive(Debug, Clone, Default)]
pub struct TrenchBroomTag {
	/// Name of the tag.
	pub name: String,
	/// The attributes applied to the brushes/faces the tag targets.
	pub attributes: Vec<TrenchBroomTagAttribute>,
	/// The pattern to match for, if this is a brush tag, it will match against the `classname`, if it is a face tag, it will match against the texture.
	pub pattern: String,
	/// Only used if this is a brush tag. When this tag is applied by the use of its keyboard shortcut, then the selected brushes will receive this texture if it is specified.
	pub texture: Option<String>,
}

impl TrenchBroomTag {
	pub fn to_json(&self) -> json::JsonValue {
		let mut json = json::object! {
			"name": self.name.clone(),
			"attribs": self.attributes.iter().copied().map(TrenchBroomTagAttribute::config_str).collect::<Vec<_>>(),
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
	///
	/// NOTE: If you are using [CURRENT_CONFIG], make sure to apply this AFTER your app gets built, otherwise you will be writing [TrenchBroomConfig]'s default value.
	pub fn write_folder(&self, folder: impl AsRef<Path>) -> io::Result<()> {
		if self.name.is_empty() {
			return Err(io::Error::new(io::ErrorKind::InvalidData, "Please set a name for your TrenchBroom config. \
			If you have, make sure you call `write_into` after the app is built. (e.g. In a startup system)"));
		}

		let folder = folder.as_ref();

		//////////////////////////////////////////////////////////////////////////////////
		//// GAME CONFIGURATION && ICON
		//////////////////////////////////////////////////////////////////////////////////

		// The game config file is basically json, so we can get 99% of the way there by just creating a json object.
		let mut json = json::object! {
			"version": 6,
			"name": self.name.clone(),
			"fileformats": self.file_formats.iter().map(|format| json::object! { "format": format.config_str() }).collect::<Vec<_>>(),
			"filesystem": {
				"searchpath": self.assets_path.to_string_lossy().to_string(),
				"packageformat": { "extension": self.package_format.extension(), "format": self.package_format.format() }
			},
			"textures": {
				"package": { "type": "directory", "root": self.texture_root.clone() },
				"format": { "extension": self.texture_extension.clone(), "format": "image" },
				"attribute": "_tb_textures",
				"excludes": self.texture_exclusions.clone(),
			},
			"entities": {
				"definitions": [ format!("{}.fgd", self.name) ],
				"defaultcolor": format!("{} {} {} {}", self.entity_default_color.x, self.entity_default_color.y, self.entity_default_color.z, self.entity_default_color.w),
				"scale": "$$scale$$", // Placeholder
				"setDefaultProperties": self.entity_set_default_properties,
			},
			"tags": {
				"brush": self.brush_tags.iter().map(TrenchBroomTag::to_json).collect::<Vec<_>>(),
				"brushface": self.face_tags.iter().map(TrenchBroomTag::to_json).collect::<Vec<_>>()
			}
		};

		if let Some(icon) = &self.icon {
			fs::write(folder.join("Icon.png"), icon)?;
			json.insert("icon", "Icon.png").unwrap();
		}

		if let Some(palette) = &self.texture_pallette {
			json["textures"].insert("palette", palette.clone()).unwrap();
		}

		if let Some(bounds) = self.soft_map_bounds {
			json.insert("softMapBounds", bounds.tb_to_string()).unwrap();
		}

		let mut buf = json.pretty(4);

		if let Some(expression) = &self.entity_scale_expression {
			buf = buf.replace("\"$$scale$$\"", expression);
		}

		fs::write(folder.join("GameConfig.cfg"), buf)?;

		//////////////////////////////////////////////////////////////////////////////////
		//// ENTITY DEFINITIONS
		//////////////////////////////////////////////////////////////////////////////////
		
		fs::write(folder.join(format!("{}.fgd", self.name)),
			self.entity_definitions.iter().map(|(name, def)| def.to_fgd(name)).join("\n\n")
		)?;

		Ok(())
	}
}

/// Mirrors [TrenchBroomConfig::scale] to [TRENCHBROOM_SCALE] if it changes.
pub fn mirror_trenchbroom_scale(config: Res<TrenchBroomConfig>) {
	if !config.is_changed() { return }
	*TRENCHBROOM_SCALE.write().unwrap() = config.scale;
}


#[test]
fn write_config() -> io::Result<()> {
	TrenchBroomConfig::new("boop") // <- The name of your game
		// Here you can customize the resulting game configuration with a builder-like syntax
		.entity_scale_expression("scale")
		// ...
		
		
		// You can define entity definitions here, these are written to your game's FGD file

		// It's highly recommended to make the first defined entity your `worldspawn`
		.define_entity("worldspawn", EntityDefinition::new_solid()
			.description("World Entity")
			
			.property("skybox", EntDefProperty::string().title("Skybox").description("Path to Skybox"))
		)

		.define_entity("angles", EntityDefinition::new_base()
			.property("angles", EntDefProperty::vec3().title("Pitch Yaw Roll (Y Z X)").default_value(Vec3::ZERO))
		)

		.define_entity("player_spawnpoint", EntityDefinition::new_point()
			.description("Bap")
			.base(["angles"])
			
			.property("testing", EntDefProperty::boolean().title("Testing Boolean").default_value(true).description("Awesome description"))

			.inserter(|commands, entity, view| {
				Ok(())
			})
		)
		
		.write_folder("/d/_tmp/config")
}