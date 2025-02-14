use super::*;
use crate::*;
use bevy::{
	asset::RenderAssetUsages,
	image::ImageSampler,
	render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
};
use bsp::*;
use ndshape::{RuntimeShape, Shape};

pub fn load_irradiance_volume(ctx: &mut BspLoadCtx, world: &mut World) -> anyhow::Result<Option<Handle<AnimatedLighting>>> {
	let config = &ctx.loader.tb_server.config;

	// Calculate irradiance volumes for light grids.
	// Right now we just have one big irradiance volume for the entire map, this means the volume has to be less than 682 (2048/3 (z axis is 3x)) cells in size.
	Ok(if let Some(light_grid) = ctx.data.bspx.parse_light_grid_octree(&ctx.data.parse_ctx) {
		let mut light_grid = light_grid?;
		light_grid.mins = config.to_bevy_space(light_grid.mins.to_array().into()).to_array().into();
		// We add 1 to the size because the volume has to be offset by half a step to line up, and as such sometimes doesn't fill the full space
		light_grid.size = light_grid.size.xzy() + 1;
		light_grid.step = config.to_bevy_space(light_grid.step.to_array().into()).to_array().into();

		let mut input_builders: [Option<IrradianceVolumeBuilder>; 4] = [(); 4].map(|_| None);

		let new_builder = || IrradianceVolumeBuilder::new(light_grid.size.to_array(), [0, 0, 0, 255], config.irradiance_volume_multipliers);

		let mut style_map_builder = IrradianceVolumeBuilder::new(light_grid.size.to_array(), [0; 4], IrradianceVolumeMultipliers::IDENTITY);

		for mut leaf in light_grid.leafs {
			leaf.mins = leaf.mins.xzy();
			let size = leaf.size().xzy();

			for x in 0..size.x {
				for y in 0..size.y {
					for z in 0..size.z {
						let LightGridCell::Filled(samples) = leaf.get_cell(x, z, y) else { continue };
						let (dst_x, dst_y, dst_z) = (x + leaf.mins.x, y + leaf.mins.y, z + leaf.mins.z);
						let mut style_map: [u8; 4] = [0; 4];

						for (slot_idx, sample) in samples.into_iter().enumerate() {
							if slot_idx >= 4 {
								error!(
									"Light grid cell at {} has more than 4 samples! Data past sample 4 will be thrown away!",
									leaf.mins + uvec3(x, y, z)
								);
								break;
							}

							// #[allow(clippy::needless_range_loop)]
							// for i in 0..3 {
							// 	color[i] = color[i].saturating_add(sample.color[i]);
							// }
							let [r, g, b] = sample.color;

							// print!("{} ", sample.style.0);
							input_builders[slot_idx]
								.get_or_insert_with(new_builder)
								.put_all([dst_x, dst_y, dst_z], [r, g, b, 255]);
							style_map[slot_idx] = sample.style.0;
						}

						style_map_builder.put_all([dst_x, dst_y, dst_z], style_map);
					}
				}
			}
		}

		// This is pretty much instructed by FTE docs
		// builder.flood_non_filled();

		// let mut image = builder.build();
		let full_size = IrradianceVolumeBuilder::full_size(light_grid.size);
		let mut output_image = Image::new_fill(
			Extent3d {
				width: full_size.x,
				height: full_size.y,
				depth_or_array_layers: full_size.z,
			},
			TextureDimension::D3,
			// Bright color -- easy to spot errors
			[0.0_f32, 1., 0., 1.].map(|f| f.to_ne_bytes()).as_flattened(),
			LIGHTMAP_OUTPUT_TEXTURE_FORMAT,
			RenderAssetUsages::RENDER_WORLD,
		);
		output_image.texture_descriptor.usage |= TextureUsages::STORAGE_BINDING;
		output_image.sampler = ImageSampler::linear();

		// let image_handle = load_context.add_labeled_asset("IrradianceVolume".s(), image);

		let mut slot_idx = 0;
		let input = input_builders.map(|builder| {
			let mut image = builder.map(IrradianceVolumeBuilder::build).unwrap_or_else(|| {
				Image::new_fill(
					Extent3d {
						width: 1,
						height: 1,
						depth_or_array_layers: 1,
					},
					TextureDimension::D3,
					&[0; 4],
					TextureFormat::Rgba8UnormSrgb,
					RenderAssetUsages::RENDER_WORLD,
				)
			});
			image.sampler = ImageSampler::linear();

			let handle = ctx.load_context.add_labeled_asset(format!("IrradianceVolumeSlot{slot_idx}"), image);
			slot_idx += 1;
			handle
		});

		let output = ctx.load_context.add_labeled_asset("IrradianceVolume".s(), output_image);

		let mut style_map_image = style_map_builder.build();
		style_map_image.texture_descriptor.format = TextureFormat::Rgba8Uint;
		style_map_image.asset_usage = RenderAssetUsages::all(); // TODO: keep this in main world for image depth work-around

		let styles = ctx.load_context.add_labeled_asset("IrradianceVolumeStyleMap".s(), style_map_image);

		let animated_lighting_handle = ctx.load_context.add_labeled_asset(
			"IrradianceVolumeAnimator".s(),
			AnimatedLighting {
				ty: AnimatedLightingType::IrradianceVolume,
				output,
				input,
				styles,
			},
		);

		let mins: Vec3 = light_grid.mins.to_array().into();
		let scale: Vec3 = (light_grid.size.as_vec3() * light_grid.step).to_array().into();

		world.spawn((
			Name::new("Light Grid Irradiance Volume"),
			LightProbe,
			AnimatedLightingHandle(animated_lighting_handle.clone()),
			Transform {
				translation: mins + scale / 2. - Vec3::from_array(light_grid.step.to_array()) / 2.,
				scale,
				..default()
			},
		));

		Some(animated_lighting_handle)
	} else {
		None
	})
}

