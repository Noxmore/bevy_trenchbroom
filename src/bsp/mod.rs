pub mod lighting;
pub mod util;

use bevy::{asset::{AssetLoader, LoadContext}, image::ImageSampler, render::{mesh::{Indices, PrimitiveTopology}, render_asset::RenderAssetUsages, render_resource::{Extent3d, TextureDimension, TextureFormat}}};
use class::ErasedQuakeClass;
use config::{EmbeddedTextureLoadView, TextureLoadView};
use geometry::{GeometryProviderMeshView, GeometryProviderView, MapGeometryTexture};
use lighting::{new_lightmap_output_image, AnimatedLighting, AnimatedLightingType};
use q1bsp::{data::{bsp::BspTexFlags, bspx::LightGridCell}, mesh::lighting::ComputeLightmapAtlasError};
use qmap::{QuakeMapEntities, QuakeMapEntity};
use util::IrradianceVolumeBuilder;

use crate::*;

pub static GENERIC_MATERIAL_PREFIX: &str = "GenericMaterial_";
pub static TEXTURE_PREFIX: &str = "Texture_";

#[derive(Asset, Reflect, Debug)]
pub struct Bsp {
    pub scene: Handle<Scene>,
    #[reflect(ignore)]
    pub embedded_textures: HashMap<String, BspEmbeddedTexture>,
    pub lightmap: Option<Handle<AnimatedLighting>>,
    pub irradiance_volume: Option<Handle<AnimatedLighting>>,
    /// Models for brush entities (world geometry).
    pub models: Vec<BspModel>,
    // TODO
    // pub entity_brushes: Vec<Handle<BrushList>>,

    pub data: Handle<BspDataAsset>,
    pub entities: QuakeMapEntities,
}

/// Store [BspData] in an asset so that it can be easily referenced from other places without referencing the [Bsp] (such as in the [Bsp]'s scene).
#[derive(Asset, TypePath, Debug, Clone, Default)]
pub struct BspDataAsset(pub BspData);

#[derive(Reflect, Debug)]
pub struct BspModel {
    /// TODO doc Textures and meshes
    /// TODO store texture flags?
    pub meshes: Vec<(String, Handle<Mesh>)>,
}

/// A reference to a texture loaded from a BSP file. Stores the handle to the image, and the alpha mode that'll work for said image for performance reasons.
#[derive(Debug)]
pub struct BspEmbeddedTexture {
    pub image: Handle<Image>,
    pub material: Handle<GenericMaterial>,
}

pub struct BspLoader {
    pub tb_server: TrenchBroomServer,
    pub asset_server: AssetServer,
}
impl FromWorld for BspLoader {
    fn from_world(world: &mut World) -> Self {
        Self {
            tb_server: world.resource::<TrenchBroomServer>().clone(),
            asset_server: world.resource::<AssetServer>().clone(),
        }
    }
}

impl AssetLoader for BspLoader {
    type Asset = Bsp;
    type Error = anyhow::Error;
    type Settings = ();

    fn load(
        &self,
        reader: &mut dyn bevy::asset::io::Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext,
    ) -> impl bevy::utils::ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            // TODO split this up into smaller functions, maybe with a context?

            let lit = load_context.read_asset_bytes(load_context.path().with_extension("lit")).await.ok();
            
            let data = BspData::parse(BspParseInput { bsp: &bytes, lit: lit.as_ref().map(Vec::as_slice) })?;

            let quake_util_map = quake_util::qmap::parse(&mut io::Cursor::new(data.entities.as_bytes()))
                .map_err(|err| anyhow!("Parsing entities: {err}"))?;
            let entities = QuakeMapEntities::from_quake_util(quake_util_map, &self.tb_server.config);

            // Need to store this separately for animation.
            // We can't use the `next` animation property because we need the handle to create the assets to create the handles.
            let embedded_texture_images: HashMap<&str, (Image, Handle<Image>)> = data.textures.iter().flatten().filter(|texture| texture.data.is_some()).map(|texture| {
                    let Some(data) = &texture.data else { unreachable!() };
                    let name = texture.header.name.as_str();

                    let is_cutout_texture = name.starts_with('{');

                    let image = Image::new(
                        Extent3d { width: texture.header.width, height: texture.header.height, ..default() },
                        TextureDimension::D2,
                        data.iter().copied().map(|pixel| {
                            if self.tb_server.config.special_textures.is_some() && is_cutout_texture && pixel == 255 {
                                [0; 4]
                            } else {
                                let [r, g, b] = self.tb_server.config.texture_pallette.1.colors[pixel as usize];
                                [r, g, b, 255]
                            }
                        }).flatten().collect(),
                        TextureFormat::Rgba8UnormSrgb,
                        self.tb_server.config.bsp_textures_asset_usages,
                    );
                    
                    let image_handle = load_context.add_labeled_asset(format!("{TEXTURE_PREFIX}{name}"), image.clone());

                    (texture.header.name.as_str(), (image, image_handle))
                })
                .collect();

