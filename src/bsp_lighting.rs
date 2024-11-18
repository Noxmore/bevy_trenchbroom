use std::{num::{NonZeroU32, NonZeroU64}, ops::Deref};

use bevy::{asset::embedded_asset, render::{extract_resource::{ExtractResource, ExtractResourcePlugin}, mesh::PrimitiveTopology, render_asset::{RenderAsset, RenderAssetPlugin, RenderAssetUsages, RenderAssets}, render_graph::{RenderGraph, RenderLabel}, render_resource::{binding_types::*, *}, renderer::RenderDevice, texture::{GpuImage, ImageSampler}, Render, RenderApp, RenderSet}};

use crate::*;

pub struct BspLightingPlugin;
impl Plugin for BspLightingPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "composite_lightmaps.wgsl");
        
        app
            .add_plugins(RenderAssetPlugin::<AnimatedLightmap>::default())
            // .add_plugins(RenderAssetPlugin::<ExtractedAnimatedLightmap>::default())
            // .add_plugins(ExtractResourcePlugin::<ExtractedAnimatedLightmaps>::default())
            // .add_plugins(ExtractResourcePlugin::<AnimatedLightmapPipeline>::default())
        
            .init_asset::<AnimatedLightmap>()

            // .add_systems(Update, Self::animate_lightmaps)
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
    ) {
        if !animated_lightmaps.is_changed() { return }
        bind_groups.values.clear();
        // WgpuFeatures::BUFF
        
        for (id, animated_lightmap) in animated_lightmaps.iter() {
            // let input_texture_view = 
            // let texture_views = animated_lightmap.images.iter().flat_map(|(_, image)| gpu_images.get(image)).map(|gpu_image| gpu_image.texture_view.deref()).collect_vec();
            bind_groups.values.insert(id, render_device.create_bind_group(None, &pipeline.texture_bind_group_layout, &[
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
                        contents: &animated_lightmap.layers.to_ne_bytes(),
                        usage: BufferUsages::UNIFORM,
                    }).as_entire_buffer_binding())
                }
            ]));
        }
    }
}

pub(crate) fn new_lightmap_output_image(width: u32, height: u32) -> Image {
    let mut image = Image::new_fill(
        Extent3d { width, height, ..default() },
        TextureDimension::D2,
        // &[0, 0, 0, 255],
        [0.0_f32, 1., 0., 1.].map(|f| f.to_ne_bytes()).as_flattened(),
        // TextureFormat::Rgba8Unorm,
        TextureFormat::Rgba32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT; // We need to render to this from a shader
    image.sampler = ImageSampler::linear();
    image
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

    /// How many layers 
    pub layers: u32,
    
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

/* #[derive(Resource)]
pub struct ExtractedAnimatedLightmaps {
    pub values: HashMap<AssetId<AnimatedLightmap>, AnimatedLightmap>,
}
impl ExtractResource for ExtractedAnimatedLightmaps {
    type Source = Assets<AnimatedLightmap>;
    fn extract_resource(source: &Self::Source) -> Self {
        Self { values: source.iter().map(|(id, animated_lightmap)| (id, animated_lightmap.clone())).collect() }
    }
} */

#[derive(Resource, Default)]
pub struct AnimatedLightmapBindGroups {
    pub values: HashMap<AssetId<AnimatedLightmap>, BindGroup>,
}


/* pub struct ExtractedAnimatedLightmap {
    pub animated_lightmap: AnimatedLightmap,
    pub bind_group: BindGroup,
}
impl RenderAsset for ExtractedAnimatedLightmap {
    type SourceAsset = AnimatedLightmap;
    type Param = (
        SRes<RenderDevice>,
    );
    
    fn prepare_asset(
        source_asset: Self::SourceAsset,
        (render_device): &mut bevy::ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<Self, bevy::render::render_asset::PrepareAssetError<Self::SourceAsset>> {
        todo!()
    }
} */

/* #[derive(AsBindGroup)]
struct Foo {
    #[texture(0)]
    #[sampler(1)]
    textures: Vec<Handle<Image>>,
    // #[sampler(1)]
    // sampler: Sampler,
} */


#[derive(Resource, ExtractResource, Clone)]
pub struct AnimatedLightmapPipeline {
    texture_bind_group_layout: BindGroupLayout,
    pipeline: CachedRenderPipelineId,
    sampler: Sampler,
    // tmp_vertex_buffer: Buffer,
}
impl FromWorld for AnimatedLightmapPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        
        let texture_bind_group_layout = render_device.create_bind_group_layout("LightmapCompositeImages", &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_3d(TextureSampleType::Float { filterable: false }),
                // texture_3d(TextureSampleType::Float { filterable: false }).count(NonZeroU32::new(254).unwrap()),
                sampler(SamplerBindingType::NonFiltering),
                uniform_buffer_sized(false, Some(NonZeroU64::new(size_of::<u32>() as u64).unwrap())),
                // texture_storage_2d_array(TextureFormat::Rgba8Unorm, StorageTextureAccess::ReadOnly).count(NonZeroU32::new(254).unwrap()),
                // texture_storage_2d(TextureFormat::Rgba32Float, StorageTextureAccess::WriteOnly),
            ),
        ));

        let shader = world.load_asset("embedded://bevy_trenchbroom/composite_lightmaps.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();

        let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("Composite Lightmap Images Pipeline".into()),
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: vec![],
                // buffers: vec![VertexBufferLayout::from_vertex_formats(VertexStepMode::Vertex, [VertexFormat::Float32x2])], // TODO tmp
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

        Self { texture_bind_group_layout, pipeline, sampler }
    }
}


#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct AnimatedLightmapsLabel;

// TODO Would a compute shader give more performance? Should we care?

pub struct AnimatedLightmapsNode;

impl bevy::render::render_graph::Node for AnimatedLightmapsNode {
    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let bind_groups = world.resource::<AnimatedLightmapBindGroups>();
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
            // println!("here");
            // StandardMaterial

            pass.set_bind_group(0, bind_group, &[]);
            // pass.set_vertex_buffer(0, *pipeline.tmp_vertex_buffer.slice(..));
            pass.draw(0..4, 0..1);
        }
            
        Ok(())
    }
}