use std::ops::Deref;

use bevy::{asset::embedded_asset, render::{extract_resource::{ExtractResource, ExtractResourcePlugin}, mesh::PrimitiveTopology, render_asset::{RenderAsset, RenderAssetPlugin, RenderAssetUsages, RenderAssets}, render_graph::{RenderGraph, RenderLabel}, render_resource::{binding_types::sampler, *}, renderer::RenderDevice, texture::{GpuImage, ImageSampler}, Render, RenderApp, RenderSet}};

use crate::*;

pub struct BspLightingPlugin;
impl Plugin for BspLightingPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "composite_lightmaps.wgsl");
        
        app
            // .add_plugins(RenderAssetPlugin::<AnimatedLightmap>::default())
            // .add_plugins(RenderAssetPlugin::<ExtractedAnimatedLightmap>::default())
            // .add_plugins(ExtractResourcePlugin::<ExtractedAnimatedLightmaps>::default())
            // .add_plugins(ExtractResourcePlugin::<AnimatedLightmapPipeline>::default())
        
            // .init_resource::<AnimatedLightmapPipeline>()
            .init_asset::<AnimatedLightmap>()

            .add_systems(Update, Self::animate_lightmaps)
        ;

        // let render_app = app.sub_app_mut(RenderApp);

        // render_app.init_resource::<AnimatedLightmapBindGroups>();

        // render_app.add_systems(Render, Self::prepare_animated_lightmaps_bind_groups.in_set(RenderSet::PrepareBindGroups));

        // let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        // render_graph.add_node(AnimatedLightmapsLabel, AnimatedLightmapsNode);
        // render_graph.add_node_edge(AnimatedLightmapsLabel, bevy::render::graph::CameraDriverLabel);
    }
}
impl BspLightingPlugin {
    pub fn animate_lightmaps(
        animated_lightmaps: Res<Assets<AnimatedLightmap>>,
        mut images: ResMut<Assets<Image>>,
        tb_server: Res<TrenchBroomServer>,
        time: Res<Time>,
    ) {
        for (animated_lightmap_id, animated_lightmap) in animated_lightmaps.iter() {
            // println!("{animated_lightmap_id}");
            let [width, height] = animated_lightmap.images.size().to_array();
            let image = images.get_or_insert_with(&animated_lightmap.output, || new_lightmap_image(width, height));

            // Reset image
            for i in 0..image.data.len() {
                if i % 4 != 3 {
                    image.data[i] = 0;
                }
            }

            for (style, atlas) in animated_lightmap.images.map() {
                // println!("{style:?} == {:?}: {}", LightmapStyle::NORMAL, *style == LightmapStyle::NORMAL);
                // if *style == LightmapStyle::NORMAL { continue }
                // let atlas = animated_lightmap.images.get(&LightmapStyle::NORMAL).unwrap();
                // let style = LightmapStyle::NORMAL;
                let Some(animator) = tb_server.config.lightmap_animators.get(style) else { continue };
                let animation_multiplier = animator.sample(time.elapsed_seconds());

                if atlas.width() != width || atlas.height() != height {
                    panic!("Not all lightmap atlas' are the same size for animated lightmap {animated_lightmap_id} style {style:?}");
                    // continue;
                    // TODO
                }
                
                // println!("here1");
                // image.data.copy_from_slice(&atlas);

                
                for (i, pixel) in atlas.pixels().copied().enumerate() {
                    let output_idx = i * 4;
                    // image.data[output_idx] = pixel[0];
                    // println!("here2");
    
                    // image.data[output_idx..output_idx + 3].copy_from_slice(&pixel.0);
                    // image.data[output_idx..output_idx + 3].copy_from_slice(&[255, 0, 0]);
                    for o in 0..3 {
                        image.data[output_idx + o] = image.data[output_idx + o].saturating_add((pixel[o] as f32 * animation_multiplier[o]) as u8);
                    }
                    // image.data[i] = image.data[i].saturating_add((byte as f32 * animator.sample(time.elapsed_seconds())[i % 3]) as u8);
                }
                // for byte in &mut image.data {
                //     *byte = 255;
                // }
            }
        }
    }

    /* pub fn prepare_animated_lightmaps_bind_groups(
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
            let texture_views = animated_lightmap.images.iter().flat_map(|(_, image)| gpu_images.get(image)).map(|gpu_image| gpu_image.texture_view.deref()).collect_vec();
            bind_groups.values.insert(id, render_device.create_bind_group(None, &pipeline.texture_bind_group_layout, &[
                BindGroupEntry {
                    binding: 0,
                    // resource: BindingResource::Buffer(BufferBinding:),
                    resource: BindingResource::TextureViewArray(&texture_views),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&pipeline.sampler),
                }
            ]));
        }
    } */
}

pub(crate) fn new_lightmap_image(width: u32, height: u32) -> Image {
    let mut image = Image::new_fill(
        Extent3d { width, height, ..default() },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
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
    pub output: Handle<Image>,
    pub images: Lightmaps,
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


#[derive(Resource, ExtractResource, Clone)]
pub struct AnimatedLightmapPipeline {
    texture_bind_group_layout: BindGroupLayout,
    pipeline: CachedRenderPipelineId,
    sampler: Sampler,
}
impl FromWorld for AnimatedLightmapPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = render_device.create_bind_group_layout("LightmapCompositeImages", &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                bevy::render::render_resource::binding_types::texture_2d_array(TextureSampleType::Float { filterable: false }),
                // texture_storage_2d_array(TextureFormat::Rgba8Unorm, StorageTextureAccess::ReadOnly).count(NonZeroU32::new(254).unwrap()),
                // texture_storage_2d(TextureFormat::Rgba32Float, StorageTextureAccess::WriteOnly),
                sampler(SamplerBindingType::NonFiltering),
            ),
        ));

        let shader = world.load_asset("embedded://bevy_trenchbroom/liquid.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();

        let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
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
            multisample: default(),
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

        Self { texture_bind_group_layout, pipeline, sampler }
    }
}


#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct AnimatedLightmapsLabel;

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


        let mut pass = render_context.command_encoder().begin_render_pass(&RenderPassDescriptor::default());
        
        pass.set_pipeline(pipeline_cache.get_render_pipeline(pipeline.pipeline).unwrap());
        // StandardMaterial

        for (_, bind_group) in &bind_groups.values {
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(0..4, 0..1);
        }
            
        Ok(())
    }
}