            let embedded_textures: HashMap<String, BspEmbeddedTexture> = embedded_texture_images.iter()
                .map(|(name, (image, image_handle))| {
                    let is_cutout_texture = name.chars().next() == Some('{');

                    let material = (self.tb_server.config.load_embedded_texture)(EmbeddedTextureLoadView {
                        parent_view: TextureLoadView {
                            name,
                            tb_config: &self.tb_server.config,
                            load_context,
                            entities: &entities,
                            alpha_mode: is_cutout_texture.then_some(AlphaMode::Mask(0.5)),
                            embedded_textures: Some(&embedded_texture_images),
                        },

                        image_handle,
                        image,
                    });

                    (name.to_string(), BspEmbeddedTexture { image: image_handle.clone(), material })
                })
                .collect();

            let lightmap = match data.compute_lightmap_atlas(self.tb_server.config.compute_lightmap_settings, LightmapAtlasType::PerSlot) {
                Ok(atlas) => {
                    let size = atlas.data.size();
                    let LightmapAtlasData::PerSlot { slots, styles } = atlas.data else {
                        unreachable!()
                    };
                    
                    // TODO tmp
                    const WRITE_DEBUG_FILES: bool = false;
                    
                    if WRITE_DEBUG_FILES {
                        fs::create_dir("target/lightmaps").ok();
                        for (i, image) in slots.iter().enumerate() {
                            image.save_with_format(format!("target/lightmaps/{i}.png"), image::ImageFormat::Png).ok();
                        }
                        styles.save_with_format(format!("target/lightmaps/styles.png"), image::ImageFormat::Png).ok();
                    }
                    
                    let output = load_context.add_labeled_asset("LightmapOutput".s(), new_lightmap_output_image(size.x, size.y));
                    
                    let mut i = 0;
                    let input = slots.map(|image| {
                        let handle = load_context.add_labeled_asset(format!("LightmapInput{i}"), Image::new(
                            Extent3d { width: image.width(), height: image.height(), ..default() },
                            TextureDimension::D2,
                            image.pixels().map(|pixel| {
                                [pixel[0], pixel[1], pixel[2], 255]
                            }).flatten().collect(),
                            // Without Srgb all the colors are washed out, so i'm guessing ericw-tools outputs sRGB, though i can't find it documented anywhere.
                            TextureFormat::Rgba8UnormSrgb,
                            self.tb_server.config.bsp_textures_asset_usages,
                        ));

                        i += 1;
                        handle
                    });

                    let styles = load_context.add_labeled_asset("LightmapStyles".s(), Image::new(
                        Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
                        TextureDimension::D2,
                        styles.into_vec(),
                        TextureFormat::Rgba8Uint,
                        RenderAssetUsages::RENDER_WORLD,
                    ));

                    let handle = load_context.add_labeled_asset("LightmapAnimator".s(), AnimatedLighting {
                        ty: AnimatedLightingType::Lightmap,
                        output,
                        input,
                        styles,
                    });

                    Some((handle, atlas.uvs))
                },
                Err(ComputeLightmapAtlasError::NoLightmaps) => None,
                Err(err) => return Err(anyhow::anyhow!(err)),
            };
            
            let mut world = World::new();

            // Load models into Bevy format
            #[derive(Default)]
            struct Model {
                meshes: Vec<ModelMesh>,
            }

            struct ModelMesh {
                texture: MapGeometryTexture,
                mesh: Mesh,
                /// Entity to apply Mesh3d to. Should probably only be one of these.
                entity: Option<Entity>,
            }

            let mut models = Vec::with_capacity(data.models.len());

