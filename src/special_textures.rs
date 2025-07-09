use bevy::{
	asset::embedded_asset,
	pbr::{ExtendedMaterial, MaterialExtension},
	render::render_resource::AsBindGroup,
};
#[cfg(feature = "bsp")]
use bevy::{
	image::TextureFormatPixelInfo,
	render::render_resource::{Extent3d, TextureDimension},
};
#[cfg(feature = "bsp")]
use bevy_materialize::animation::{GenericMaterialAnimationState, ImagesAnimation, MaterialAnimations};
#[cfg(feature = "bsp")]
use bsp::TEXTURE_PREFIX;
#[cfg(feature = "bsp")]
use config::EmbeddedTextureLoadView;
use wgpu_types::Face;

use crate::*;

pub struct SpecialTexturesPlugin;
impl Plugin for SpecialTexturesPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "liquid.wgsl");
		embedded_asset!(app, "quake_sky.wgsl");

		#[rustfmt::skip]
		app
			.add_plugins(MaterialPlugin::<LiquidMaterial>::default())
			.add_plugins(MaterialPlugin::<QuakeSkyMaterial>::default())
		;
	}
}

/// If [`TrenchBroomConfig`] contains [Quake special textures](https://quakewiki.org/wiki/Textures), this attempts to load them using the material provided as a base.
#[cfg(feature = "bsp")]
pub fn load_special_texture(view: &mut EmbeddedTextureLoadView, material: &StandardMaterial) -> Option<GenericMaterial> {
	// We save a teeny tiny bit of time by only cloning if we need to :)
	let mut material = material.clone();
	if let Some(exposure) = view.tb_config.lightmap_exposure {
		material.lightmap_exposure = exposure;
	}

	if let Some(default_fn) = view.tb_config.embedded_liquid_material {
		if view.name.starts_with('*') {
			let water_alpha: f32 = view
				.entities
				.worldspawn()
				.and_then(|worldspawn| worldspawn.get("water_alpha").ok())
				.unwrap_or(1.);

			if water_alpha < 1. {
				material.alpha_mode = AlphaMode::Blend;
				material.base_color = Color::srgba(1., 1., 1., water_alpha);
			}

			let handle = view.add_material(LiquidMaterial {
				base: material,
				extension: default_fn(),
			});

			return Some(GenericMaterial {
				handle: handle.into(),
				properties: default(),
			});
		}
	}
	if let Some(default_fn) = view.tb_config.embedded_quake_sky_material {
		if view.name.starts_with("sky") {
			// We need to separate the sky into the 2 foreground and background images here because otherwise we will get weird wrapping when linear filtering is on.

			fn separate_sky_image(view: &mut EmbeddedTextureLoadView, x_range: std::ops::Range<u32>, alpha_on_black: bool) -> Image {
				// Technically, we know what the format should be, but this is just a bit more generic && reusable i guess
				let mut data: Vec<u8> =
					Vec::with_capacity(((view.image.width() / 2) * view.image.height()) as usize * view.image.texture_descriptor.format.pixel_size());

				// Because of the borrow checker we have to use a classic for loop instead of the iterator API :DDD
				for y in 0..view.image.height() {
					for x in x_range.clone() {
						if alpha_on_black && view.image.get_color_at(x, y).unwrap().to_srgba() == Srgba::BLACK {
							data.extend(repeat_n(0, view.image.texture_descriptor.format.pixel_size()));
							// data.extend([127, 127, 127, 0]);
						} else {
							data.extend(view.image.pixel_bytes(uvec3(x, y, 0)).unwrap());
						}
					}
				}

				let mut image = Image::new(
					Extent3d {
						width: view.image.width() / 2,
						height: view.image.height(),
						depth_or_array_layers: 1,
					},
					TextureDimension::D2,
					data,
					view.image.texture_descriptor.format,
					view.tb_config.bsp_textures_asset_usages,
				);

				image.sampler = view.tb_config.texture_sampler.clone();

				image
			}

			let fg = separate_sky_image(view, 0..view.image.width() / 2, true);
			let fg = view
				.parent_view
				.load_context
				.add_labeled_asset(format!("FG_{TEXTURE_PREFIX}{}", view.name), fg);

			let bg = separate_sky_image(view, view.image.width() / 2..view.image.width(), false);
			let bg = view
				.parent_view
				.load_context
				.add_labeled_asset(format!("BG_{TEXTURE_PREFIX}{}", view.name), bg);

			let handle = view.add_material(QuakeSkyMaterial { fg, bg, ..default_fn() });

			return Some(GenericMaterial {
				handle: handle.into(),
				properties: default(),
			});
		}
	}
	if let Some(fps) = view.tb_config.embedded_texture_animation_fps {
		if view.name.starts_with('+') {
			let embedded_textures = view.embedded_textures?;

			let mut chars = view.name.chars();
			chars.next();

			let texture_frame_idx = chars.next().and_then(|c| c.to_digit(10))?;
			let name_content = &view.name[2..];

			let mut frames = Vec::new();
			let mut frame_num = 0;
			while let Some((_, frame_handle)) = embedded_textures.get(format!("+{frame_num}{name_content}").as_str()) {
				frames.push(frame_handle.clone());
				frame_num += 1;
			}

			let handle = view.add_material(material);

			let mut generic_material = GenericMaterial::new(handle);

			generic_material.set_property(
				GenericMaterial::ANIMATION,
				MaterialAnimations {
					next: None,
					images: Some(ImagesAnimation {
						fps,
						fields: [("base_color_texture".s(), frames)].into_iter().collect(),
						state: GenericMaterialAnimationState {
							current_frame: texture_frame_idx.wrapping_sub(1) as usize,
							next_frame_time: Duration::default(),
						},
					}),
				},
			);

			return Some(generic_material);
		}
	}

	None
}

