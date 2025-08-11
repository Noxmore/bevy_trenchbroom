mod types;
pub use types::*;

use bevy::{
	asset::{RenderAssetUsages, embedded_asset},
	image::ImageSampler,
	pbr::Lightmap,
	render::{
		Render, RenderApp, RenderSystems,
		extract_resource::{ExtractResource, ExtractResourcePlugin},
		globals::{GlobalsBuffer, GlobalsUniform},
		render_asset::{RenderAssetPlugin, RenderAssets},
		render_graph::{RenderGraph, RenderLabel},
		render_resource::{binding_types::*, *},
		renderer::{RenderDevice, RenderQueue},
		texture::GpuImage,
	},
};

use crate::*;

/// Max animators passed to a shader, must match value in `composite_lightmaps.wgsl`.
pub const MAX_ANIMATORS: usize = 255;
/// Max number of steps in a [`LightingAnimator`].
pub const MAX_LIGHTMAP_FRAMES: usize = 64;

const IRRADIANCE_VOLUME_WORKGROUP_SIZE: u32 = 4;
const LIGHTMAP_WORKGROUP_SIZE: u32 = 8;

/// The format used for the out channel in lighting animation. I've found that [`TextureFormat::Rgba8UnormSrgb`] and the like show noticeable banding on slower animations.
pub(crate) const ANIMATED_LIGHTING_OUTPUT_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba32Float;
pub(crate) fn new_animated_lighting_output_image(extent: Extent3d, dimension: TextureDimension) -> Image {
	let mut image = Image::new_fill(
		extent,
		dimension,
		// Bright color -- easy to spot errors
		[0.0_f32, 1., 0., 1.].map(|f| f.to_ne_bytes()).as_flattened(),
		ANIMATED_LIGHTING_OUTPUT_TEXTURE_FORMAT,
		RenderAssetUsages::RENDER_WORLD,
	);
	image.texture_descriptor.usage |= TextureUsages::STORAGE_BINDING;
	image.sampler = ImageSampler::linear();
	image
}

pub struct BspLightingPlugin;
impl Plugin for BspLightingPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "composite_lightmaps.wgsl");
		embedded_asset!(app, "composite_irradiance_volumes.wgsl");

		#[rustfmt::skip]
		app
			.add_plugins(RenderAssetPlugin::<AnimatedLighting>::default())

			.add_plugins(ExtractResourcePlugin::<LightingAnimators>::default())
			.init_resource::<LightingAnimators>()

			.init_asset::<AnimatedLighting>()

			.add_systems(PreUpdate, Self::insert_animated_lightmaps)
		;

		let render_app = app.sub_app_mut(RenderApp);

		render_app.init_resource::<AnimatedLightingBindGroups>();

		render_app.add_systems(
			Render,
			Self::prepare_animated_lighting_bind_groups.in_set(RenderSystems::PrepareBindGroups),
		);

		let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
		render_graph.add_node(AnimatedLightingLabel, AnimatedLightingNode);

		render_graph.add_node_edge(AnimatedLightingLabel, bevy::render::graph::CameraDriverLabel);
	}

	fn finish(&self, app: &mut App) {
		app.sub_app_mut(RenderApp).init_resource::<AnimatedLightingPipeline>();
	}
}
impl BspLightingPlugin {
	/// Inserts [`Lightmap`] components into entities with [`AnimatedLighting`].
	pub fn insert_animated_lightmaps(
		mut commands: Commands,
		query: Query<(Entity, &AnimatedLightingHandle), (Without<Lightmap>, Without<IrradianceVolume>)>,
		animated_lighting_assets: Res<Assets<AnimatedLighting>>,
		tb_server: Res<TrenchBroomServer>,
	) {
		for (entity, animated_lightmap) in &query {
			let Some(animated_lighting) = animated_lighting_assets.get(&animated_lightmap.0) else { continue };

			match animated_lighting.ty {
				AnimatedLightingType::Lightmap => {
					commands.entity(entity).insert(Lightmap {
						image: animated_lighting.output.clone(),
						uv_rect: Rect::new(0., 0., 1., 1.),
						bicubic_sampling: tb_server.config.bicubic_lightmap_filtering,
					});
				}
				AnimatedLightingType::IrradianceVolume => {
					commands.entity(entity).insert(IrradianceVolume {
						voxels: animated_lighting.output.clone(),
						intensity: tb_server.config.default_irradiance_volume_intensity,
						affects_lightmapped_meshes: false, // TODO: This might help with normals?
					});
				}
			}
		}
	}

