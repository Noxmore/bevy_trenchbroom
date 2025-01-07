use bevy::{asset::{AssetLoader, LoadContext}, render::{mesh::{Indices, PrimitiveTopology}, render_asset::RenderAssetUsages, render_resource::{Extent3d, TextureDimension, TextureFormat}}};
use geometry::{GeometryProviderMeshView, GeometryProviderView, MapGeometryTexture};
use ndshape::{RuntimeShape, Shape};
use q1bsp::{data::bsp::BspTexFlags, mesh::lighting::ComputeLightmapAtlasError};
use qmap::QuakeMap;

use crate::*;

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

    #[reflect(ignore)]
    pub data: BspData,
    pub qmap: QuakeMap,
}

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
    pub type_registry: AppTypeRegistry,
}
impl FromWorld for BspLoader {
    fn from_world(world: &mut World) -> Self {
        Self {
            tb_server: world.resource::<TrenchBroomServer>().clone(),
            asset_server: world.resource::<AssetServer>().clone(),
            type_registry: world.resource::<AppTypeRegistry>().clone(),
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

            let quake_util_map = parse_qmap(data.entities.as_bytes()).map_err(io_add_msg!("Parsing entities"))?;
            let map = QuakeMap::from_quake_util(quake_util_map, &self.tb_server.config);

            let embedded_textures: HashMap<String, BspEmbeddedTexture> = data.parse_embedded_textures(self.tb_server.config.texture_pallette.1)
                .into_iter()
                .map(|(name, image)| {
                    // let image = rgb_image_to_bevy_image(&image, &self.server, self.server.config.special_textures.is_some() && name.chars().next() == Some('{'));
                    let image = Image::new(
                        Extent3d { width: image.width(), height: image.height(), ..default() },
                        TextureDimension::D2,
                        image.pixels().map(|pixel| {
                            if self.tb_server.config.special_textures.is_some() && name.chars().next() == Some('{') && pixel.0 == self.tb_server.config.texture_pallette.1.colors[255] {
                                [0; 4]
                            } else {
                                [pixel[0], pixel[1], pixel[2], 255]
                            }
                        }).flatten().collect(),
                        // Without Srgb all the colors are washed out, so i'm guessing ericw-tools outputs sRGB, though i can't find it documented anywhere.
                        TextureFormat::Rgba8UnormSrgb,
                        self.tb_server.config.bsp_textures_asset_usages,
                    );
                    
                    // TODO
                    let alpha_mode = alpha_mode_from_image(&image);
                    
                    let image_handle = load_context.add_labeled_asset(format!("Texture_{name}"), image);

                    let material = (self.tb_server.config.load_embedded_texture)(TextureLoadView {
                        name: &name,
                        type_registry: &self.type_registry,
                        tb_config: &self.tb_server.config,
                        load_context,
                        map: &map,
                    }, image_handle.clone());

                    (name, BspEmbeddedTexture { image: image_handle, material })
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
                            image.clone().save_with_format(format!("target/lightmaps/{i}.png"), image::ImageFormat::Png).ok();
                        }
                        styles.clone().save_with_format(format!("target/lightmaps/styles.png"), image::ImageFormat::Png).ok();
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
            
            let mut model_options = repeat_n((), data.models.len()).map(|_| None).collect_vec();
            let mut world = World::new();

            for (map_entity_idx, map_entity) in map.entities.iter().enumerate() {
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

                // Nesting hell, TODO fix this
                if let Some(geometry_provider) = (class.geometry_provider_fn)(map_entity) {
                    // TODO dumb worldspawn fix
                    if let Ok(model) = map_entity.get::<String>("model").or_else(|err| if class.info.name == "worldspawn" { Ok("*0".s()) } else { Err(err) }) {
                        let model_idx = model.trim_start_matches('*');
                        // If there wasn't a * at the start, this is invalid
                        if model_idx != model {
                            if let Ok(model_idx) = model_idx.parse::<usize>() {
                                let model_output = data.mesh_model(model_idx, lightmap.as_ref().map(|(_, uvs)| uvs));
                                let mut meshes = Vec::new();
                                
                                for exported_mesh in model_output.meshes {
                                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());

                                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, exported_mesh.positions.into_iter().map(convert_vec3(&self.tb_server)).collect_vec());
                                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, exported_mesh.normals.into_iter().map(convert_vec3(&self.tb_server)).collect_vec());
                                    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, exported_mesh.uvs.iter().map(q1bsp::glam::Vec2::to_array).collect_vec());
                                    if let Some(lightmap_uvs) = &exported_mesh.lightmap_uvs {
                                        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, lightmap_uvs.iter().map(q1bsp::glam::Vec2::to_array).collect_vec());
                                    }
                                    mesh.insert_indices(Indices::U32(exported_mesh.indices.into_flattened()));
                                    
                                    let mesh_entity = world.spawn_empty().id();
                                    world.entity_mut(entity_id).add_child(mesh_entity);
                                    
                                    let material = embedded_textures.get(&exported_mesh.texture)
                                        .map(|texture| texture.material.clone())
                                        .unwrap_or_else(|| (self.tb_server.config.load_loose_texture)(TextureLoadView {
                                            name: &exported_mesh.texture,
                                            type_registry: &self.type_registry,
                                            tb_config: &self.tb_server.config,
                                            load_context,
                                            map: &map,
                                        }));

                                    meshes.push(GeometryProviderMeshView {
                                        entity: mesh_entity,
                                        mesh,
                                        texture: MapGeometryTexture {
                                            material,
                                            lightmap: lightmap.as_ref().map(|(handle, _)| handle),
                                            name: exported_mesh.texture,
                                            // TODO this makes some things pitch black maybe?
                                            special: exported_mesh.tex_flags != BspTexFlags::Normal,
                                        },
                                    });
                                }

                                let mut view = GeometryProviderView {
                                    world: &mut world,
                                    entity: entity_id,
                                    tb_server: &self.tb_server,
                                    asset_server: &self.asset_server,
                                    map_entity,
                                    map_entity_idx,
                                    meshes,
                                    load_context,
                                };
                                
                                for provider in geometry_provider.providers {
                                    provider(&mut view);
                                }

                                (self.tb_server.config.global_geometry_provider)(&mut view);

                                *model_options.get_mut(model_idx).ok_or_else(|| anyhow::anyhow!("invalid model index {model_idx}"))? = Some(BspModel {
                                    meshes: view.meshes.into_iter().enumerate().map(|(mesh_idx, view)| {
                                        // TODO asset added via geometry providers?
                                        let mesh_handle = load_context.add_labeled_asset(format!("Model{model_idx}Mesh{mesh_idx}"), view.mesh);
                                        (view.texture.name, mesh_handle)
                                    }).collect(),
                                });
                            }
                        }
                    }
                }

                let mut entity = world.entity_mut(entity_id);

                (self.tb_server.config.global_spawner)(&self.tb_server.config, map_entity, &mut entity)
                    .map_err(|err| anyhow!("spawning entity {map_entity_idx} ({classname}) with global spawner: {err}"))?;
            }

            let mut models = Vec::with_capacity(model_options.len());

            for (model_idx, model) in model_options.into_iter().enumerate() {
                match model {
                    Some(model) => models.push(model),
                    None => return Err(anyhow::anyhow!("model {model_idx} not used by any entities")),
                }
            }

            Ok(Bsp {
                scene: load_context.add_labeled_asset("Scene".s(), Scene::new(world)),
                embedded_textures,
                lightmap: lightmap.map(|(handle, _)| handle),
                irradiance_volume: None, // TODO
                models,

                data,
                qmap: map,
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

struct IrradianceVolumeBuilder {
    size: UVec3,
    full_shape: RuntimeShape<u32, 3>,
    data: Vec<[u8; 4]>,
    filled: Vec<bool>,
}
impl IrradianceVolumeBuilder {
    pub fn new(size: impl Into<UVec3>, default_color: [u8; 4]) -> Self {
        let size: UVec3 = size.into();
        let shape = RuntimeShape::<u32, 3>::new([
            size.x,
            size.y * 2,
            size.z * 3,
        ]);
        let vec_size = shape.usize();
        Self {
            size,
            full_shape: shape,
            data: vec![default_color; vec_size],
            filled: vec![false; vec_size],
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
        let pos = pos.into();
        self.put(pos, IrradianceVolumeDirection::X, color);
        self.put(pos, IrradianceVolumeDirection::Y, color);
        self.put(pos, IrradianceVolumeDirection::Z, color);
        self.put(pos, IrradianceVolumeDirection::NEG_X, color);
        self.put(pos, IrradianceVolumeDirection::NEG_Y, color);
        self.put(pos, IrradianceVolumeDirection::NEG_Z, color);
    }

    /// For any non-filled color, get replace with neighboring filled colors.
    pub fn flood_non_filled(&mut self) {
        for (i, filled) in self.filled.iter().copied().enumerate() {
            if filled { continue }

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

            if contributors == 0 { continue }
            // Average 'em
            self.data[i] = color.map(|v| (v / contributors) as u8)
        }
    }

    pub fn build(self) -> Image {
        Image::new(
            Extent3d { width: self.size.x, height: self.size.y * 2, depth_or_array_layers: self.size.z * 3 },
            TextureDimension::D3,
            self.data.into_flattened(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct IrradianceVolumeDirection(UVec3);
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