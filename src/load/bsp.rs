use crate::*;
use super::*;

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext}, render::{mesh::{Indices, PrimitiveTopology}, render_asset::RenderAssetUsages, render_resource::{Extent3d, TextureDimension, TextureFormat}}, utils::ConditionalSendFuture
};
use ndshape::*;
use q1bsp::{data::{bsp::BspTexFlags, bspx::LightGridCell}, glam::Vec3Swizzles, mesh::lighting::ComputeLightmapAtlasError};

/// A reference to a texture loaded from a BSP file. Stores the handle to the image, and the alpha mode that'll work for said image for performance reasons.
#[derive(Reflect, Debug, Clone, PartialEq, Eq)]
pub struct BspEmbeddedTexture {
    pub image_handle: Handle<Image>,
    pub alpha_mode: AlphaMode,
}

pub struct BspLoader {
    pub server: TrenchBroomServer,
}
impl FromWorld for BspLoader {
    fn from_world(world: &mut World) -> Self {
        Self { server: world.resource::<TrenchBroomServer>().clone() }
    }
}
impl AssetLoader for BspLoader {
    type Asset = Map;
    type Settings = ();
    type Error = io::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            let lit = load_context.read_asset_bytes(load_context.path().with_extension("lit")).await.ok();
            
            let data = BspData::parse(BspParseInput { bsp: &bytes, lit: lit.as_ref().map(Vec::as_slice) }).map_err(io::Error::other)?;

            let embedded_textures: HashMap<String, BspEmbeddedTexture> = data.parse_embedded_textures(self.server.config.texture_pallette.1)
                .into_iter()
                .map(|(name, image)| {
                    // let image = rgb_image_to_bevy_image(&image, &self.server, self.server.config.special_textures.is_some() && name.chars().next() == Some('{'));
                    let image = Image::new(
                        Extent3d { width: image.width(), height: image.height(), ..default() },
                        TextureDimension::D2,
                        image.pixels().map(|pixel| {
                            if self.server.config.special_textures.is_some() && name.chars().next() == Some('{') && pixel.0 == self.server.config.texture_pallette.1.colors[255] {
                                [0; 4]
                            } else {
                                [pixel[0], pixel[1], pixel[2], 255]
                            }
                        }).flatten().collect(),
                        // Without Srgb all the colors are washed out, so i'm guessing ericw-tools outputs sRGB, though i can't find it documented anywhere.
                        TextureFormat::Rgba8UnormSrgb,
                        self.server.config.bsp_textures_asset_usages,
                    );
                    
                    let alpha_mode = alpha_mode_from_image(&image);
                    
                    let image_handle = load_context.add_labeled_asset(name.clone(), image);
                    (name, BspEmbeddedTexture { image_handle, alpha_mode })
                })
                .collect();

