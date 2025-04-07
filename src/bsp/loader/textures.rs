use bevy::asset::{io::AssetReaderError, ReadAssetBytesError};
use bsp::*;
use loader::BspLoadCtx;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};

use crate::*;

pub struct EmbeddedTextures<'d> {
	pub images: HashMap<&'d str, (Image, Handle<Image>)>,
	pub textures: HashMap<String, BspEmbeddedTexture>,
}

impl<'d> EmbeddedTextures<'d> {
	pub async fn setup<'a: 'd, 'lc>(ctx: &mut BspLoadCtx<'a, 'lc>) -> anyhow::Result<Self> {
		let config = &ctx.loader.tb_server.config;

		let palette = match ctx.load_context.read_asset_bytes(config.texture_pallette.as_path()).await {
			Ok(bytes) => Palette::parse(&bytes).map_err(|err| anyhow!("Parsing palette file {:?}: {err}", config.texture_pallette))?,
			Err(ReadAssetBytesError::AssetReaderError(AssetReaderError::NotFound(_))) => QUAKE_PALETTE.clone(),
			Err(err) => return Err(err.into()),
		};

		let images: HashMap<&str, (Image, Handle<Image>)> = ctx
			.data
			.textures
			.iter()
			.flatten()
			.filter(|texture| texture.data.is_some())
			.map(|texture| {
				let Some(data) = &texture.data else { unreachable!() };
				let name = texture.header.name.as_str();

				let is_cutout_texture = name.starts_with('{');

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

		let mut textures: HashMap<String, BspEmbeddedTexture> = HashMap::with_capacity(images.len());

		for (name, (image, image_handle)) in &images {
			#[cfg(feature = "client")]
			let is_cutout_texture = name.starts_with('{');

			let material = (config.load_embedded_texture)(EmbeddedTextureLoadView {
				parent_view: TextureLoadView {
					name,
					tb_config: config,
					load_context: ctx.load_context,
					asset_server: ctx.asset_server,
					entities: ctx.entities,
					#[cfg(feature = "client")]
					alpha_mode: is_cutout_texture.then_some(AlphaMode::Mask(0.5)),
					embedded_textures: Some(&images),
				},

				image_handle,
				image,
			})
			.await;

			textures.insert(
				name.s(),
				BspEmbeddedTexture {
					image: image_handle.clone(),
					material,
				},
			);
		}

		Ok(Self { images, textures })
	}

	/// Loads the placeholder images, and returns the embedded textures.
	pub fn finalize(self, ctx: &mut BspLoadCtx) -> HashMap<String, BspEmbeddedTexture> {
		for (name, (image, _)) in self.images {
			ctx.load_context.add_labeled_asset(format!("{TEXTURE_PREFIX}{name}"), image);
		}

		self.textures
	}
}