#[derive(Debug, Clone, Copy)]
pub struct IrradianceVolumeMultipliers {
	pub x: [f32; 3],
	pub y: [f32; 3],
	pub z: [f32; 3],
	pub neg_x: [f32; 3],
	pub neg_y: [f32; 3],
	pub neg_z: [f32; 3],
}
impl IrradianceVolumeMultipliers {
	pub const IDENTITY: Self = Self {
		x: [1.; 3],
		y: [1.; 3],
		z: [1.; 3],
		neg_x: [1.; 3],
		neg_y: [1.; 3],
		neg_z: [1.; 3],
	};

	pub const SLIGHT_SHADOW: Self = Self {
		x: [0.9; 3],
		y: [0.7; 3],
		z: [1.; 3],
		neg_x: [1.2; 3],
		neg_y: [1.4; 3],
		neg_z: [1.; 3],
	};
}
impl Default for IrradianceVolumeMultipliers {
	fn default() -> Self {
		Self::IDENTITY
	}
}

/// Little helper API to create irradiance volumes for BSPs.
struct IrradianceVolumeBuilder {
	size: UVec3,
	full_shape: RuntimeShape<u32, 3>,
	data: Vec<[u8; 4]>,
	filled: Vec<bool>,
	multipliers: IrradianceVolumeMultipliers,
}
impl IrradianceVolumeBuilder {
	pub fn new(size: impl Into<UVec3>, default_color: [u8; 4], multipliers: IrradianceVolumeMultipliers) -> Self {
		let size: UVec3 = size.into();
		let full_size = Self::full_size(size);
		let shape = RuntimeShape::<u32, 3>::new(full_size.to_array());
		let vec_size = shape.usize();
		Self {
			size,
			full_shape: shape,
			data: vec![default_color; vec_size],
			filled: vec![false; vec_size],
			multipliers,
		}
	}

	pub fn full_size(size: UVec3) -> UVec3 {
		uvec3(size.x, size.y * 2, size.z * 3)
	}

	pub fn delinearize(&self, idx: usize) -> (UVec3, IrradianceVolumeDirection) {
		let pos = UVec3::from_array(Shape::delinearize(&self.full_shape, idx as u32));
		let grid_offset = uvec3(0, pos.y / self.size.y, pos.z / self.size.z);
		let dir = IrradianceVolumeDirection::from_offset(grid_offset).expect("idx out of bounds");

		(pos - grid_offset * self.size, dir)
	}

