use bevy::{
	asset::RenderAssetUsages,
	render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use ndshape::{RuntimeShape, Shape};

use crate::*;

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
	pub const ONE: Self = Self {
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
		Self::ONE
	}
}

/// Little helper API to create irradiance volumes for BSPs.
pub(super) struct IrradianceVolumeBuilder {
	size: UVec3,
	full_shape: RuntimeShape<u32, 3>,
	data: Vec<[u8; 4]>,
	filled: Vec<bool>,
	multipliers: IrradianceVolumeMultipliers,
}
impl IrradianceVolumeBuilder {
	pub fn new(size: impl Into<UVec3>, default_color: [u8; 4], multipliers: IrradianceVolumeMultipliers) -> Self {
		let size: UVec3 = size.into();
		let shape = RuntimeShape::<u32, 3>::new([size.x, size.y * 2, size.z * 3]);
		let vec_size = shape.usize();
		Self {
			size,
			full_shape: shape,
			data: vec![default_color; vec_size],
			filled: vec![false; vec_size],
			multipliers,
		}
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
pub(super) struct IrradianceVolumeDirection(UVec3);
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
