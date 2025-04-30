use super::*;
use crate::*;
use bevy::{
	asset::RenderAssetUsages,
	render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bsp::*;
use lighting::{AnimatedLightingType, new_animated_lighting_output_image};
use qbsp::mesh::lightmap::ComputeLightmapAtlasError;

/// Writes lightmaps to target/lightmaps folder
const WRITE_DEBUG_FILES: bool = true;

pub struct BspLightmap {
	pub animated_lighting: Handle<AnimatedLighting>,
	pub uv_map: LightmapUvMap,
}

impl BspLightmap {
	pub fn compute(ctx: &mut BspLoadCtx) -> anyhow::Result<Option<Self>> {
		let config = &ctx.loader.tb_server.config;

		if config.no_bsp_lighting {
			return Ok(None);
		}

		match ctx
			.data
			.compute_lightmap_atlas(PerSlotLightmapPacker::new(config.compute_lightmap_settings))
		{
			Ok(atlas) => {
				let size = atlas.data.size();

				if WRITE_DEBUG_FILES {
					fs::create_dir("target/lightmaps").ok();
					for (i, image) in atlas.data.slots.iter().enumerate() {
						image.save_with_format(format!("target/lightmaps/{i}.png"), image::ImageFormat::Png).ok();
					}
					atlas
						.data
						.styles
						.save_with_format("target/lightmaps/styles.png", image::ImageFormat::Png)
						.ok();
				}

				let output = ctx.load_context.add_labeled_asset(
					"LightmapOutput".s(),
					new_animated_lighting_output_image(
						Extent3d {
							width: size.x,
							height: size.y,
							..default()
						},
						TextureDimension::D2,
					),
				);

				let mut i = 0;
				let input = atlas.data.slots.map(|image| {
					let handle = ctx.load_context.add_labeled_asset(
						format!("LightmapInput{i}"),
						Image::new(
							Extent3d {
								width: image.width(),
								height: image.height(),
								..default()
							},
							TextureDimension::D2,
							image.pixels().flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255]).collect(),
							// Without Srgb all the colors are washed out, so i'm guessing ericw-tools outputs sRGB, though i can't find it documented anywhere.
							TextureFormat::Rgba8UnormSrgb,
							config.bsp_textures_asset_usages,
						),
					);

					i += 1;
					handle
				});

				let styles = ctx.load_context.add_labeled_asset(
					"LightmapStyles".s(),
					Image::new(
						Extent3d {
							width: size.x,
							height: size.y,
							depth_or_array_layers: 1,
						},
						TextureDimension::D2,
						atlas.data.styles.into_vec(),
						TextureFormat::Rgba8Uint,
						RenderAssetUsages::RENDER_WORLD,
					),
				);

				let handle = ctx.load_context.add_labeled_asset(
					"LightmapAnimator".s(),
					AnimatedLighting {
						ty: AnimatedLightingType::Lightmap,
						output,
						input,
						styles,
					},
				);

				Ok(Some(Self {
					animated_lighting: handle,
					uv_map: atlas.uvs,
				}))
			}
			Err(ComputeLightmapAtlasError::NoLightmaps) => Ok(None),
			Err(err) => Err(anyhow::anyhow!(err)),
		}
	}
}
