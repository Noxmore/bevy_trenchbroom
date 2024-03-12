use crate::*;

/// A definition for an entity class type that will be both written out to the game's `fgd` file, and used to insert the entity into the world once loaded.
#[derive(Debug, Clone, Default, DefaultBuilder, Serialize, Deserialize)]
pub struct EntityDefinition {
	/// The type of entity this is, see documentation for [EntDefClassType] variants.
	#[builder(skip)]
	pub class_type: EntDefClassType,

	/// A more detailed description of this entity.
	#[builder(into)]
	pub description: Option<String>,

	/// Any base classes this entity might have.
	#[builder(skip)]
	pub base: Vec<String>,
	/// An optional model to apply to this entity.
	#[builder(into)]
	pub model: Option<EntDefModel>,
	#[builder(into)]
	pub color: EntDefAttribute<[u8; 3]>,
	/// The path to a sprite to display this entity as. If an iconsprite is set, the default model size will be set to `1`.
	#[builder(into)]
	pub iconsprite: EntDefAttribute<String>,
	/// The size of this entity's bounding box in TrenchBroom units. Defaults to `16x16x16`.
	#[builder(into)]
	pub size: EntDefAttribute<[Vec3; 2]>,

	/// The properties specific to this definition, if you want properties that accounts for class hierarchy, use the `get_property` function.
	#[builder(skip)]
	pub properties: IndexMap<String, EntDefProperty>,

	/// How this entity inserts itself into the Bevy world.
	#[serde(skip)]
	#[builder(skip)]
	pub inserter: Option<EntityInserter>,
}

impl EntityDefinition {
	/// Creates a new `@BaseClass` [EntityDefinition].
	pub fn new_base() -> Self {
		Self { class_type: EntDefClassType::Base, ..default() }
	}
	/// Creates a new `@PointClass` [EntityDefinition].
	pub fn new_point() -> Self {
		Self { class_type: EntDefClassType::Point, ..default() }
	}
	/// Creates a new `@SolidClass` [EntityDefinition].
	pub fn new_solid() -> Self {
		Self { class_type: EntDefClassType::Solid, ..default() }
	}
	
	
	/// Adds a property to this definition.
	pub fn property(mut self, id: impl Into<String>, property: EntDefProperty) -> Self {
		self.properties.insert(id.into(), property);
		self
	}
	
	/// Any base classes this entity might have.
	pub fn base<S: Into<String>>(mut self, base: impl IntoIterator<Item=S>) -> Self {
		self.base = base.into_iter().map(Into::into).collect();
		self
	}

	/// How this entity inserts itself into the Bevy world.
	pub fn inserter(mut self, inserter: EntityInserter) -> Self {
		self.inserter = Some(inserter);
		self
	}
	
