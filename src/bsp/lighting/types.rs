//! Data types split off from the main lighting module for organization.

use bevy::render::{extract_resource::ExtractResource, render_asset::RenderAsset, render_resource::*};
use ser::SerializeStruct;

use crate::*;

use super::MAX_LIGHTMAP_FRAMES;

/// Provides the *animation* of animated lightmaps.
#[derive(ShaderType, Reflect, Debug, Clone, Copy)]
pub struct LightingAnimator {
	/// The sequence of values to multiply the light style's lightmap with. Each frame is an RGB value.
	pub sequence: [Vec3; MAX_LIGHTMAP_FRAMES],
	/// How many frames to read of `sequence` before starting from the beginning.
	pub sequence_len: u32,

	/// How many frames of `sequence` to advance a second.
	pub speed: f32,

	/// How much to linearly interpolate between elements in the sequence.
	///
	/// 0 swaps between them instantly, 1 smoothly interpolates,
	/// 0.5 interpolates to the next frame 2 times as quick, stopping in the middle, etc.
	pub interpolate: f32,
}
impl LightingAnimator {
	pub fn new<const N: usize>(speed: f32, interpolate: f32, sequence: [Vec3; N]) -> Self {
		let mut target_sequence = [Vec3::ZERO; MAX_LIGHTMAP_FRAMES];
		#[allow(clippy::manual_memcpy)]
		for i in 0..N {
			target_sequence[i] = sequence[i];
		}

		Self {
			speed,
			interpolate,
			sequence: target_sequence,
			sequence_len: N as u32,
		}
	}

	#[inline]
	pub fn unanimated(rgb: Vec3) -> Self {
		Self {
			sequence: [rgb; MAX_LIGHTMAP_FRAMES],
			sequence_len: 1,
			speed: 0.,
			interpolate: 0.,
		}
	}

	/// Samples this animator given the current elapsed seconds the same way it samples in the shader.
	///
	/// # Example
	/// ```
	/// # use bevy::prelude::*;
	/// # use bevy_trenchbroom::prelude::*;
	/// fn sample_example(
	///     time: Res<Time>,
	///     animators: Res<LightingAnimators>,
	/// ) {
	///     let _ = animators.values.get(&LightmapStyle(1)).unwrap().sample(time.elapsed_secs());
	/// }
	/// ```
	pub fn sample(&self, seconds: f32) -> Vec3 {
		// MUST stay the same as in shaders
		let mut mul = self.sequence[(seconds * self.speed) as usize % self.sequence_len as usize];

		if self.interpolate > 0. {
			let next = self.sequence[((seconds * self.speed) as usize + 1) % self.sequence_len as usize];
			let t = (((seconds * self.speed) % 1.) / self.interpolate).min(1.);
			mul = mul.lerp(next, t);
		}

		mul
	}
}
impl Default for LightingAnimator {
	fn default() -> Self {
		Self::unanimated(Vec3::ONE)
	}
}
impl Serialize for LightingAnimator {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let mut s = serializer.serialize_struct("LightingAnimator", 3)?;

		s.serialize_field("sequence", &self.sequence[..usize::min(self.sequence_len as usize, MAX_LIGHTMAP_FRAMES)])?;

		s.serialize_field("speed", &self.speed)?;
		s.serialize_field("interpolate", &self.interpolate)?;

		s.end()
	}
}
// Holy boilerplate, Batman!
impl<'de> Deserialize<'de> for LightingAnimator {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		enum Field {
			Sequence,
			Speed,
			Interpolate,
			Ignore,
		}
		struct FieldVisitor;
		impl de::Visitor<'_> for FieldVisitor {
			type Value = Field;

			fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
				fmt.write_str("field identifier")
			}

			fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
				Ok(match v {
					0 => Field::Sequence,
					1 => Field::Speed,
					2 => Field::Interpolate,
					_ => Field::Ignore,
				})
			}

			fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
				Ok(match v {
					"sequence" => Field::Sequence,
					"speed" => Field::Speed,
					"interpolate" => Field::Interpolate,
					_ => Field::Ignore,
				})
			}

			fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
				Ok(match v {
					b"sequence" => Field::Sequence,
					b"speed" => Field::Speed,
					b"interpolate" => Field::Interpolate,
					_ => Field::Ignore,
				})
			}
		}
		impl<'de> Deserialize<'de> for Field {
			fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
				deserializer.deserialize_identifier(FieldVisitor)
			}
		}

		struct Visitor;
		impl<'de> de::Visitor<'de> for Visitor {
			type Value = LightingAnimator;

			fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
				fmt.write_str("struct LightingAnimator")
			}

			// rustfmt is threatening to make these look even more boilerplate-y then they already do.
			#[rustfmt::skip]
			fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
				let sequence_vec: Vec<Vec3> = seq.next_element()?.ok_or(de::Error::invalid_length(0, &"struct LightingAnimator with 3 elements"))?;
				let speed: f32 = seq.next_element()?.ok_or(de::Error::invalid_length(1, &"struct LightingAnimator with 3 elements"))?;
				let interpolate: f32 = seq.next_element()?.ok_or(de::Error::invalid_length(2, &"struct LightingAnimator with 3 elements"))?;

				visit_internal(sequence_vec, speed, interpolate)
			}

			#[rustfmt::skip]
			fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
				let mut sequence_vec: Option<Vec<Vec3>> = None;
				let mut speed: Option<f32> = None;
				let mut interpolate: Option<f32> = None;

				while let Some(key) = map.next_key::<Field>()? {
					match key {
						Field::Sequence => if sequence_vec.is_some() {
							return Err(de::Error::duplicate_field("sequence"));
						} else {
							sequence_vec = map.next_value()?;
						},
						Field::Speed => if speed.is_some() {
							return Err(de::Error::duplicate_field("speed"));
						} else {
							speed = map.next_value()?;
						},
						Field::Interpolate => if interpolate.is_some() {
							return Err(de::Error::duplicate_field("interpolate"));
						} else {
							interpolate = map.next_value()?;
						},
						Field::Ignore => {
							map.next_value::<de::IgnoredAny>()?;
						},
					}
				}

				visit_internal(
					sequence_vec.ok_or(de::Error::missing_field("sequence"))?,
					speed.ok_or(de::Error::missing_field("speed"))?,
					interpolate.ok_or(de::Error::missing_field("interpolate"))?,
				)
			}
		}

		fn visit_internal<E: de::Error>(sequence_vec: Vec<Vec3>, speed: f32, interpolate: f32) -> Result<LightingAnimator, E> {
			if sequence_vec.len() > MAX_LIGHTMAP_FRAMES {
				return Err(de::Error::custom(format_args!(
					"sequence has {} frames, but the max is {MAX_LIGHTMAP_FRAMES}",
					sequence_vec.len()
				)));
			}

			let mut sequence = [Vec3::ZERO; MAX_LIGHTMAP_FRAMES];
			sequence[..sequence_vec.len()].copy_from_slice(&sequence_vec);

			Ok(LightingAnimator {
				sequence,
				sequence_len: sequence_vec.len() as u32,
				speed,
				interpolate,
			})
		}

		deserializer.deserialize_struct("LightingAnimator", &["sequence", "speed", "interpolate"], Visitor)
	}
}

/// Resource that contains the current lightmap animators for each [`LightmapStyle`].
///
/// You can use this to change animations, and do things like toggle lights.
#[derive(Resource, ExtractResource, Reflect, Debug, Clone, Default, Serialize, Deserialize)]
#[reflect(Resource, Default, Serialize, Deserialize)]
pub struct LightingAnimators {
	pub values: HashMap<LightmapStyle, LightingAnimator>,
}

/// Contains multiple images that are composited together in `output` using the current [`TrenchBroomConfig`]'s lightmap animators. Used for both lightmaps an irradiance volumes.
#[derive(Asset, TypePath, Clone)]
pub struct AnimatedLighting {
	pub ty: AnimatedLightingType,

	/// The image to output the composited image to.
	pub output: Handle<Image>,

	/// An input image for each lightmap style slot globally.
	pub input: [Handle<Image>; 4],

	/// An image containing the [`LightmapStyle`]s to use for each pixel.
	///
	/// 4 8-bit color channels refer to 4 lightmap style slots, each channel being the animator to use to composite.
	pub styles: Handle<Image>,
}
impl RenderAsset for AnimatedLighting {
	type SourceAsset = Self;
	type Param = ();

	fn prepare_asset(
		source_asset: Self::SourceAsset,
		_asset_id: AssetId<Self::SourceAsset>,
		_param: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
	) -> std::result::Result<Self, bevy::render::render_asset::PrepareAssetError<Self::SourceAsset>> {
		Ok(source_asset)
	}
}

/// Holds an [`AnimatedLighting`] handle and automatically inserts the output [`Lightmap`](bevy::pbr::Lightmap) or [`IrradianceVolume`] based on [`AnimatedLighting::ty`] onto the entity.
#[derive(Component, Reflect, Debug, Clone, Default, PartialEq, Eq, Deref, DerefMut)]
#[reflect(Component, Default)]
pub struct AnimatedLightingHandle(pub Handle<AnimatedLighting>);

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimatedLightingType {
	/// 2D textures, if textures differ in size they will be stretched.
	Lightmap,
	/// 3D textures, if texture differ in size they will repeat, this allows for non-directional volumes to store a 6th of the required data.
	IrradianceVolume,
}
