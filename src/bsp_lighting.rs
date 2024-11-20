use bevy::{asset::embedded_asset, render::{extract_resource::{ExtractResource, ExtractResourcePlugin}, mesh::PrimitiveTopology, render_asset::{RenderAsset, RenderAssetPlugin, RenderAssetUsages, RenderAssets}, render_graph::{RenderGraph, RenderLabel}, render_resource::{binding_types::*, *}, renderer::{RenderDevice, RenderQueue}, texture::{GpuImage, ImageSampler}, Render, RenderApp, RenderSet}};

use crate::*;

/// Max animators passed to a shader, must match value in `composite_lightmaps.wgsl`.
pub const MAX_ANIMATORS: usize = 254;
pub const MAX_LIGHTMAP_FRAMES: usize = 64;

pub struct BspLightingPlugin;
impl Plugin for BspLightingPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "composite_lightmaps.wgsl");
        
        app
            .add_plugins(RenderAssetPlugin::<AnimatedLightmap>::default())
            .add_plugins(ExtractResourcePlugin::<LightmapAnimators>::default())

            .init_resource::<LightmapAnimators>()
        
            .init_asset::<AnimatedLightmap>()
        ;

        let render_app = app.sub_app_mut(RenderApp);

        render_app.init_resource::<AnimatedLightmapBindGroups>();

        render_app.add_systems(Render, Self::prepare_animated_lightmaps_bind_groups.in_set(RenderSet::PrepareBindGroups));

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(AnimatedLightmapsLabel, AnimatedLightmapsNode);
        
        render_graph.add_node_edge(AnimatedLightmapsLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).init_resource::<AnimatedLightmapPipeline>();
    }
}
impl BspLightingPlugin {
    pub fn prepare_animated_lightmaps_bind_groups(
        animated_lightmaps: Res<RenderAssets<AnimatedLightmap>>,
        mut bind_groups: ResMut<AnimatedLightmapBindGroups>,
        gpu_images: Res<RenderAssets<GpuImage>>,
        render_device: Res<RenderDevice>,
        pipeline: Res<AnimatedLightmapPipeline>,
        animators: Res<LightmapAnimators>,
        render_queue: Res<RenderQueue>,
        time: Res<Time>,
    ) {
        let mut seconds_buffer = UniformBuffer::from(time.elapsed_seconds());
        seconds_buffer.write_buffer(&render_device, &render_queue);
        
        bind_groups.seconds = Some(render_device.create_bind_group(None, &pipeline.seconds_bind_group_layout, &[
            BindGroupEntry {
                binding: 0,
                resource: seconds_buffer.into_binding(),
            },
        ]));
        
        if !animated_lightmaps.is_changed() && !animators.is_changed() { return }
        bind_groups.values.clear();
        
        for (id, animated_lightmap) in animated_lightmaps.iter() {
            // let texture_views = animated_lightmap.images.iter().flat_map(|(_, image)| gpu_images.get(image)).map(|gpu_image| gpu_image.texture_view.deref()).collect_vec();

            let mut target_animators = [LightmapAnimator::default(); MAX_ANIMATORS];
            for (i, style) in animated_lightmap.styles.iter().enumerate() {
                target_animators[i] = animators.values.get(style).copied().unwrap_or_default();
            }
            let mut target_animators_buffer = UniformBuffer::from(target_animators);
            target_animators_buffer.set_label(Some("animators"));
            target_animators_buffer.write_buffer(&render_device, &render_queue);
            // assert_eq!(unsafe { any_as_byte_slice(&target_animators) }.len(), size_of::<[LightmapAnimator; MAX_ANIMATORS]>());
            
            bind_groups.values.insert(id, render_device.create_bind_group(None, &pipeline.bind_group_layout, &[
                BindGroupEntry {
                    binding: 0,
                    // resource: BindingResource::Buffer(BufferBinding:),
                    resource: BindingResource::TextureView(&gpu_images.get(&animated_lightmap.input).unwrap().texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Buffer(render_device.create_buffer_with_data(&BufferInitDescriptor {
                        label: Some("layers"),
                        contents: &(animated_lightmap.styles.len() as u32).to_ne_bytes(),
                        usage: BufferUsages::UNIFORM,
                    }).as_entire_buffer_binding())
                },
                BindGroupEntry {
                    binding: 3,
                    resource: target_animators_buffer.into_binding(),
                },
            ]));
        }
    }
}

pub(crate) fn new_lightmap_output_image(width: u32, height: u32) -> Image {
    let mut image = Image::new_fill(
        Extent3d { width, height, ..default() },
        TextureDimension::D2,
        [0.0_f32, 1., 0., 1.].map(|f| f.to_ne_bytes()).as_flattened(),
        TextureFormat::Rgba32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT; // We need to render to this from a shader
    image.sampler = ImageSampler::linear();
    image
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
        Self { values: HashMap::from([
            // TODO copy quake's default animators?
            (LightmapStyle(1), LightmapAnimator::new(4., true, [0.2, 0.7, 1., 0.5, 0.8, 0.4].map(Vec3::splat))),
            (LightmapStyle(2), LightmapAnimator::new(0.5, true, [0., 1.].map(Vec3::splat))),
        ]) }
    }
}

#[derive(Component)]
pub struct BspIrradianceVolume {
    pub owner: Handle<Map>,
}

/// Contains multiple lightmaps that are composited together in `output` using the current [TrenchBroomConfig]'s lightmap animators.
#[derive(Asset, TypePath, Clone)]
pub struct AnimatedLightmap {
    /// The image to output the composited lightmap atlas to.
    pub output: Handle<Image>,