	fn prepare_animated_lighting_bind_groups(
		animated_lighting_assets: Res<RenderAssets<AnimatedLighting>>,
		mut bind_groups: ResMut<AnimatedLightingBindGroups>,
		gpu_images: Res<RenderAssets<GpuImage>>,
		render_device: Res<RenderDevice>,
		pipeline: Res<AnimatedLightingPipeline>,
		animators: Res<LightingAnimators>,
		render_queue: Res<RenderQueue>,
		globals: Res<GlobalsBuffer>,
	) {
		bind_groups.globals = Some(render_device.create_bind_group(
			None,
			&pipeline.globals_bind_group_layout,
			&[BindGroupEntry {
				binding: 0,
				resource: globals.buffer.into_binding(),
			}],
		));

		if !animated_lighting_assets.is_changed() && !animators.is_changed() {
			return;
		}
		bind_groups.values.clear();

		let mut target_animators = [LightingAnimator::default(); MAX_ANIMATORS];
		for (style, animator) in &animators.values {
			target_animators[style.0 as usize] = *animator;
		}
		let mut target_animators_buffer = StorageBuffer::from(target_animators);
		target_animators_buffer.set_label(Some("animators"));
		target_animators_buffer.write_buffer(&render_device, &render_queue);

		for (id, animated_lighting) in animated_lighting_assets.iter() {
			match animated_lighting.ty {
				AnimatedLightingType::Lightmap => {
					bind_groups.values.insert(
						id,
						render_device.create_bind_group(
							None,
							&pipeline.lightmap_bind_group_layout,
							&BindGroupEntries::sequential((
								&gpu_images.get(&animated_lighting.input[0]).unwrap().texture_view,
								&gpu_images.get(&animated_lighting.input[1]).unwrap().texture_view,
								&gpu_images.get(&animated_lighting.input[2]).unwrap().texture_view,
								&gpu_images.get(&animated_lighting.input[3]).unwrap().texture_view,
								&gpu_images.get(&animated_lighting.styles).unwrap().texture_view,
								target_animators_buffer.into_binding(),
								&gpu_images.get(&animated_lighting.output).unwrap().texture_view,
							)),
						),
					);
				}
				AnimatedLightingType::IrradianceVolume => {
					bind_groups.values.insert(
						id,
						render_device.create_bind_group(
							None,
							&pipeline.irradiance_volume_bind_group_layout,
							&BindGroupEntries::sequential((
								&gpu_images.get(&animated_lighting.input[0]).unwrap().texture_view,
								&gpu_images.get(&animated_lighting.input[1]).unwrap().texture_view,
								&gpu_images.get(&animated_lighting.input[2]).unwrap().texture_view,
								&gpu_images.get(&animated_lighting.input[3]).unwrap().texture_view,
								&gpu_images.get(&animated_lighting.styles).unwrap().texture_view,
								target_animators_buffer.into_binding(),
								&gpu_images.get(&animated_lighting.output).unwrap().texture_view,
							)),
						),
					);
				}
			}
		}
	}
}

#[derive(Resource, Default)]
pub struct AnimatedLightingBindGroups {
	pub values: HashMap<AssetId<AnimatedLighting>, BindGroup>,
	pub globals: Option<BindGroup>,
}

