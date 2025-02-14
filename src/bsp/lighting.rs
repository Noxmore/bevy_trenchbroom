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
use ser::SerializeStruct;

use crate::*;

/// Max animators passed to a shader, must match value in `composite_lightmaps.wgsl`.
pub const MAX_ANIMATORS: usize = 255;
/// Max number of steps in a [`LightmapAnimator`].
pub const MAX_LIGHTMAP_FRAMES: usize = 64;

const COMPUTE_WORKGROUP_SIZE: u32 = 4;

/// The format used for the out channel in lighting animation. I've found that [`TextureFormat::Rgba8UnormSrgb`] and the like show noticeable banding on slower animations.
pub(crate) const LIGHTMAP_OUTPUT_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba32Float;
pub(crate) fn new_animated_lighting_output_image(extent: Extent3d, dimension: TextureDimension) -> Image {
	let mut image = Image::new_fill(
		extent,
		dimension,
		// Bright color -- easy to spot errors
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
			.add_plugins(ExtractResourcePlugin::<IrradianceVolumeDepths>::default())
			.init_resource::<IrradianceVolumeDepths>()

			.register_type::<AnimatedLightingHandle>()

			.add_plugins(ExtractResourcePlugin::<LightmapAnimators>::default())
			.register_type::<LightmapAnimators>()
			.init_resource::<LightmapAnimators>()

			.init_asset::<AnimatedLighting>()

			.add_systems(PreUpdate, (
				Self::insert_animated_lightmaps,
				Self::populate_irradiance_volume_depths,
			))
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
	/// Inserts [`Lightmap`] components into entities with [`AnimatedLightmap`].
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
					});
				}
				AnimatedLightingType::IrradianceVolume => {
					commands.entity(entity).insert(IrradianceVolume {
						voxels: animated_lighting.output.clone(),
						intensity: tb_server.config.default_irradiance_volume_intensity,
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

	fn populate_irradiance_volume_depths(
		mut depths: ResMut<IrradianceVolumeDepths>,
		animated_lighting_assets: Res<Assets<AnimatedLighting>>,
		images: Res<Assets<Image>>,
	) {
		for (id, animated_lighting) in animated_lighting_assets.iter() {
			if animated_lighting.ty != AnimatedLightingType::IrradianceVolume {
				continue;
			}
			if depths.values.contains_key(&id) {
				continue;
			}
			let Some(image) = images.get(&animated_lighting.styles) else { continue };

			depths.values.insert(id, image.texture_descriptor.size.depth_or_array_layers);
		}
	}
}

/// TODO: Workaround for [`GpuImage`] not storing depth until 0.16
#[derive(Resource, ExtractResource, Clone, Default)]
struct IrradianceVolumeDepths {
	values: HashMap<AssetId<AnimatedLighting>, u32>,
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

	/// How much to linearly interpolate between elements in the sequence.
	///
	/// 0 swaps between them instantly, 1 smoothly interpolates,
	/// 0.5 interpolates to the next frame 2 times as quick, stopping in the middle, etc.
	pub interpolate: f32,
}
impl LightmapAnimator {
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
}
impl Default for LightmapAnimator {
	fn default() -> Self {
		Self::unanimated(Vec3::ONE)
	}
}
impl Serialize for LightmapAnimator {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let mut s = serializer.serialize_struct("LightmapAnimator", 3)?;

		s.serialize_field("sequence", &self.sequence[..usize::min(self.sequence_len as usize, MAX_LIGHTMAP_FRAMES)])?;

		s.serialize_field("speed", &self.speed)?;
		s.serialize_field("interpolate", &self.interpolate)?;

		s.end()
	}
}
// Holy boilerplate, Batman!
impl<'de> Deserialize<'de> for LightmapAnimator {
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
			type Value = LightmapAnimator;

			fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
				fmt.write_str("struct LightmapAnimator")
			}

			// rustfmt is threatening to make these look even more boilerplate-y then they already do.
			#[rustfmt::skip]
			fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
				let sequence_vec: Vec<Vec3> = seq.next_element()?.ok_or(de::Error::invalid_length(0, &"struct LightmapAnimator with 3 elements"))?;
				let speed: f32 = seq.next_element()?.ok_or(de::Error::invalid_length(1, &"struct LightmapAnimator with 3 elements"))?;
				let interpolate: f32 = seq.next_element()?.ok_or(de::Error::invalid_length(2, &"struct LightmapAnimator with 3 elements"))?;

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

		fn visit_internal<E: de::Error>(sequence_vec: Vec<Vec3>, speed: f32, interpolate: f32) -> Result<LightmapAnimator, E> {
			if sequence_vec.len() > MAX_LIGHTMAP_FRAMES {
				return Err(de::Error::custom(format_args!(
					"sequence has {} frames, but the max is {MAX_LIGHTMAP_FRAMES}",
					sequence_vec.len()
				)));
			}

			let mut sequence = [Vec3::ZERO; MAX_LIGHTMAP_FRAMES];
			sequence[..sequence_vec.len()].copy_from_slice(&sequence_vec);

			Ok(LightmapAnimator {
				sequence,
				sequence_len: sequence_vec.len() as u32,
				speed,
				interpolate,
			})
		}

		deserializer.deserialize_struct("LightmapAnimator", &["sequence", "speed", "interpolate"], Visitor)
	}
}