            let lightmap = match data.compute_lightmap_atlas(self.server.config.compute_lightmap_settings, LightmapAtlasType::PerSlot) {
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
                    // let atlas_output = load_context.add_labeled_asset(format!("model_{model_idx}_lightmap_atlas"), if let Some(image) = output.lightmap_atlas.get(&LightmapStyle::NORMAL) {
                    //     rgb_image_to_bevy_image(image, &self.server, false)
                    // } else { new_lightmap_image(width, height) });
                    let output = load_context.add_labeled_asset("output".s(), new_lightmap_output_image(size.x, size.y));
        
                    /* let mut input_data = Vec::with_capacity(size.element_product() as usize * atlas.images.inner().len());
        
                    for (style, image) in atlas.images.inner() {
                        for rgb in image.chunks_exact(3) {
                            input_data.extend(rgb);
                            input_data.push(255);
                        }
                        styles.push(*style);
                    } */
                    
                    let mut i = 0;
                    let input = slots.map(|image| {
                        let handle = load_context.add_labeled_asset(format!("input_{i}"), Image::new(
                            Extent3d { width: image.width(), height: image.height(), ..default() },
                            TextureDimension::D2,
                            image.pixels().map(|pixel| {
                                [pixel[0], pixel[1], pixel[2], 255]
                            }).flatten().collect(),
                            // Without Srgb all the colors are washed out, so i'm guessing ericw-tools outputs sRGB, though i can't find it documented anywhere.
                            TextureFormat::Rgba8UnormSrgb,
                            self.server.config.bsp_textures_asset_usages,
                        ));

                        i += 1;
                        handle
                    });
                    /* let input = load_context.add_labeled_asset("lightmap_atlas_input".s(), Image::new(
                        Extent3d { width: atlas.images.size().x, height: atlas.images.size().y, depth_or_array_layers: atlas.images.inner().len() as u32 },
                        TextureDimension::D3,
                        input_data,
                        TextureFormat::Rgba8Unorm,
                        RenderAssetUsages::RENDER_WORLD,
                    )); */

                    let styles = load_context.add_labeled_asset("styles".s(), Image::new(
                        Extent3d { width: size.x, height: size.y, depth_or_array_layers: 1 },
                        TextureDimension::D2,
                        styles.into_vec(),
                        TextureFormat::Rgba8Uint,
                        RenderAssetUsages::RENDER_WORLD,
                    ));

                    let handle = load_context.add_labeled_asset("lightmap_atlas_animator".s(), AnimatedLightmap {
                        output,
                        input,
                        styles,
                    });

                    Some((handle, atlas.uvs))
                },
                Err(ComputeLightmapAtlasError::NoLightmaps) => None,
                Err(err) => return Err(io::Error::other(err)),
            };

            let qmap = parse_qmap(data.entities.as_bytes()).map_err(io_add_msg!("Parsing entities"))?;
            let mut map = qmap_to_map(qmap, load_context.path().to_string_lossy().into(), &self.server.config, |map_entity| {
                if map_entity.classname().map_err(invalid_data)? == "worldspawn" {
                    map_entity.geometry = MapEntityGeometry::Bsp(self.convert_q1bsp_mesh(&data, 0, &embedded_textures, lightmap.as_ref()));
                    return Ok(());
                }

                let Some(model) = map_entity.properties.get("model") else { return Ok(()) };
                let model_idx = model.trim_start_matches('*');
                // If there wasn't a * at the start, this is invalid
                if model_idx == model { return Ok(()) }

                let Ok(model_idx) = model_idx.parse::<usize>() else { return Ok(()) };

                map_entity.geometry = MapEntityGeometry::Bsp(self.convert_q1bsp_mesh(&data, model_idx, &embedded_textures, lightmap.as_ref()));
                Ok(())
            })?;

            map.embedded_textures = embedded_textures;

            // Calculate irradiance volumes for light grids.
            // Right now we just have one big irradiance volume for the entire map, this means the volume has to be less than 682 (2048/3 (z axis is 3x)) cells in size.
            // TODO In the future, it'd be better to split these up, cutting out big areas outside the map.
            if let Some(light_grid) = data.bspx.parse_light_grid_octree(&data.parse_ctx) {
                let mut light_grid = light_grid.map_err(io::Error::other)?;
                light_grid.mins = self.server.config.to_bevy_space(light_grid.mins.to_array().into()).to_array().into();
                // We add 1 to the size because the volume has to be offset by half a step to line up, and as such sometimes doesn't fill the full space
                light_grid.size = light_grid.size.xzy() + 1;
                light_grid.step = self.server.config.to_bevy_space(light_grid.step.to_array().into()).to_array().into();

                let mut builder = IrradianceVolumeBuilder::new(light_grid.size.to_array(), [0, 0, 0, 255]);
                
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
                image.sampler = bevy::render::texture::ImageSampler::linear();

                let image_handle = load_context.add_labeled_asset("IrradianceVolume".s(), image);

                let mins: Vec3 = light_grid.mins.to_array().into();
                let scale: Vec3 = (light_grid.size.as_vec3() * light_grid.step).to_array().into();

                map.irradiance_volumes.push((IrradianceVolume { voxels: image_handle, intensity: self.server.config.default_irradiance_volume_intensity }, Transform {
                    translation: mins + scale / 2. - Vec3::from_array(light_grid.step.to_array()) / 2.,
                    scale,
                    ..default()
                }));
            }

            Ok(map)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["bsp"]
    }
}
impl BspLoader {
    fn convert_q1bsp_mesh(
        &self,
        data: &BspData,
        model_idx: usize,
        embedded_textures: &HashMap<String, BspEmbeddedTexture>,
        lightmap: Option<&(Handle<AnimatedLightmap>, LightmapUvMap)>,
    ) -> Vec<(MapEntityGeometryTexture, Mesh)> {
        let (animated_lightmap, lightmap_uvs) = match lightmap {
            Some((animated_lightmap, lightmap_uvs)) => (Some(animated_lightmap), Some(lightmap_uvs)),
            None => (None, None),
        };
        
        let output = data.mesh_model(model_idx, lightmap_uvs);
        let mut meshes = Vec::with_capacity(output.meshes.len());

        for exported_mesh in output.meshes {
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
    
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, exported_mesh.positions.into_iter().map(convert_vec3(&self.server)).collect_vec());
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, exported_mesh.normals.into_iter().map(convert_vec3(&self.server)).collect_vec());
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, exported_mesh.uvs.iter().map(q1bsp::glam::Vec2::to_array).collect_vec());
            if let Some(lightmap_uvs) = &exported_mesh.lightmap_uvs {
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, lightmap_uvs.iter().map(q1bsp::glam::Vec2::to_array).collect_vec());
            }
            mesh.insert_indices(Indices::U32(exported_mesh.indices.into_flattened()));
    
            let texture = MapEntityGeometryTexture {
                embedded: embedded_textures.get(&exported_mesh.texture).cloned(),
                name: exported_mesh.texture,
                lightmap: animated_lightmap.cloned(),
                special: exported_mesh.tex_flags != BspTexFlags::Normal,
            };
    
            meshes.push((texture, mesh));
        }
        
        meshes
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

// enum IrradianceVolumeDirection {
//     X,
//     Y,
//     Z,
//     NegX,
//     NegY,
//     NegZ,
// }

#[test]
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
}