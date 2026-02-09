use super::*;

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

	pub(crate) fn to_json(&self, match_type: &str) -> jzon::JsonValue {
		let mut json = jzon::object! {
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
	Used { name: Cow<'static, str>, description: Option<Cow<'static, str>> },
}
impl BitFlag {
	#[inline] // We use &str to avoid `None` description causing an unspecified generic
	pub fn new(name: &'static str, description: Option<&'static str>) -> Self {
		Self::Used { name: name.into(), description: description.map(Into::into) }
	}
}
impl From<BitFlag> for jzon::JsonValue {
	fn from(value: BitFlag) -> Self {
		match value {
			BitFlag::Unused => jzon::object! { "unused": true },
			BitFlag::Used { name, description } => {
				let mut json = jzon::object! {
					"name": name.as_ref(),
				};

				if let Some(description) = description {
					json.insert("description", description.as_ref()).unwrap();
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
impl From<&DefaultFaceAttributes> for jzon::JsonValue {
	fn from(value: &DefaultFaceAttributes) -> Self {
		let mut json = jzon::JsonValue::new_object();

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