/// Resource that contains the current lightmap animators for each [`LightmapStyle`].
///
/// You can use this to change animations, and do things like toggle lights.
///
/// The default value somewhat mirrors some of Quake's animators.
#[derive(Resource, ExtractResource, Reflect, Debug, Clone, Serialize, Deserialize)]
#[reflect(Resource, Default, Serialize, Deserialize)]
pub struct LightmapAnimators {
	pub values: HashMap<LightmapStyle, LightmapAnimator>,
}
impl LightmapAnimators {
	/// Returns an empty animator map.
	pub fn none() -> Self {
		Self { values: default() }
	}
}

impl Default for LightmapAnimators {
	fn default() -> Self {
		Self {
			values: HashMap::from([
				(
					LightmapStyle(1),
					LightmapAnimator::new(6., 0.7, [0.8, 0.75, 1., 0.7, 0.8, 0.7, 0.9, 0.7, 0.6, 0.7, 0.9, 1., 0.7].map(Vec3::splat)),
				),
				(LightmapStyle(2), LightmapAnimator::new(0.5, 1., [0., 1.].map(Vec3::splat))),
			]),
		}
	}
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
		_: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
	) -> Result<Self, bevy::render::render_asset::PrepareAssetError<Self::SourceAsset>> {
		Ok(source_asset)
	}
}

/// Holds an [`AnimatedLighting`] handle and automatically inserts the output [`Lightmap`] or [`IrradianceVolume`] based on [`AnimatedLighting::ty`] onto the entity.
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
	irradiance_volume_pipeline: CachedComputePipelineId,
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
				ShaderStages::COMPUTE,
				(
					texture_3d(TextureSampleType::Float { filterable: false }),
					texture_3d(TextureSampleType::Float { filterable: false }),
					texture_3d(TextureSampleType::Float { filterable: false }),
					texture_3d(TextureSampleType::Float { filterable: false }),
					texture_3d(TextureSampleType::Uint),
					uniform_buffer_sized(false, Some(<[LightmapAnimator; MAX_ANIMATORS]>::SHADER_SIZE)),
					BindingType::StorageTexture {
						access: StorageTextureAccess::WriteOnly,
						format: LIGHTMAP_OUTPUT_TEXTURE_FORMAT,
						view_dimension: TextureViewDimension::D3,
					}
					.into_bind_group_layout_entry_builder(),
				),
			),
		);

		let globals_bind_group_layout = render_device.create_bind_group_layout(
			None,
			&BindGroupLayoutEntries::single(
				ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
				uniform_buffer_sized(false, Some(GlobalsUniform::SHADER_SIZE)),
			),
		);

		let pipeline_cache = world.resource::<PipelineCache>();

		let lightmap_pipeline = pipeline_cache.queue_render_pipeline({
			let shader = world.load_asset("embedded://bevy_trenchbroom/bsp/composite_lightmaps.wgsl");
			RenderPipelineDescriptor {
				label: Some("Composite Lightmap Images Pipeline".into()),
				layout: vec![lightmap_bind_group_layout.clone(), globals_bind_group_layout.clone()],
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
		});

		let irradiance_volume_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
			label: Some("Composite Irradiance Volume Images Pipeline".into()),
			layout: vec![irradiance_volume_bind_group_layout.clone(), globals_bind_group_layout.clone()],
			push_constant_ranges: vec![],
			shader: world.load_asset("embedded://bevy_trenchbroom/bsp/composite_irradiance_volumes.wgsl"),
			shader_defs: vec![],
			entry_point: "main".into(),
			zero_initialize_workgroup_memory: true,
		});

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
			match animated_lighting.ty {
				AnimatedLightingType::Lightmap => {
					let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.lightmap_pipeline) else { return Ok(()) };

					let mut pass = render_context.command_encoder().begin_render_pass(&RenderPassDescriptor {
						label: Some("Composite Lightmap"),
						color_attachments: &[Some(RenderPassColorAttachment {
							view: &output_image.texture_view,
							resolve_target: None,
							ops: Operations {
								load: LoadOp::Clear(LinearRgba::rgb(1., 0., 1.).into()), // Obvious error color
								store: StoreOp::Store,
							},
						})],
						..default()
					});

					pass.set_pipeline(render_pipeline);

					pass.set_bind_group(0, bind_group, &[]);
					pass.set_bind_group(1, globals_bind_group, &[]);

					pass.draw(0..4, 0..1);
				}

				AnimatedLightingType::IrradianceVolume => {
					let Some(depth) = world.resource::<IrradianceVolumeDepths>().values.get(id).copied() else { continue };
					let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline.irradiance_volume_pipeline) else { return Ok(()) };

					let mut pass = render_context.command_encoder().begin_compute_pass(&ComputePassDescriptor {
						label: Some("Composite Irradiance Volume"),
						..default()
					});

					pass.set_pipeline(pipeline);

					pass.set_bind_group(0, bind_group, &[]);
					pass.set_bind_group(1, globals_bind_group, &[]);

					// Add 1 as a hack to run on the entire image because integer division rounds down
					pass.dispatch_workgroups(
						output_image.size.x / COMPUTE_WORKGROUP_SIZE + 1,
						output_image.size.y / COMPUTE_WORKGROUP_SIZE + 1,
						depth / COMPUTE_WORKGROUP_SIZE + 1,
					);
				}
			}
		}

		Ok(())
	}
}