            for model_idx in 0..data.models.len() {
                let model_output = data.mesh_model(model_idx, lightmap.as_ref().map(|(_, uvs)| uvs));
                let mut model = Model::default();
                
                for exported_mesh in model_output.meshes {
                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());

                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, exported_mesh.positions.into_iter().map(convert_vec3(&self.tb_server)).collect_vec());
                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, exported_mesh.normals.into_iter().map(convert_vec3(&self.tb_server)).collect_vec());
                    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, exported_mesh.uvs.iter().map(q1bsp::glam::Vec2::to_array).collect_vec());
                    if let Some(lightmap_uvs) = &exported_mesh.lightmap_uvs {
                        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, lightmap_uvs.iter().map(q1bsp::glam::Vec2::to_array).collect_vec());
                    }
                    mesh.insert_indices(Indices::U32(exported_mesh.indices.into_flattened()));
                    
                    let material = embedded_textures.get(&exported_mesh.texture)
                        .map(|texture| texture.material.clone())
                        .unwrap_or_else(|| (self.tb_server.config.load_loose_texture)(TextureLoadView {
                            name: &exported_mesh.texture,
                            tb_config: &self.tb_server.config,
                            load_context,
                            entities: &entities,
                            alpha_mode: None,
                            embedded_textures: Some(&embedded_texture_images),
                        }));

                    model.meshes.push(ModelMesh {
                        texture: MapGeometryTexture {
                            material,
                            lightmap: lightmap.as_ref().map(|(handle, _)| handle.clone()),
                            name: exported_mesh.texture,
                            // TODO this makes some things pitch black maybe?
                            special: exported_mesh.tex_flags != BspTexFlags::Normal,
                        },
                        mesh,
                        entity: None,
                    });
                }

                models.push(model);
            }

            // We need this here while we still have access to data for later
            let light_grid_octree = data.bspx.parse_light_grid_octree(&data.parse_ctx);
            
            // So we can access the handle in the scene
            let data_handle = load_context.add_labeled_asset("BspData".s(), BspDataAsset(data));
            
            // Spawn entities into scene
            for (map_entity_idx, map_entity) in entities.iter().enumerate() {
                let Some(classname) = map_entity.properties.get("classname") else { continue };
                let Some(class) = self.tb_server.config.get_class(classname) else {
                    if !self.tb_server.config.ignore_invalid_entity_definitions {
                        error!("No class found for classname `{classname}` on entity {map_entity_idx}");
                    }
                    
                    continue;
                };

                let mut entity = world.spawn_empty();
                let entity_id = entity.id();
                
                class.apply_spawn_fn_recursive(&self.tb_server.config, map_entity, &mut entity)
                    .map_err(|err| anyhow!("spawning entity {map_entity_idx} ({classname}): {err}"))?;
                
                if let Some(geometry_provider) = (class.geometry_provider_fn)(map_entity) {
                    if let Some(model_idx) = get_model_idx(map_entity, class) {
                        let model = models.get_mut(model_idx).ok_or_else(|| anyhow!("invalid model index {model_idx}"))?;
                        let mut meshes = Vec::with_capacity(model.meshes.len());
                        
                        for model_mesh in &mut model.meshes {
                            if self.tb_server.config.auto_remove_textures.contains(&model_mesh.texture.name) {
                                continue;
                            }
                            
                            let mesh_entity = world.spawn(Name::new(model_mesh.texture.name.clone())).id();
                            world.entity_mut(entity_id).add_child(mesh_entity);
                            
                            meshes.push(GeometryProviderMeshView {
                                entity: mesh_entity,
                                mesh: &mut model_mesh.mesh,
                                texture: &mut model_mesh.texture,
                            });

                            if model_mesh.entity.is_some() {
                                error!("Map entity {map_entity_idx} ({}) points to model {model_idx}, but it has already been used by a different entity. Make an issue because i thought this wasn't possible!", class.info.name);
                            }
                            model_mesh.entity = Some(mesh_entity);
                        }

                        let mut view = GeometryProviderView {
                            world: &mut world,
                            entity: entity_id,
                            tb_server: &self.tb_server,
                            map_entity,
                            map_entity_idx,
                            meshes,
                            load_context,
                        };
                        
                        for provider in geometry_provider.providers {
                            provider(&mut view);
                        }

                        (self.tb_server.config.global_geometry_provider)(&mut view);
                    }
                }

                let mut entity = world.entity_mut(entity_id);

                (self.tb_server.config.global_spawner)(&self.tb_server.config, map_entity, &mut entity)
                    .map_err(|err| anyhow!("spawning entity {map_entity_idx} ({classname}) with global spawner: {err}"))?;
            }

            let bsp_models = models.into_iter().enumerate().map(|(model_idx, model)| {
                BspModel {
                    meshes: model.meshes.into_iter().enumerate().map(|(mesh_idx, model_mesh)| {
                        let mesh_handle = load_context.add_labeled_asset(format!("Model{model_idx}Mesh{mesh_idx}"), model_mesh.mesh);

                        if let Some(mesh_entity) = model_mesh.entity {
                            world.entity_mut(mesh_entity).insert(Mesh3d(mesh_handle.clone()));
                        }

                        (model_mesh.texture.name, mesh_handle)
                    }).collect(),
                }
            }).collect();

            // TODO
            let irradiance_volume = None;
            
            // Calculate irradiance volumes for light grids.
            // Right now we just have one big irradiance volume for the entire map, this means the volume has to be less than 682 (2048/3 (z axis is 3x)) cells in size.
            if let Some(light_grid) = light_grid_octree {
                let mut light_grid = light_grid.map_err(io::Error::other)?;
                light_grid.mins = self.tb_server.config.to_bevy_space(light_grid.mins.to_array().into()).to_array().into();
                // We add 1 to the size because the volume has to be offset by half a step to line up, and as such sometimes doesn't fill the full space
                light_grid.size = light_grid.size.xzy() + 1;
                light_grid.step = self.tb_server.config.to_bevy_space(light_grid.step.to_array().into()).to_array().into();

                let mut builder = IrradianceVolumeBuilder::new(light_grid.size.to_array(), [0, 0, 0, 255], self.tb_server.config.irradiance_volume_multipliers);
                
                for mut leaf in light_grid.leafs {
                    leaf.mins = leaf.mins.xzy();
                    let size = leaf.size().xzy();
                    
                    for x in 0..size.x {
                        for y in 0..size.y {
                            for z in 0..size.z {
                                let LightGridCell::Filled(samples) = leaf.get_cell(x, z, y) else { continue };
                                let mut color: [u8; 4] = [0, 0, 0, 255];

                                for sample in samples {
                                    // println!("{sample:?}");
                                    for i in 0..3 {
                                        color[i] = color[i].saturating_add(sample.color[i]);
                                    }
                                }
                                
                                let (dst_x, dst_y, dst_z) = (x + leaf.mins.x, y + leaf.mins.y, z + leaf.mins.z);
                                builder.put_all([dst_x, dst_y, dst_z], color);
                            }
                        }
                    }
                }

                // This is pretty much instructed by FTE docs
                builder.flood_non_filled();

                let mut image = builder.build();
                image.sampler = ImageSampler::linear();

                let image_handle = load_context.add_labeled_asset("IrradianceVolume".s(), image);

                // TODO animated irradiance volumes
                // let animated_lighting_handle = load_context.add_labeled_asset("LightmapAnimator".s(), AnimatedLighting {
                //     ty: AnimatedLightingType::Lightmap,
                //     output: image_handle.clone(),
                //     input: [Handle::default(), Handle::default(), Handle::default(), Handle::default()],
                //     styles: Handle::default(),
                // });

                let mins: Vec3 = light_grid.mins.to_array().into();
                let scale: Vec3 = (light_grid.size.as_vec3() * light_grid.step).to_array().into();

                world.spawn((
                    Name::new("Light Grid Irradiance Volume"),
                    LightProbe,
                    IrradianceVolume { 
                        voxels: image_handle,
                        intensity: self.tb_server.config.default_irradiance_volume_intensity,
                    },
                    Transform {
                    translation: mins + scale / 2. - Vec3::from_array(light_grid.step.to_array()) / 2.,
                    scale,
                    ..default()
                }));

                // irradiance_volume = Some(animated_lighting_handle);
            }

            Ok(Bsp {
                scene: load_context.add_labeled_asset("Scene".s(), Scene::new(world)),
                embedded_textures,
                lightmap: lightmap.map(|(handle, _)| handle),
                irradiance_volume,
                models: bsp_models,

                data: data_handle,
                entities,
            })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["bsp"]
    }
}

