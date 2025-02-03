use bevy::{
	asset::embedded_asset,
	image::ImageSampler,
	pbr::Lightmap,
	render::{
		extract_resource::{ExtractResource, ExtractResourcePlugin},
		globals::{GlobalsBuffer, GlobalsUniform},
		mesh::PrimitiveTopology,
		render_asset::{RenderAsset, RenderAssetPlugin, RenderAssetUsages, RenderAssets},
		render_graph::{RenderGraph, RenderLabel},
		render_resource::{binding_types::*, *},
		renderer::{RenderDevice, RenderQueue},
		texture::GpuImage,
		Render, RenderApp, RenderSet,
	},
};

use crate::*;

/// Max animators passed to a shader, must match value in `composite_lightmaps.wgsl`.
pub const MAX_ANIMATORS: usize = 255;
pub const MAX_LIGHTMAP_FRAMES: usize = 64;

pub(crate) const LIGHTMAP_OUTPUT_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba32Float;
pub(crate) fn new_lightmap_output_image(width: u32, height: u32) -> Image {
	let mut image = Image::new_fill(
		Extent3d { width, height, ..default() },
		TextureDimension::D2,
		[0.0_f32, 1., 0., 1.].map(|f| f.to_ne_bytes()).as_flattened(),
		LIGHTMAP_OUTPUT_TEXTURE_FORMAT,
		RenderAssetUsages::RENDER_WORLD,
	);
	image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT; // We need to render to this from a shader
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
			.add_plugins(ExtractResourcePlugin::<LightmapAnimators>::default())
			.register_type::<AnimatedLightmap>()
			.init_resource::<LightmapAnimators>()
			.init_asset::<AnimatedLighting>()
			.add_systems(PreUpdate, Self::insert_animated_lightmaps)
		;

		let render_app = app.sub_app_mut(RenderApp);

		render_app.init_resource::<AnimatedLightingBindGroups>();

		render_app.add_systems(Render, Self::prepare_animated_lighting_bind_groups.in_set(RenderSet::PrepareBindGroups));

		let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
		render_graph.add_node(AnimatedLightingLabel, AnimatedLightingNode);

		render_graph.add_node_edge(AnimatedLightingLabel, bevy::render::graph::CameraDriverLabel);
	}

	fn finish(&self, app: &mut App) {
		app.sub_app_mut(RenderApp).init_resource::<AnimatedLightingPipeline>();
	}
}
impl BspLightingPlugin {
	pub fn insert_animated_lightmaps(
		mut commands: Commands,
		query: Query<(Entity, &AnimatedLightmap), Without<Lightmap>>,
		animated_lighting_assets: Res<Assets<AnimatedLighting>>,
	) {
		for (entity, animated_lightmap) in &query {
			let Some(animated_lighting) = animated_lighting_assets.get(&animated_lightmap.0) else { continue };

			commands.entity(entity).insert(Lightmap {
				image: animated_lighting.output.clone(),
				uv_rect: Rect::new(0., 0., 1., 1.),
			});
		}
	}

	pub fn prepare_animated_lighting_bind_groups(
		animated_lighting_assets: Res<RenderAssets<AnimatedLighting>>,
		mut bind_groups: ResMut<AnimatedLightingBindGroups>,
		gpu_images: Res<RenderAssets<GpuImage>>,
		render_device: Res<RenderDevice>,
		pipeline: Res<AnimatedLightingPipeline>,
		animators: Res<LightmapAnimators>,
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

		for (id, animated_lighting) in animated_lighting_assets.iter() {
			let mut target_animators = [LightmapAnimator::default(); MAX_ANIMATORS];
			for (style, animator) in &animators.values {
				target_animators[style.0 as usize] = *animator;
			}
			let mut target_animators_buffer = UniformBuffer::from(target_animators);
			target_animators_buffer.set_label(Some("animators"));
			target_animators_buffer.write_buffer(&render_device, &render_queue);

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
								&pipeline.sampler,
								target_animators_buffer.into_binding(),
							)),
						),
					);
				}
				AnimatedLightingType::IrradianceVolume => {
					// TODO Currently, GpuImage doesn't store its size in 3D, so we won't animated irradiance volumes until either that's fixed,
					//      or i decide to get a work-around going.
					let size: UVec3 = UVec3::ONE; // gpu_images.get(&animated_lighting.output).unwrap().size
					let size_buffer = UniformBuffer::from(size);
					target_animators_buffer.set_label(Some("size"));
					target_animators_buffer.write_buffer(&render_device, &render_queue);

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
								size_buffer.into_binding(),
								target_animators_buffer.into_binding(),
							)),
						),
					);
				}
			}
		}
	}
}

/// Provides the *animation* of animated lightmaps.
#[derive(ShaderType, Reflect, Debug, Clone, Copy)]
pub struct LightmapAnimator {
	/// The sequence of values to multiply the light style's lightmap with. Each frame is an RGB value.
	pub sequence: [Vec3; MAX_LIGHTMAP_FRAMES],
	/// How many frames to read of `sequence` before starting from the beginning.
	pub sequence_len: u32,