#[derive(Resource, ExtractResource, Clone)]
pub struct AnimatedLightingPipeline {
	lightmap_bind_group_layout: BindGroupLayout,
	irradiance_volume_bind_group_layout: BindGroupLayout,
	// Globals (for time) is split into another bind group, so i don't have to update the main bind group every frame.
	// Does this help with performance? No idea, I don't even know how to profile it.
	globals_bind_group_layout: BindGroupLayout,
	lightmap_pipeline: CachedComputePipelineId,
	irradiance_volume_pipeline: CachedComputePipelineId,
}
impl FromWorld for AnimatedLightingPipeline {
	fn from_world(world: &mut World) -> Self {
		let render_device = world.resource::<RenderDevice>();

		let lightmap_bind_group_layout = render_device.create_bind_group_layout(
			"LightmapCompositeImages",
			&BindGroupLayoutEntries::sequential(
				ShaderStages::COMPUTE,
				(
					texture_2d(TextureSampleType::Float { filterable: false }),
					texture_2d(TextureSampleType::Float { filterable: false }),
					texture_2d(TextureSampleType::Float { filterable: false }),
					texture_2d(TextureSampleType::Float { filterable: false }),
					texture_2d(TextureSampleType::Uint),
					storage_buffer_read_only_sized(false, Some(<[LightingAnimator; MAX_ANIMATORS]>::SHADER_SIZE)),
					texture_storage_2d(ANIMATED_LIGHTING_OUTPUT_TEXTURE_FORMAT, StorageTextureAccess::WriteOnly),
				),
			),
		);

		let irradiance_volume_bind_group_layout = render_device.create_bind_group_layout(
			"IrradianceVolumeCompositeImages",
			&BindGroupLayoutEntries::sequential(
				ShaderStages::COMPUTE,
				(
					texture_3d(TextureSampleType::Float { filterable: false }),
					texture_3d(TextureSampleType::Float { filterable: false }),
					texture_3d(TextureSampleType::Float { filterable: false }),
					texture_3d(TextureSampleType::Float { filterable: false }),
					texture_3d(TextureSampleType::Uint),
					storage_buffer_read_only_sized(false, Some(<[LightingAnimator; MAX_ANIMATORS]>::SHADER_SIZE)),
					BindingType::StorageTexture {
						access: StorageTextureAccess::WriteOnly,
						format: ANIMATED_LIGHTING_OUTPUT_TEXTURE_FORMAT,
						view_dimension: TextureViewDimension::D3,
					}
					.into_bind_group_layout_entry_builder(),
				),
			),
		);

		let globals_bind_group_layout = render_device.create_bind_group_layout(
			None,
			&BindGroupLayoutEntries::single(ShaderStages::COMPUTE, uniform_buffer_sized(false, Some(GlobalsUniform::SHADER_SIZE))),
		);

		let pipeline_cache = world.resource::<PipelineCache>();

		let lightmap_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
			label: Some("Composite Lightmap Images Pipeline".into()),
			layout: vec![lightmap_bind_group_layout.clone(), globals_bind_group_layout.clone()],
			push_constant_ranges: vec![],
			shader: world.load_asset("embedded://bevy_trenchbroom/bsp/lighting/composite_lightmaps.wgsl"),
			shader_defs: vec![],
			entry_point: Some("main".into()),
			zero_initialize_workgroup_memory: true,
		});

		let irradiance_volume_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
			label: Some("Composite Irradiance Volume Images Pipeline".into()),
			layout: vec![irradiance_volume_bind_group_layout.clone(), globals_bind_group_layout.clone()],
			push_constant_ranges: vec![],
			shader: world.load_asset("embedded://bevy_trenchbroom/bsp/lighting/composite_irradiance_volumes.wgsl"),
			shader_defs: vec![],
			entry_point: Some("main".into()),
			zero_initialize_workgroup_memory: true,
		});

		Self {
			lightmap_bind_group_layout,
			irradiance_volume_bind_group_layout,
			globals_bind_group_layout,
			lightmap_pipeline,
			irradiance_volume_pipeline,
		}
	}
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct AnimatedLightingLabel;

pub struct AnimatedLightingNode;

impl bevy::render::render_graph::Node for AnimatedLightingNode {
	fn run<'w>(
		&self,
		_graph: &mut bevy::render::render_graph::RenderGraphContext,
		render_context: &mut bevy::render::renderer::RenderContext<'w>,
		world: &'w World,
	) -> Result<(), bevy::render::render_graph::NodeRunError> {
		let bind_groups = world.resource::<AnimatedLightingBindGroups>();
		let Some(globals_bind_group) = &bind_groups.globals else { return Ok(()) };
		let pipeline_cache = world.resource::<PipelineCache>();
		let pipeline = world.resource::<AnimatedLightingPipeline>();
		let animated_lighting_assets = world.resource::<RenderAssets<AnimatedLighting>>();
		let gpu_images = world.resource::<RenderAssets<GpuImage>>();

		let mut pass = render_context.command_encoder().begin_compute_pass(&ComputePassDescriptor {
			label: Some("Composite animated lighting"),
			..default()
		});

		for (id, bind_group) in &bind_groups.values {
			let Some(animated_lighting) = animated_lighting_assets.get(*id) else { continue };
			let Some(output_image) = gpu_images.get(&animated_lighting.output) else { continue };

			// TODO if there is only unanimated styles, and it's already run once, we don't need to run it again!
			match animated_lighting.ty {
				AnimatedLightingType::Lightmap => {
					let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline.lightmap_pipeline) else { return Ok(()) };

					pass.set_pipeline(pipeline);

					pass.set_bind_group(0, bind_group, &[]);
					pass.set_bind_group(1, globals_bind_group, &[]);

					pass.dispatch_workgroups(
						output_image.size.width.div_ceil(LIGHTMAP_WORKGROUP_SIZE),
						output_image.size.height.div_ceil(LIGHTMAP_WORKGROUP_SIZE),
						1,
					);
				}

				AnimatedLightingType::IrradianceVolume => {
					let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline.irradiance_volume_pipeline) else { return Ok(()) };

					pass.set_pipeline(pipeline);

					pass.set_bind_group(0, bind_group, &[]);
					pass.set_bind_group(1, globals_bind_group, &[]);

					pass.dispatch_workgroups(
						output_image.size.width.div_ceil(IRRADIANCE_VOLUME_WORKGROUP_SIZE),
						output_image.size.height.div_ceil(IRRADIANCE_VOLUME_WORKGROUP_SIZE),
						output_image.size.depth_or_array_layers.div_ceil(IRRADIANCE_VOLUME_WORKGROUP_SIZE),
					);
				}
			}
		}

		Ok(())
	}
}
