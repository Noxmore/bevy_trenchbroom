use crate::*;

/// Domain specific language for defining your TrenchBroom configuration's entity definitions. See readme.md/[root documentation](crate) for a usage example.
///
/// # Specification
///
/// Each definition starts like so (`<>` meaning required, and `[]` meaning optional):
/// ```ignore
/// /// [description]
/// <class type> <name>[( <setting>(<expression>).. )]
/// ```
///
/// <br>
///
/// There are 3 different class types:
/// - `Base`: doesn't appear in the editor, and is used like an abstract class in OOP languages
/// - `Point`: Appears in the entity menu, has a position, optionally with a model/iconsprite display
/// - `Solid`: Contains brushes, can be created by selecting the brushes you want to convert, right clicking, and selecting your entity from the `create brush entity` menu
///
/// It's the convention to make your entity names `snake_case`.
///
/// For the list of available settings, see documentation on [EntDefSettings].
///
/// After this, you have to add a block of properties, similar to how you define fields when creating a struct:
/// ```ignore
/// ... {
///     /// [<title> : [description]]
///     <name>: <type> [= <default>],
///     ..
/// }
/// ```
///
/// The main differences here are the lack of visibility specifiers, and the optional default.
///
/// You can also specify the property's title and description via documentation comment, the `" : "` string separating them.
///
/// The type here is also different, you can use just a regular type, as long as it implements [TrenchBroomValue], or you could set it to a custom type by using a string in place of the type. (e.g. using `"studio"` for a model)
///
/// You can also set it to a `choices` type by using this syntax: `[ <key> : <title>, .. ]` That tells the user for the value to be one of the `<key>`s, although its not guaranteed.
///
/// Then, optionally, you can define a spawner, a piece of code that runs when an entity is spawned with this definition or a subclass of this definition:
/// ```ignore
/// ... |world, entity, view| {
///     ...
/// } => {
///     // The closure arguments above give you exclusive access to the Bevy world,
///     // the Bevy entity this TrenchBroom entity is being spawned into,
///     // and a view into the the TrenchBroom entity (as well as the current TrenchBroomConfig).
///     // With this, you can get properties from the TrenchBroom entity with `view.get(key)`.
/// }
/// ```
/// It should be noted that [TrenchBroomConfig] also has a global spawner, that is called on every entity regardless of classname.
#[macro_export]
macro_rules! entity_definitions {
    {
        $($(#[$attr:meta])* $class_type:ident $classname:ident $(($($setting:ident($($setting_expr:tt)+))*))? $(: $($base:ty),+)? {
            $(
            $(#[$prop_attr:meta])*
            $prop_name:ident : $prop_type:tt $(= $default:expr)?
            ),* $(,)?
        } $(|$commands:ident, $entity:ident, $view:ident $(,)?| $spawner:expr)?)*
    } => {{
        use $crate::prelude::*;
        use bevy::reflect::Typed;

        $($(#[$attr])* #[allow(non_camel_case_types)] #[derive(bevy::reflect::Reflect)] struct $classname {
            $($(#[$prop_attr])* $prop_name: (),)*
        })*

        // Property autocomplete
        $($(#[allow(unused)] let $prop_name = ();)*)*

        // Base autocomplete helper
        #[allow(unused_must_use)] {
            $($($(std::any::type_name::<$base>();)+)?)*
        }

        indexmap::IndexMap::<String, EntityDefinition>::from([
            $((stringify!($classname).to_string(), EntityDefinition {
                class_type: EntDefClassType::$class_type,
                description: $classname::type_info().docs().map(str::to_string),

                // Base classes
                $(base: vec![$(stringify!($base).to_string()),+],)?

                // Settings
                $(settings: EntDefSettings {
                    $($setting: Some(stringify!($($setting_expr)+).to_string()),)*
                    ..Default::default()
                },)?

                // Properties
                properties: indexmap::IndexMap::<String, EntDefProperty>::from([
                    $((stringify!($prop_name).to_string(), {
                        let bevy::reflect::TypeInfo::Struct(struct_info) = $classname::type_info() else { unreachable!() };
                        let mut docs = struct_info.field(stringify!($prop_name)).unwrap().docs().unwrap_or_default().split(" : ").map(str::trim).map(str::to_string);
                        EntDefProperty {
                            ty: entity_definitions!(@PROPERTY_TYPE $prop_type),
                            title: docs.next(),
                            description: docs.next(),
                            $(default_value: Some(($default).tb_to_string_quoted()),)?
                            ..Default::default()
                        }
                    })),*
                ]),

                // Spawner
                $(spawner: Some(#[allow(unused)] |$commands, $entity, $view| {$spawner Ok(())}),)?

                ..Default::default()
            })),*
        ])
    }};

    (@PROPERTY_TYPE [$($choice:literal : $title:literal),+ $(,)?]) => {
        EntDefPropertyType::Choices(vec![$((($choice).tb_to_string_quoted(), $title.to_string())),+])
    };
    (@PROPERTY_TYPE $ty:ty) => {
        <$ty>::fgd_type()
    };
    (@PROPERTY_TYPE $value:literal) => {
        EntDefPropertyType::Value($value.to_string())
    };
}

/// A definition for an entity class type that will be both written out to the game's `fgd` file, and used to spawn the entity into the world once loaded.
#[derive(Debug, Clone, SmartDefault, Serialize, Deserialize)]
pub struct EntityDefinition {
    /// The type of entity this is, see documentation for [EntDefClassType] variants.
    pub class_type: EntDefClassType,

    /// A more detailed description of this entity class.
    pub description: Option<String>,

    /// Any base classes this entity might have.
    pub base: Vec<String>,

    pub settings: EntDefSettings,

    /// The properties specific to this definition, if you want properties that accounts for class hierarchy, use the `get_property` function.
    pub properties: IndexMap<String, EntDefProperty>,

    /// How this entity spawns itself into the Bevy world.
    #[serde(skip)]
    pub spawner: Option<EntitySpawner>,
}

impl EntityDefinition {
    /// Returns this definition in `FGD` format.
    pub fn to_fgd(&self, entity_name: &str, config: &TrenchBroomConfig) -> String {
        let mut out = String::new();

        out += &format!("@{:?}Class ", self.class_type);

        // Settings
        if !self.base.is_empty() {
            out += &format!("base({}) ", self.base.join(", "));
        }

        macro_rules! setting {($setting:ident) => {
            if let Some(value) = &self.settings.$setting {
                out += &format!("{}({}) ", stringify!($setting), value
                    .replace("$tb_scale$", &config.scale.to_string())
                    // Band-aid fix because stringify! doesn't account for user whitespace, hopefully this doesn't mess anything up
                    .replace('\n', "")
                    .replace("{ {", "{{")
                    .replace("} }", "}}")
                );
            }
        };}

        setting!(color);
        setting!(iconsprite);
        setting!(size);
        setting!(model);

        // Title
        out += &format!("= {}", entity_name);
        if let Some(description) = &self.description {
            out += &format!(" : \"{description}\"");
        }
        out += "\n[\n";

        // Properties
        for (property_name, property) in &self.properties {
            out += &format!(
                "\t{property_name}({}) : \"{}\" : {} : \"{}\"",
                match &property.ty {
                    EntDefPropertyType::Value(ty) => ty,
                    EntDefPropertyType::Choices(_) => "choices",
                },
                property.title.clone().unwrap_or(property_name.clone()),
                property.default_value.clone().unwrap_or_default(),
                property.description.clone().unwrap_or_default(),
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntDefSettings {
    /// A model that the entity shows up as in the editor. See the page on the [TrenchBroom docs](https://trenchbroom.github.io/manual/latest/#display-models-for-entities) for more info.
    pub model: Option<String>,
    pub color: Option<String>,
    /// An icon that the entity appears as in the editor. Takes a single value representing the path to the image to show.
    pub iconsprite: Option<String>,
    /// The size of the bounding box of the entity in the editor.
    pub size: Option<String>,
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
