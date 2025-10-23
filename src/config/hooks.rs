#[cfg(feature = "bsp")]
use crate::bsp::MaterialProperties;

use super::*;

pub type LoadEmbeddedTextureFn = dyn for<'a, 'b> Fn(EmbeddedTextureLoadView<'a, 'b>) -> BoxedFuture<'a, Handle<GenericMaterial>> + Send + Sync;
pub type LoadLooseTextureFn = dyn for<'a, 'b> Fn(TextureLoadView<'a, 'b>) -> BoxedFuture<'a, Handle<GenericMaterial>> + Send + Sync;
pub type SpawnFn = dyn Fn(&mut QuakeClassSpawnView) -> anyhow::Result<()> + Send + Sync;
pub type SpawnFnOnce = dyn FnOnce(&mut QuakeClassSpawnView) -> anyhow::Result<()> + Send + Sync;

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

/// Various inputs available when loading textures.
pub struct TextureLoadView<'a, 'b> {
	pub name: &'a str,
	pub tb_config: &'a TrenchBroomConfig,
	pub load_context: &'a mut LoadContext<'b>,
	/// Because [`LoadContext`] doesn't expose its [`AssetServer`], this does it for you, allowing you do do things you couldn't with just the load context.
	pub asset_server: &'a AssetServer,
	pub entities: &'a QuakeMapEntities,
	/// `Some` if it is determined that a specific alpha mode should be used for a material, such as in some embedded textures.
	#[cfg(feature = "client")]
	pub alpha_mode: Option<AlphaMode>,
	/// If the map contains embedded textures, this will be a map of texture names to image handles.
	/// This is useful for things like animated textures.
	pub embedded_textures: Option<&'a HashMap<&'a str, (Image, Handle<Image>)>>,
}
impl TextureLoadView<'_, '_> {
	/// Shorthand for adding a material asset with the correct label.
	#[cfg(feature = "client")]
	pub fn add_material<M: Material>(&mut self, material: M) -> Handle<M> {
		self.load_context.add_labeled_asset(format!("Material_{}", self.name), material)
	}
}

#[derive(Deref, DerefMut)]
pub struct EmbeddedTextureLoadView<'a, 'b> {
	#[deref]
	pub parent_view: TextureLoadView<'a, 'b>,

	#[cfg(feature = "bsp")]
	pub material_properties: &'a dyn MaterialProperties,

	/// The handle of the image of this embedded texture.
	pub image_handle: &'a Handle<Image>,
	/// The actual image data behind the texture.
	pub image: &'a Image,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hook_stack() {
		let mut hook: Hook<dyn Fn() -> i32 + Send + Sync> = Hook(Arc::new(|| 2));
		assert_eq!(hook(), 2);
		hook.set(|prev| Arc::new(move || prev() + 1));
		assert_eq!(hook(), 3);
	}
}
