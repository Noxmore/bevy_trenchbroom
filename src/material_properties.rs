use std::{borrow::Cow, future::Future};

use bevy::{asset::{AssetLoader, AsyncReadExt}, utils::ConditionalSendFuture};

use crate::*;

// TODO should this be in the config?

/// The extension that material properties files should have
pub static MATERIAL_PROPERTIES_EXTENSION: &str = "toml";

/// Information about an expected field from [MaterialProperties]. Built-in properties are stored in the [MaterialProperties] namespace, such as [MaterialProperties::RENDER].
pub struct MaterialProperty<T: Deserialize<'static>> {
    pub key: Cow<'static, str>,
    pub default_value: T,
}

impl<T: Deserialize<'static>> MaterialProperty<T> {
    pub const fn new(key: &'static str, default_value: T) -> Self {
        Self {
            key: Cow::Borrowed(key),
            default_value,
        }
    }
}

/// Stores toml information about a material, made for brushes.
///
/// # Examples
///
/// ```
/// use bevy_trenchbroom::prelude::*;
///
/// // You should load MaterialProperties via your app's asset server, this is just for demonstration purposes.
/// let mat_properties = MaterialPropertiesLoader.load_sync(&std::fs::read_to_string("assets/textures/test.toml").unwrap()).unwrap();
///
/// assert_eq!(mat_properties.get(MaterialProperties::RENDER), true);
/// assert_eq!(mat_properties.get(MaterialProperties::COLLIDE), false);
/// assert_eq!(mat_properties.get(MaterialProperties::ROUGHNESS), 0.25);
/// ```
#[derive(Asset, TypePath, Debug, Clone, Default)]
pub struct MaterialProperties {
    pub properties: HashMap<String, toml::Value>,
}

impl MaterialProperties {
    /// Whether the surface should render in the world.
    pub const RENDER: MaterialProperty<bool> = MaterialProperty::new("render", true);
    /// Whether the surface should have general collision.
    pub const COLLIDE: MaterialProperty<bool> = MaterialProperty::new("collide", true);
    /// How rough the surface should be. Used for [StandardMaterial::perceptual_roughness]. The only difference being the default of this is `1.0`.
    pub const ROUGHNESS: MaterialProperty<f32> = MaterialProperty::new("roughness", 1.);
    /// How metallic the surface should be. Used for [StandardMaterial::metallic].
    pub const METALLIC: MaterialProperty<f32> = MaterialProperty::new("metallic", 0.);
    /// How a material's base color alpha channel is used for transparency. See [MaterialPropertiesAlphaMode docs](MaterialPropertiesAlphaMode).
    pub const ALPHA_MODE: MaterialProperty<MaterialPropertiesAlphaMode> =
        MaterialProperty::new("alpha_mode", MaterialPropertiesAlphaMode::Opaque);
    /// The amount of emissive light given off from the surface. Used for [StandardMaterial::emissive].
    pub const EMISSIVE: MaterialProperty<LinearRgba> = MaterialProperty::new("emissive", LinearRgba::BLACK);
    /// Whether to cull back faces.
    pub const DOUBLE_SIDED: MaterialProperty<bool> = MaterialProperty::new("double_sided", false);

    /// Gets a property from these properties, if it isn't defined, uses the supplied [MaterialProperty]'s default.
    pub fn get<T: Deserialize<'static>>(&self, property: MaterialProperty<T>) -> T {
        // I feel like turning the value into a string just to deserialize it again isn't the best way of doing this, but i don't know of another
        self.properties
            .get(property.key.as_ref())
            .map(|value| T::deserialize(toml::de::ValueDeserializer::new(&value.to_string())).ok())
            .flatten()
            .unwrap_or(property.default_value)
    }
}

#[derive(Default)]
pub struct MaterialPropertiesLoader;
/// I have to put this in its own static variable for some reason.
static MATERIAL_PROPERTIES_LOADER_EXTENSIONS: &[&str] = &[MATERIAL_PROPERTIES_EXTENSION];
impl AssetLoader for MaterialPropertiesLoader {
    type Asset = MaterialProperties;
    type Settings = ();
    type Error = io::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> impl ConditionalSendFuture + Future<Output = Result<<Self as AssetLoader>::Asset, <Self as AssetLoader>::Error>> {
        Box::pin(async move {
            let mut buf = String::new();
            reader.read_to_string(&mut buf).await?;
            self.load_sync(&buf)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
        })
    }

    fn extensions(&self) -> &[&str] {
        MATERIAL_PROPERTIES_LOADER_EXTENSIONS
    }
}
impl MaterialPropertiesLoader {
    /// Loads a [MaterialProperties] synchronously, you should probably use regular asset loading instead of this.
    pub fn load_sync(&self, input: &str) -> Result<MaterialProperties, toml::de::Error> {
        toml::from_str::<HashMap<String, toml::Value>>(input)
            .map(|properties| MaterialProperties { properties })
    }
}

/// Caches textures used on brushes to [StandardMaterial] handles.
pub static BRUSH_TEXTURE_TO_MATERIALS_CACHE: Lazy<
    Mutex<HashMap<String, Handle<StandardMaterial>>>,
> = Lazy::new(default);

/// A serializable copy of [AlphaMode] for [MaterialProperties]
///
/// # Examples
/// ```toml
/// alpha_mode = { type = "Cutout" } // Shorthand for { type = "Mask", threshold = 0.7 }
/// ```
#[derive(Reflect, Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "type", content = "threshold")]
pub enum MaterialPropertiesAlphaMode {
    #[default]
    /// [AlphaMode::Opaque]
    Opaque,
    /// [AlphaMode::Mask]
    Mask(f32),
    /// Shortcut for `AlphaMode::Mask(0.5)`
    Cutout,
    /// [AlphaMode::Blend]
    Blend,
    /// [AlphaMode::Premultiplied]
    Premultiplied,
    /// [AlphaMode::Add]
    Add,
    /// [AlphaMode::Multiply]
    Multiply,
}

impl From<MaterialPropertiesAlphaMode> for AlphaMode {
    fn from(value: MaterialPropertiesAlphaMode) -> Self {
        match value {
            MaterialPropertiesAlphaMode::Opaque => Self::Opaque,
            MaterialPropertiesAlphaMode::Mask(v) => Self::Mask(v),
            MaterialPropertiesAlphaMode::Cutout => Self::Mask(0.5),
            MaterialPropertiesAlphaMode::Blend => Self::Blend,
            MaterialPropertiesAlphaMode::Premultiplied => Self::Premultiplied,
            MaterialPropertiesAlphaMode::Add => Self::Add,
            MaterialPropertiesAlphaMode::Multiply => Self::Multiply,
        }
    }
}

#[derive(Reflect, Debug)]
pub struct EmptyMaterialType;
impl MaterialType for EmptyMaterialType {
    fn should_collide(&self) -> bool {
        false
    }
    fn should_render(&self) -> bool {
        false
    }
}

pub trait MaterialType: Reflect + std::fmt::Debug {
    fn should_render(&self) -> bool;
    fn should_collide(&self) -> bool;
}