	pub fn linearize(&self, pos: impl Into<UVec3>, dir: IrradianceVolumeDirection) -> usize {
		let mut pos: UVec3 = pos.into();
		pos += dir.offset() * self.size;
		Shape::linearize(&self.full_shape, [pos.x, pos.y, pos.z]) as usize
	}

	#[inline]
	#[track_caller]
	pub fn put(&mut self, pos: impl Into<UVec3>, dir: IrradianceVolumeDirection, color: [u8; 4]) {
		let idx = self.linearize(pos, dir);

		self.data[idx] = color;
		self.filled[idx] = true;
	}

	// TODO Right now we waste the directionality of irradiance volumes when using light grids. Not quite show how yet, but we should fix this in the future.

	#[inline]
	#[track_caller]
	pub fn put_all(&mut self, pos: impl Into<UVec3>, color: [u8; 4]) {
		#[inline]
		fn mul_color([r, g, b, a]: [u8; 4], [mul_r, mul_g, mul_b]: [f32; 3]) -> [u8; 4] {
			[(r as f32 * mul_r) as u8, (g as f32 * mul_g) as u8, (b as f32 * mul_b) as u8, a]
		}

		let pos = pos.into();
		self.put(pos, IrradianceVolumeDirection::X, mul_color(color, self.multipliers.x));
		self.put(pos, IrradianceVolumeDirection::Y, mul_color(color, self.multipliers.y));
		self.put(pos, IrradianceVolumeDirection::Z, mul_color(color, self.multipliers.z));
		self.put(pos, IrradianceVolumeDirection::NEG_X, mul_color(color, self.multipliers.neg_x));
		self.put(pos, IrradianceVolumeDirection::NEG_Y, mul_color(color, self.multipliers.neg_y));
		self.put(pos, IrradianceVolumeDirection::NEG_Z, mul_color(color, self.multipliers.neg_z));
	}

	/// For any non-filled color, get replaced with neighboring filled colors.
	pub fn flood_non_filled(&mut self) {
		for (i, filled) in self.filled.iter().copied().enumerate() {
			if filled {
				continue;
			}

			let (pos, dir) = self.delinearize(i);
			let min = pos.saturating_sub(UVec3::splat(1));
			let max = (pos + 1).min(self.size - 1);

			let mut color = [0_u16; 4];
			let mut contributors: u16 = 0;

			for x in min.x..=max.x {
				for y in min.y..=max.y {
					for z in min.z..=max.z {
						let offset_idx = self.linearize([x, y, z], dir);

						if self.filled[offset_idx] {
							contributors += 1;
							#[allow(clippy::needless_range_loop)]
							for color_channel in 0..4 {
								color[color_channel] += self.data[offset_idx][color_channel] as u16;
							}
						}
					}
				}
			}

			if contributors == 0 {
				continue;
			}
			// Average 'em
			self.data[i] = color.map(|v| (v / contributors) as u8)
		}
	}

	pub fn build(self) -> Image {
		Image::new(
			Extent3d {
				width: self.size.x,
				height: self.size.y * 2,
				depth_or_array_layers: self.size.z * 3,
			},
			TextureDimension::D3,
			self.data.into_flattened(),
			TextureFormat::Rgba8UnormSrgb,
			RenderAssetUsages::RENDER_WORLD,
		)
	}
}

#[derive(Debug, Clone, Copy)]
struct IrradianceVolumeDirection(UVec3);
impl IrradianceVolumeDirection {
	pub fn from_offset(offset: UVec3) -> Option<Self> {
		if offset.x != 0 || !(0..=1).contains(&offset.y) || !(0..=2).contains(&offset.z) {
			None
		} else {
			Some(Self(offset))
		}
	}

	#[inline]
	pub fn offset(&self) -> UVec3 {
		self.0
	}

	pub const X: Self = Self(uvec3(0, 1, 0));
	pub const Y: Self = Self(uvec3(0, 1, 1));
	pub const Z: Self = Self(uvec3(0, 1, 2));
	pub const NEG_X: Self = Self(uvec3(0, 0, 0));
	pub const NEG_Y: Self = Self(uvec3(0, 0, 1));
	pub const NEG_Z: Self = Self(uvec3(0, 0, 2));
}