	/// How many frames of `sequence` to advance a second.
	pub speed: f32,

	/// Whether to linearly interpolate between elements in the sequence, or swap between them instantly.
	///
	/// Has to be a `u32` because it's being passed to a shader. Non-zero for `true`.
	pub interpolate: u32,
}
impl LightmapAnimator {
	pub fn new<const N: usize>(speed: f32, interpolate: bool, sequence: [Vec3; N]) -> Self {
		let mut target_sequence = [Vec3::ZERO; MAX_LIGHTMAP_FRAMES];
		#[allow(clippy::manual_memcpy)]
		for i in 0..N {
			target_sequence[i] = sequence[i];
		}

		Self {
			speed,
			interpolate: interpolate as u32,
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
			interpolate: 0,
		}
	}
}
impl Default for LightmapAnimator {
	fn default() -> Self {
		Self::unanimated(Vec3::ONE)
	}
}

// TODO reflect (LightmapStyle doesn't impl it)
#[derive(Resource, ExtractResource, Debug, Clone)]
pub struct LightmapAnimators {
	pub values: HashMap<LightmapStyle, LightmapAnimator>,
}

impl Default for LightmapAnimators {
	fn default() -> Self {
		Self {
			values: HashMap::from([
				// TODO copy quake's default animators?
				(
					LightmapStyle(1),
					LightmapAnimator::new(
						6.,
						true,
						[0.8, 0.75, 1., 0.7, 0.8, 0.7, 0.9, 0.7, 0.6, 0.7, 0.9, 1., 0.7].map(Vec3::splat),
					),
				),
				(LightmapStyle(2), LightmapAnimator::new(0.5, true, [0., 1.].map(Vec3::splat))),
			]),
		}
	}
}

/// Contains multiple images that are composited together in `output` using the current [TrenchBroomConfig]'s lightmap animators. Used for both lightmaps an irradiance volumes.
#[derive(Asset, TypePath, Clone)]
pub struct AnimatedLighting {
	pub ty: AnimatedLightingType,

	/// The image to output the composited image to.
	pub output: Handle<Image>,

	/// An input image for each lightmap style slot globally.
	pub input: [Handle<Image>; 4],

	/// An image containing the [LightmapStyle]s to use for each pixel.
	///
	/// 4 8-bit color channels refer to 4 lightmap style slots, each channel being the animator to use to composite.
	pub styles: Handle<Image>,
}
impl RenderAsset for AnimatedLighting {
	type SourceAsset = Self;
	type Param = ();

	fn prepare_asset(
		source_asset: Self::SourceAsset,
		_: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
	) -> Result<Self, bevy::render::render_asset::PrepareAssetError<Self::SourceAsset>> {
		Ok(source_asset)
	}
}

/// Holds an [AnimatedLighting] handle and automatically inserts the output lightmap onto the entity.
#[derive(Component, Reflect, Debug, Clone, Default, PartialEq, Eq, Deref, DerefMut)]
#[reflect(Component, Default)]
pub struct AnimatedLightmap(pub Handle<AnimatedLighting>);