	/// Returns this definition in `FGD` format.
	pub fn to_fgd(&self, entity_name: &str, config: &TrenchBroomConfig) -> String {
		let mut out = String::new();
		
		out += &format!("@{:?}Class ", self.class_type);

		// Attributes
		if !self.base.is_empty() {
			out += &format!("base({}) ", self.base.join(", "));
		}

		if let Some(value) = self.color.to_fgd() {
			out += &format!("color({}) ", value.trim_matches('"'));
		}

		if let Some(value) = self.iconsprite.to_fgd() {
			out += &format!("iconsprite({value}) ");
		}

		match &self.size {
			EntDefAttribute::Undefined => {},
			EntDefAttribute::Expr(prop) => out += &format!("size({prop}) "),
			EntDefAttribute::Set(value) => out += &format!("size({}) ", Aabb::from_min_max(value[0], value[1]).tb_to_string()),
		}

		// (Model)
		if let Some(model) = &self.model {
			let mut model_props: Vec<String> = default();

			if let Some(path) = model.path.to_fgd() {
				model_props.push(format!(r#""path": {path}"#));
			}
			if let Some(frame) = model.frame.to_fgd() {
				model_props.push(format!(r#""frame": {frame}"#));
			}
			if let Some(skin) = model.skin.to_fgd() {
				model_props.push(format!(r#""skin": {skin}"#));
			}
			if let Some(scale) = model.scale.to_fgd() {
				model_props.push(format!(r#""scale": {}"#, scale.replace("$tb_scale$", &config.scale.to_string())));
			}

			out += &format!("model({{ {} }}) ", model_props.join(", "));
		}

		// Title
		out += &format!("= {}", entity_name);
		if let Some(description) = &self.description {
			out += &format!(" : \"{description}\"");
		}
		out += "\n[\n";
		
		// Properties
		for (property_name, property) in &self.properties {
			out += &format!("\t{property_name}({}) : \"{}\" : {} : \"{}\"",
				match &property.ty { EntDefPropertyType::Value(ty) => ty, EntDefPropertyType::Choices(_) => "choices" },
				property.title.clone().unwrap_or(property_name.clone()),
				property.default_value.clone().unwrap_or_default(),
				property.description.clone().unwrap_or_default()
			);

			if let EntDefPropertyType::Choices(choices) = &property.ty {
				out += " = \n\t[\n";
				for (key, title) in choices {
					out += &format!("\t\t{key} : \"{title}\"\n");
				}
				out += "\t]";
			}

			out += "\n";
		}
		
		out += "]";
		
		out
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum EntDefClassType {
	/// Cannot be spawned in TrenchBroom, works like a base class in any object-oriented language.
	#[default]
	Base,
	/// An entity that revolves around a single point.
	Point,
	/// An entity that contains brushes.
	Solid,
}


/// A property for an entity definition. the property type (`ty`) doesn't have a set of different options, it more just tells users what kind of data you are expecting. 
#[derive(Debug, Clone, Default, DefaultBuilder, Serialize, Deserialize)]
pub struct EntDefProperty {
	#[builder(skip)]
	pub ty: EntDefPropertyType,
	#[builder(into)]
	pub title: Option<String>,
	#[builder(into)]
	pub description: Option<String>,
	#[builder(skip)]
	pub default_value: Option<String>,
}

impl EntDefProperty {
	/// Creates a new non-choices [EntDefProperty] with the specified property type.
	///
	/// If you are creating a property for a common type, you should use its built-in function. (For example if you want a boolean, use [EntDefProperty::boolean])
	pub fn value(ty: impl Into<String>) -> Self {
		Self { ty: EntDefPropertyType::Value(ty.into()), ..default() }
	}

	/// Creates a new multi-choice [EntDefProperty]. A value that must be one of a pre-defined set of values.
	/// # Examples
	/// ```
	/// use bevy_trenchbroom::prelude::*;
	/// EntDefProperty::choices([(0, "First awesome thing"), (1, "Second awesome thing"), (2, "EVEN MORE AWESOME")]);
	/// ```
	pub fn choices<Key: TrenchBroomValue, Title: Into<String>>(choices: impl IntoIterator<Item=(Key, Title)>) -> Self {
		Self { ty: EntDefPropertyType::Choices(choices.into_iter().map(|(key, title)| (key.tb_to_string_quoted(), title.into())).collect()), ..default() }
	}
	
	pub fn string() -> Self { Self::value("string") }
	/// The Entity IO target name of an entity.
	pub fn target_source() -> Self { Self::value("target_source") }
	/// The Entity IO target of an entity.
	pub fn target_destination() -> Self { Self::value("target_destination") }
	/// Floating point number.
	pub fn float() -> Self { Self::value("float") }
	/// Integer number.
	pub fn integer() -> Self { Self::value("integer") }
	/// A color made up of 3 floats going from `0.0` to `1.0`.
	pub fn color1() -> Self { Self::value("color1") }
	/// Boolean, can be true or false.
	pub fn boolean() -> Self { Self::choices([(true, "true"), (false, "false")]) }
	/// 3 floating point numbers.
	pub fn vec3() -> Self { Self::value("vector") }
	/// A model, currently doesn't have any special menu in TrenchBroom.
	pub fn studio() -> Self { Self::value("studio") }
	/// A sound file, currently doesn't have any special menu in TrenchBroom.
	pub fn sound() -> Self { Self::value("sound") }
	
	
	/// Sets the default value for this property.
	pub fn default_value<T: TrenchBroomValue>(mut self, value: T) -> Self {
		self.default_value = Some(value.tb_to_string_quoted());
		self
	}
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntDefPropertyType {
	Value(String),
	Choices(Vec<(String, String)>),
}

impl Default for EntDefPropertyType {
	fn default() -> Self {
		Self::Value("string".into())
	}
}

/// Shorthand for `EntDefAttribute::Expr(expr.into())`
pub fn expr_attr<T: TrenchBroomValue>(expr: impl Into<String>) -> EntDefAttribute<T> {
	EntDefAttribute::Expr(expr.into())
}
/// Shorthand for `EntDefAttribute::Set(value)`
pub fn set_attr<T: TrenchBroomValue>(value: T) -> EntDefAttribute<T> {
	EntDefAttribute::Set(value)
}

/// An attribute about an entity definition, can be not set, set to a property of that entity, or set to a specific value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum EntDefAttribute<T: TrenchBroomValue> {
	#[default]
	Undefined,
	/// Will use the result of the specified expression for this attribute. You can find the documentation for expressions [here](https://trenchbroom.github.io/manual/latest/#expression_language).
	Expr(String),
	/// A set value for this attribute, cannot be changed in-editor.
	Set(T),
}

impl<T: TrenchBroomValue> EntDefAttribute<T> {
	/// If this attribute is [EntDefAttribute::Set], it will call `mapper` on the value, returning [EntDefAttribute::Set] of the new value, otherwise it will just return self.
	pub fn map<O: TrenchBroomValue>(self, mapper: impl FnOnce(T) -> O) -> EntDefAttribute<O> {
		match self {
			Self::Undefined => EntDefAttribute::Undefined,
			Self::Expr(p) => EntDefAttribute::Expr(p),
			Self::Set(value) => EntDefAttribute::Set(mapper(value)),
		}
	}
}

impl<T: TrenchBroomValue> EntDefAttribute<T> {
	pub fn to_fgd(&self) -> Option<String> {
		match self {
			Self::Undefined => None,
			Self::Expr(property) => Some(property.clone()),
			Self::Set(value) => Some(value.tb_to_string_quoted()),
		}
	}
}

impl<T: TrenchBroomValue> From<T> for EntDefAttribute<T> {
	fn from(value: T) -> Self {
		Self::Set(value)
	}
}

impl From<EntDefAttribute<&str>> for EntDefAttribute<String> {
	fn from(value: EntDefAttribute<&str>) -> Self {
		value.map(str::to_string)
	}
}


#[derive(Debug, Clone, SmartDefault, DefaultBuilder, Serialize, Deserialize)]
pub struct EntDefModel {
	#[builder(skip)] pub path: EntDefAttribute<String>,
	#[builder(into)] pub frame: EntDefAttribute<usize>,
	#[builder(into)] pub skin: EntDefAttribute<usize>,
	/// The scale of the model. Any instances of the string `$tb_scale$` in an expression will be replaced with the scale configured in the [TrenchBroomConfig] (Default: `$tb_scale$`)
	#[default(expr_attr("$tb_scale$"))]
	#[builder(into)] pub scale: EntDefAttribute<f32>,
}

impl EntDefModel {
	pub fn new_set_path(path: impl Into<String>) -> Self {
		Self { path: EntDefAttribute::Set(path.into()), ..default() }
	}
	
	pub fn new_expr_path(property: impl Into<String>) -> Self {
		Self { path: EntDefAttribute::Expr(property.into()), ..default() }
	}
}