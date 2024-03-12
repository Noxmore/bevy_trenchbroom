use crate::*;

#[derive(Component, Reflect, Debug, Clone, SmartDefault, Serialize, Deserialize)]
#[reflect(Component)]
#[serde(default)]
pub struct MaterialProperties {
    pub kind: MaterialKind,
    #[default(1.)]
    pub roughness: f32,
    #[default(0.)]
    pub metallic: f32,
    pub alpha_mode: MaterialAlphaMode,
    #[default(Color::BLACK)]
    pub emissive: Color,
    pub double_sided: bool,
}

lazy_static! {
    pub static ref MATERIAL_PROPERTIES_CACHE: Mutex<HashMap<PathBuf, MaterialProperties>> =
        default();
}

impl MaterialProperties {
    /// Loads material properties from the specified path, the first time this path is called it loads it from file, then caches it for later loads.
    pub fn load(path: impl AsRef<Path>) -> MaterialProperties {
        let path = path.as_ref();
        MATERIAL_PROPERTIES_CACHE
            .lock()
            .unwrap()
            .entry(path.to_owned())
            .or_insert_with(|| {
                ron::from_str(&fs::read_to_string(path).unwrap_or("()".into()))
                    .unwrap_or(MaterialProperties::default())
            })
            .clone()
    }
}

/// The kind of material a material is, this includes things like:
/// - What the material sounds like when walked-on/hit/scraped.
/// - How the material appears in the world.
#[derive(Reflect, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MaterialKind {
    /// A general solid material, with specified sounds.
    Solid(String),
    /// Doesn't draw in-game, but does have collision, even if a trimesh collider is used. Sounds are optional.
    CollisionOnly(Option<String>),
    /// Does not appear in-game, no collision, if a trimesh collider is used.
    Empty,
}

impl MaterialKind {
    pub fn sounds(&self) -> Option<&str> {
        match self {
            Self::Solid(sounds) => Some(sounds),
            Self::CollisionOnly(sounds) => sounds.as_ref().map(String::as_str),
            Self::Empty => None,
        }
    }

    pub fn should_render(&self) -> bool {
        match self {
            Self::Solid(_) => true,
            Self::CollisionOnly(_) => false,
            Self::Empty => false,
        }
    }

    pub fn should_collide(&self) -> bool {
        match self {
            Self::Solid(_) => true,
            Self::CollisionOnly(_) => true,
            Self::Empty => false,
        }
    }
}

impl Default for MaterialKind {
    fn default() -> Self {
        Self::Solid("rock".into())
    }
}

/// A serializable copy of [AlphaMode] for [MaterialProperties]
#[derive(Reflect, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum MaterialAlphaMode {
    #[default]
    Opaque,
    Cutout,
    Blend,
    Premultiplied,
    Add,
    Multiply,
}

impl From<MaterialAlphaMode> for AlphaMode {
    fn from(value: MaterialAlphaMode) -> Self {
        match value {
            MaterialAlphaMode::Opaque => Self::Opaque,
            MaterialAlphaMode::Cutout => Self::Mask(0.5),
            MaterialAlphaMode::Blend => Self::Blend,
            MaterialAlphaMode::Premultiplied => Self::Premultiplied,
            MaterialAlphaMode::Add => Self::Add,
            MaterialAlphaMode::Multiply => Self::Multiply,
        }
    }
}