    /// A 3-dimensional texture, each pixel along the z axis is a different lightmap atlas.
    pub input: Handle<Image>,

    /// The lightmap styles of each lightmap in `input`. The length of this also defines how many layers `input` has.
    pub styles: Vec<LightmapStyle>,
    
    // pub images: Lightmaps,
    // pub images: HashMap<LightmapStyle, Handle<Image>>,
}
impl RenderAsset for AnimatedLightmap {
    type SourceAsset = Self;
    type Param = ();
    
    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _: &mut bevy::ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<Self, bevy::render::render_asset::PrepareAssetError<Self::SourceAsset>> {
        Ok(source_asset)
    }
}


#[derive(Resource, Default)]
pub struct AnimatedLightmapBindGroups {
    pub values: HashMap<AssetId<AnimatedLightmap>, BindGroup>,
    pub seconds: Option<BindGroup>,
}


#[derive(Resource, ExtractResource, Clone)]
pub struct AnimatedLightmapPipeline {
    bind_group_layout: BindGroupLayout,
    // Time is split into another bind group, so i don't have to update the main bind group every frame.
    // Does this help with performance? No idea, i don't even know how to profile it.
    seconds_bind_group_layout: BindGroupLayout,
    pipeline: CachedRenderPipelineId,
    sampler: Sampler,
}
impl FromWorld for AnimatedLightmapPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        
        let bind_group_layout = render_device.create_bind_group_layout("LightmapCompositeImages", &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_3d(TextureSampleType::Float { filterable: false }),
                sampler(SamplerBindingType::NonFiltering),
                uniform_buffer_sized(false, Some(u32::SHADER_SIZE)),
                uniform_buffer_sized(false, Some(<[LightmapAnimator; MAX_ANIMATORS]>::SHADER_SIZE)),
            ),
        ));

        let seconds_bind_group_layout = render_device.create_bind_group_layout(None, &BindGroupLayoutEntries::single(
            ShaderStages::FRAGMENT,
            uniform_buffer_sized(false, Some(f32::SHADER_SIZE)),
        ));

        let shader = world.load_asset("embedded://bevy_trenchbroom/composite_lightmaps.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();

        let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("Composite Lightmap Images Pipeline".into()),
            layout: vec![bind_group_layout.clone(), seconds_bind_group_layout.clone()],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: vec![],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(IndexFormat::Uint16),
                front_face: FrontFace::Cw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: default(), // Do not multisample
            fragment: Some(FragmentState {
                shader,shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(TextureFormat::Rgba32Float.into())],
            }),
        });

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..default()
        });

        // let tmp_vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor { label: None, contents: [0.0_f32, 0.,  1., 0.,  0., 1.,  1., 1.].map(|f| f.to_ne_bytes()).as_flattened(), usage: BufferUsages::VERTEX });

        Self { bind_group_layout, seconds_bind_group_layout, pipeline, sampler }
    }
}


#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct AnimatedLightmapsLabel;

// TODO Would a compute shader give more performance? Should we care?
//      It does seem like vkQuake uses a compute shader, and it does get more performance
//      We also need to think about compatibility, currently, wgpu doesn't support compute shaders on the web

pub struct AnimatedLightmapsNode;

impl bevy::render::render_graph::Node for AnimatedLightmapsNode {
    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let bind_groups = world.resource::<AnimatedLightmapBindGroups>();
        let Some(seconds_bind_group) = &bind_groups.seconds else { return Ok(()) };
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<AnimatedLightmapPipeline>();
        let animated_lightmaps = world.resource::<RenderAssets<AnimatedLightmap>>();
        let gpu_images = world.resource::<RenderAssets<GpuImage>>();

        for (id, bind_group) in &bind_groups.values {
            let Some(animated_lightmap) = animated_lightmaps.get(*id) else { continue };
            let Some(output_image) = gpu_images.get(&animated_lightmap.output) else { continue };
            
            let mut pass = render_context.command_encoder().begin_render_pass(&RenderPassDescriptor {
                label: Some("Composite Lightmap Images"),
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
            
            let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline) else { return Ok(()) };
            pass.set_pipeline(render_pipeline);

            pass.set_bind_group(0, bind_group, &[]);
            pass.set_bind_group(1, seconds_bind_group, &[]);
            pass.draw(0..4, 0..1);
        }
            
        Ok(())
    }
}