#[inline]
fn convert_vec3<'a>(server: &'a TrenchBroomServer) -> impl Fn(q1bsp::glam::Vec3) -> Vec3 + 'a {
    |x| server.config.to_bevy_space(Vec3::from_array(x.to_array()))
}

fn get_model_idx(map_entity: &QuakeMapEntity, class: &ErasedQuakeClass) -> Option<usize> {
    // Worldspawn always has model 0
    if class.info.name == "worldspawn" { return Some(0) }
    
    let model_property = map_entity.get::<String>("model").ok()?;
    let model_property_trimmed = model_property.trim_start_matches('*');
    // If there wasn't a * at the start, this is invalid
    if model_property_trimmed == model_property { return None }
    model_property_trimmed.parse::<usize>().ok()
}

/* #[test]
fn bsp_loading() {
    let mut app = App::new();

    app
        .add_plugins((AssetPlugin::default(), TaskPoolPlugin::default(), TrenchBroomPlugin::new(default())))
        .init_asset::<Map>()
        .init_asset::<Image>()
        .init_asset::<StandardMaterial>()
        .init_asset_loader::<BspLoader>()
    ;

    let bsp_handle = app.world().resource::<AssetServer>().load::<Map>("maps/example.bsp");
    
    for _ in 0..1000 {
        match app.world().resource::<AssetServer>().load_state(&bsp_handle) {
            bevy::asset::LoadState::Loaded => return,
            bevy::asset::LoadState::Failed(err) => panic!("{err}"),
            _ => std::thread::sleep(std::time::Duration::from_millis(5)),
        }
        
        app.update();
    }
    panic!("Bsp took longer than 5 seconds to load.");
} */