#[derive(Reflect, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimatedLightingType {
	/// 2D textures, if textures differ in size they will be stretched.
	Lightmap,
	/// 3D textures, if texture differ in size they will repeat, this allows for non-directional volumes to store a 6th of the required data.
	IrradianceVolume,
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
	lightmap_pipeline: CachedRenderPipelineId,
	irradiance_volume_pipeline: CachedRenderPipelineId,
	sampler: Sampler,
}
impl FromWorld for AnimatedLightingPipeline {
	fn from_world(world: &mut World) -> Self {
		let render_device = world.resource::<RenderDevice>();

		let lightmap_bind_group_layout = render_device.create_bind_group_layout(
			"LightmapCompositeImages",
			&BindGroupLayoutEntries::sequential(
				ShaderStages::FRAGMENT,
				(
					texture_2d(TextureSampleType::Float { filterable: false }),
					texture_2d(TextureSampleType::Float { filterable: false }),
					texture_2d(TextureSampleType::Float { filterable: false }),
					texture_2d(TextureSampleType::Float { filterable: false }),
					texture_2d(TextureSampleType::Uint),
					sampler(SamplerBindingType::NonFiltering),
					uniform_buffer_sized(false, Some(<[LightmapAnimator; MAX_ANIMATORS]>::SHADER_SIZE)),
				),
			),
		);

		let irradiance_volume_bind_group_layout = render_device.create_bind_group_layout(
			"IrradianceVolumeCompositeImages",
			&BindGroupLayoutEntries::sequential(
				ShaderStages::VERTEX_FRAGMENT,
				(
					texture_3d(TextureSampleType::Float { filterable: false }),
					texture_3d(TextureSampleType::Float { filterable: false }),
					texture_3d(TextureSampleType::Float { filterable: false }),
					texture_3d(TextureSampleType::Float { filterable: false }),
					texture_3d(TextureSampleType::Uint),
					uniform_buffer_sized(false, Some(UVec3::SHADER_SIZE)),
					uniform_buffer_sized(false, Some(<[LightmapAnimator; MAX_ANIMATORS]>::SHADER_SIZE)),
				),
			),
		);

		let globals_bind_group_layout = render_device.create_bind_group_layout(
			None,
			&BindGroupLayoutEntries::single(ShaderStages::FRAGMENT, uniform_buffer_sized(false, Some(GlobalsUniform::SHADER_SIZE))),
		);

		let pipeline_cache = world.resource::<PipelineCache>();

		// For things shared across both pipelines
		fn create_pipeline_descriptor(
			shader: Handle<Shader>,
			label: &'static str,
			bind_group_layout: BindGroupLayout,
			globals_bind_group_layout: BindGroupLayout,
		) -> RenderPipelineDescriptor {
			RenderPipelineDescriptor {
				label: Some(label.into()),
				layout: vec![bind_group_layout, globals_bind_group_layout],
				push_constant_ranges: vec![],
				vertex: VertexState {
					shader: shader.clone(),
					shader_defs: vec![],
					entry_point: "vertex".into(),
					buffers: vec![],
				},
				primitive: PrimitiveState {
					topology: PrimitiveTopology::TriangleStrip,
					strip_index_format: None,
					front_face: FrontFace::Cw,
					cull_mode: None,
					unclipped_depth: false,
					polygon_mode: PolygonMode::Fill,
					conservative: false,
				},
				depth_stencil: None,
				multisample: default(), // Do not multisample
				fragment: Some(FragmentState {
					shader,
					shader_defs: vec![],
					entry_point: "fragment".into(),
					targets: vec![Some(LIGHTMAP_OUTPUT_TEXTURE_FORMAT.into())],
				}),
				zero_initialize_workgroup_memory: true,
			}
		}

		let lightmap_pipeline = pipeline_cache.queue_render_pipeline(create_pipeline_descriptor(
			world.load_asset("embedded://bevy_trenchbroom/bsp/composite_lightmaps.wgsl"),
			"Composite Lightmap Images Pipeline",
			lightmap_bind_group_layout.clone(),
			globals_bind_group_layout.clone(),
		));
		let irradiance_volume_pipeline = pipeline_cache.queue_render_pipeline(create_pipeline_descriptor(
			world.load_asset("embedded://bevy_trenchbroom/bsp/composite_irradiance_volumes.wgsl"),
			"Composite Irradiance Volume Images Pipeline",
			irradiance_volume_bind_group_layout.clone(),
			globals_bind_group_layout.clone(),
		));

		let sampler = render_device.create_sampler(&SamplerDescriptor {
			mag_filter: FilterMode::Nearest,
			min_filter: FilterMode::Nearest,
			..default()
		});

		Self {
			lightmap_bind_group_layout,
			irradiance_volume_bind_group_layout,
			globals_bind_group_layout,
			lightmap_pipeline,
			irradiance_volume_pipeline,
			sampler,
		}
	}
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct AnimatedLightingLabel;

// TODO Would a compute shader give more performance? Should we care?
//      It does seem like vkQuake uses a compute shader, and it does get more performance
//      We also need to think about compatibility, currently, wgpu doesn't support compute shaders on the web

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

		for (id, bind_group) in &bind_groups.values {
			let Some(animated_lighting) = animated_lighting_assets.get(*id) else { continue };
			let Some(output_image) = gpu_images.get(&animated_lighting.output) else { continue };
			// TODO if there is only unanimated styles, and it's already run once, we don't need to run it again!

			let mut pass = render_context.command_encoder().begin_render_pass(&RenderPassDescriptor {
				label: Some("Composite Lighting"),
				color_attachments: &[Some(RenderPassColorAttachment {
					view: &output_image.texture_view,
					resolve_target: None,
					ops: Operations {
						load: LoadOp::Clear(LinearRgba::rgb(1., 0., 1.).into()), // Obvious error color
						// load: LoadOp::Load,
						store: StoreOp::Store,
					},
				})],
				..default()
			});

			let pipeline_id = match animated_lighting.ty {
				AnimatedLightingType::Lightmap => pipeline.lightmap_pipeline,
				AnimatedLightingType::IrradianceVolume => pipeline.irradiance_volume_pipeline,
			};
			let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline_id) else { return Ok(()) };
			pass.set_pipeline(render_pipeline);

			pass.set_bind_group(0, bind_group, &[]);
			pass.set_bind_group(1, globals_bind_group, &[]);

			match animated_lighting.ty {
				AnimatedLightingType::Lightmap => pass.draw(0..4, 0..1),
				// AnimatedLightingType::IrradianceVolume => pass.draw(0..4 * output_image.size.depth_or_array_layers, 0..1),
				AnimatedLightingType::IrradianceVolume => todo!(),
			}
		}

		Ok(())
	}
}
