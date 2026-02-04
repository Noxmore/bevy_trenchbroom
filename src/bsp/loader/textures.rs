use bsp::*;
use loader::BspLoadCtx;
use qbsp::data::texture::EmbeddedTextureName;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

use crate::*;

pub struct EmbeddedTextures {
	pub images: HashMap<EmbeddedTextureName, (Image, Handle<Image>)>,
	pub textures: HashMap<EmbeddedTextureName, BspEmbeddedTexture>,
}

fn is_cutout_texture(name: &EmbeddedTextureName) -> bool {
	name.as_bytes()[0] == b'{'
}

impl EmbeddedTextures {
	pub async fn setup<'a, 'lc>(ctx: &mut BspLoadCtx<'a, 'lc>) -> anyhow::Result<Self> {
		let config = &ctx.loader.tb_server.config;

		// Have to clone `texture_pallette` for the borrow checker. Can't figure out why.
		let palette = match ctx.load_context.read_asset_bytes(config.texture_pallette.clone()).await.ok() {
			Some(bytes) => Palette::parse(&bytes).map_err(|err| anyhow!("Parsing palette file {:?}: {err}", config.texture_pallette))?,
			None => QUAKE_PALETTE.clone(),
		};

		let images: HashMap<EmbeddedTextureName, (Image, Handle<Image>)> = ctx
			.data
			.textures
			.iter()
			.flatten()
			.filter(|texture| texture.data.full.is_some())
			.map(|texture| {
				let Some(data) = &texture.data.full else { unreachable!() };
				let name = texture.header.name;

				let is_cutout_texture = is_cutout_texture(&name);

				let palette = texture.data.palette.as_ref().unwrap_or(&palette);

				let image = Image::new(
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

				let image_handle = ctx.load_context.get_label_handle(format!("{TEXTURE_PREFIX}{name}"));

				(name, (image, image_handle))
			})
			.collect();

		let mut textures: HashMap<EmbeddedTextureName, BspEmbeddedTexture> = HashMap::with_capacity_and_hasher(images.len(), default());

		for (name, (image, image_handle)) in &images {
			#[cfg(feature = "client")]
			let is_cutout_texture = is_cutout_texture(name);

			let material = (config.load_embedded_texture)(EmbeddedTextureLoadView {
				parent_view: TextureLoadView {
					name: name.as_str(),
					tb_server: &ctx.loader.tb_server,
					load_context: ctx.load_context,
					asset_server: ctx.asset_server,
					entities: ctx.entities,
					#[cfg(feature = "client")]
					alpha_mode: is_cutout_texture.then_some(AlphaMode::Mask(0.5)),
					embedded_textures: Some(&images),
				},

				bsp_format: ctx.data.parse_ctx.format,
				image_handle,
				image,
			})
			.await;

			textures.insert(
				*name,
				BspEmbeddedTexture {
					image: image_handle.clone(),
					material,
				},
			);
		}

		Ok(Self { images, textures })
	}

	/// Loads the placeholder images, and returns the embedded textures.
	pub fn finalize(self, ctx: &mut BspLoadCtx) -> HashMap<EmbeddedTextureName, BspEmbeddedTexture> {
		for (name, (image, _)) in self.images {
			ctx.load_context.add_labeled_asset(format!("{TEXTURE_PREFIX}{name}"), image);
		}

		self.textures
	}
}