/// Material extension to [`StandardMaterial`] that emulates the wave effect of Quake liquid.
pub type LiquidMaterial = ExtendedMaterial<StandardMaterial, LiquidMaterialExt>;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, SmartDefault)]
pub struct LiquidMaterialExt {
	#[uniform(100)]
	#[default(0.1)]
	pub magnitude: f32,
	#[uniform(100)]
	#[default(PI)]
	pub cycles: f32,
}
impl MaterialExtension for LiquidMaterialExt {
	fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
		"embedded://bevy_trenchbroom/liquid.wgsl".into()
	}
}

/// Material that emulates the Quake sky.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, SmartDefault)]
#[bind_group_data(QuakeSkyKey)]
pub struct QuakeSkyMaterial {
	/// The speed the foreground layer moves.
	#[uniform(0)]
	#[default(Vec2::splat(0.1))]
	pub fg_scroll: Vec2,
	/// The speed the background layer moves.
	#[uniform(0)]
	#[default(Vec2::splat(0.05))]
	pub bg_scroll: Vec2,
	/// The scale of the textures.
	#[uniform(0)]
	#[default(2.)]
	pub texture_scale: f32,
	/// Scales the sphere before it is re-normalized, used to shape it.
	#[uniform(0)]
	#[default(vec3(1., 3., 1.))]
	pub sphere_scale: Vec3,

	#[texture(1)]
	#[sampler(2)]
	pub fg: Handle<Image>,

	#[texture(3)]
	#[sampler(4)]
	pub bg: Handle<Image>,

	/// Whether to cull the "front", "back" or neither side of a mesh.
	/// If set to `None`, the two sides of the mesh are visible.
	///
	/// Defaults to `Some(Face::Back)`.
	#[reflect(ignore, clone)]
	#[default(Some(Face::Back))]
	pub cull_mode: Option<Face>,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct QuakeSkyKey {
	pub cull_mode: Option<Face>,
}
impl From<&QuakeSkyMaterial> for QuakeSkyKey {
	fn from(value: &QuakeSkyMaterial) -> Self {
		Self { cull_mode: value.cull_mode }
	}
}
impl Material for QuakeSkyMaterial {
	fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
		"embedded://bevy_trenchbroom/quake_sky.wgsl".into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}

	fn specialize(
		_pipeline: &bevy::pbr::MaterialPipeline<Self>,
		descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
		_layout: &bevy_mesh::MeshVertexBufferLayoutRef,
		key: bevy::pbr::MaterialPipelineKey<Self>,
	) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
		// Noxmore: If we don't do this, cameras with a depth prepass will completely mess up
		// rendering culled faces, showing the app clear color instead of anything behind them.
		// This causes some nasty visual errors in some maps.
		// Why does this fix that? I have no idea !!!!
		descriptor.primitive.cull_mode = key.bind_group_data.cull_mode;
		Ok(())
	}
}
