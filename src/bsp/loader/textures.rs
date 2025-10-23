use std::hash::{Hash, Hasher};

use bsp::*;
use loader::BspLoadCtx;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

use crate::*;

// For some reason, `<() as PartialReflect>::reflect_hash` returns `None` even though `(): Hash`
#[derive(Reflect, Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[reflect(Hash)]
#[reflect(PartialEq)]
pub struct NoMaterialProperties;

pub trait MaterialProperties: PartialReflect {
	fn write_material(&self, material: &mut StandardMaterial) -> anyhow::Result<()>;
}

pub struct MaterialId<T: ?Sized = dyn MaterialProperties> {
	name: String,
	props: T,
}

impl MaterialProperties for NoMaterialProperties {
	fn write_material(&self, _material: &mut StandardMaterial) -> anyhow::Result<()> {
		Ok(())
	}
}

impl<T> MaterialId<T>
where
	T: PartialReflect,
{
	pub fn name(&self) -> String {
		format!(
			"{}${}",
			self.name,
			self.props
				.reflect_hash()
				.expect("To be used as a material property, a type must implement `Hash` and be annotated with `#[reflect(Hash)]`")
		)
	}
}

impl PartialEq for Box<MaterialId> {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name
			&& self
				.props
				.reflect_partial_eq(&other.props)
				.expect("To be used as a material property, a type must implement `PartialEq` and be annotated with `#[reflect(PartialEq)]`")
	}
}

impl Eq for Box<MaterialId> {}

impl Hash for Box<MaterialId> {
	fn hash<H: Hasher>(&self, hasher: &mut H) {
		self.name.hash(&mut *hasher);
		self.props
			.reflect_hash()
			.expect("To be used as a material property, a type must implement `Hash`")
			.hash(&mut *hasher);
	}
}

pub struct EmbeddedTextures<'d> {
	pub images: HashMap<&'d str, (Image, Handle<Image>)>,
	pub materials: HashMap<Box<MaterialId>, BspEmbeddedTexture>,
}

impl<'d> EmbeddedTextures<'d> {
	pub async fn setup<'a: 'd, 'lc>(ctx: &mut BspLoadCtx<'a, 'lc>) -> anyhow::Result<Self> {
		let config = &ctx.loader.tb_server.config;

		// Have to clone `texture_pallette` for the borrow checker. Can't figure out why.
		let palette = match ctx.load_context.read_asset_bytes(config.texture_pallette.clone()).await.ok() {
			Some(bytes) => Palette::parse(&bytes).map_err(|err| anyhow!("Parsing palette file {:?}: {err}", config.texture_pallette))?,
			None => QUAKE_PALETTE.clone(),
		};

		let images: HashMap<&str, (Image, Handle<Image>)> = ctx
			.data
			.textures
			.iter()
			.flatten()
			.filter(|texture| texture.data.full.is_some())
			.map(|texture| {
				let is_cutout_texture = texture.header.name.as_str().starts_with('{');

				let Some(data) = &texture.data.full else { unreachable!() };
				let name = texture.header.name.as_str();

				let palette = texture.data.palette.as_ref().unwrap_or(&palette);

				let mut image = Image::new(
					Extent3d {
						width: texture.header.width,
						height: texture.header.height,
						..default()
					},
					TextureDimension::D2,
					data.iter()
						.copied()
						.flat_map(|pixel| {
							if config.embedded_texture_cutouts && is_cutout_texture && pixel == 255 {
								[0; 4]
							} else {
								let [r, g, b] = palette.colors[pixel as usize];
								[r, g, b, 255]
							}
						})
						.collect(),
					TextureFormat::Rgba8UnormSrgb,
					config.bsp_textures_asset_usages,
				);
				image.sampler = config.texture_sampler.clone();

				let image_handle = ctx.load_context.get_label_handle(format!("{TEXTURE_PREFIX}{name}"));

				(texture.header.name.as_str(), (image, image_handle))
			})
			.collect();

		Ok(Self {
			images,
			materials: default(),
		})
	}

	pub async fn material<S, MatProps>(&mut self, ctx: &mut BspLoadCtx<'_, '_>, name: S, mat_props: MatProps) -> Option<Handle<GenericMaterial>>
	where
		S: AsRef<str>,
		MatProps: MaterialProperties,
	{
		let name = name.as_ref();
		let material_id: Box<MaterialId> = Box::new(MaterialId {
			name: name.to_string(),
			props: mat_props,
		});

		if let Some(mat) = self.materials.get(&material_id) {
			Some(mat.material.clone())
		} else {
			let is_cutout_texture = name.starts_with('{');

			let (image, image_handle) = self.images.get(name)?;

			let config = &ctx.loader.tb_server.config;

			let material = (config.load_embedded_texture)(EmbeddedTextureLoadView {
				parent_view: TextureLoadView {
					name,
					tb_config: config,
					load_context: ctx.load_context,
					asset_server: ctx.asset_server,
					entities: ctx.entities,
					#[cfg(feature = "client")]
					alpha_mode: is_cutout_texture.then_some(AlphaMode::Mask(0.5)),
					embedded_textures: Some(&self.images),
				},

				image_handle,
				image,

				material_properties: &material_id.props,
			})
			.await;

			self.materials.insert(
				material_id,
				BspEmbeddedTexture {
					image: image_handle.clone(),
					material: material.clone(),
				},
			);

			Some(material)
		}
	}

	/// Loads the placeholder images, and returns the embedded textures.
	pub fn finalize(self, ctx: &mut BspLoadCtx) -> HashMap<String, BspEmbeddedTexture> {
		for (name, (image, _)) in self.images {
			ctx.load_context.add_labeled_asset(format!("{TEXTURE_PREFIX}{name}"), image);
		}

		self.materials.into_iter().map(|(k, v)| (k.name, v)).collect()
	}
}
