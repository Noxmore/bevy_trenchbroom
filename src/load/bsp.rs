use crate::*;
use super::*;

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext}, render::{mesh::{Indices, PrimitiveTopology}, render_asset::RenderAssetUsages, render_resource::{Extent3d, TextureDimension, TextureFormat}}, utils::ConditionalSendFuture
};
use q1bsp::{data::BspTexFlags, mesh::lighting::ComputeLightmapAtlasError};

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
            // use pre-computed one
            // let mut aabb = Aabb::enclosing(data.vertices.iter().map(|x| Vec3::from_array(x.to_array())));
            // data.

            let embedded_textures: HashMap<String, BspEmbeddedTexture> = data.parse_embedded_textures(self.server.config.texture_pallette.1)
                .into_iter()
                .map(|(name, image)| {
                    let image = rgb_image_to_bevy_image(&image, &self.server, self.server.config.special_textures.is_some() && name.chars().next() == Some('{'));
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
                        let handle = load_context.add_labeled_asset(format!("input_{i}"), rgb_image_to_bevy_image(&image, &self.server, false));
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
            // image::RgbImage::
    
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

fn rgb_image_to_bevy_image(image: &image::RgbImage, tb_server: &TrenchBroomServer, enable_alphatest: bool) -> Image {
    Image::new(
        Extent3d { width: image.width(), height: image.height(), depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        image.pixels().map(|pixel| {
            if enable_alphatest && pixel.0 == tb_server.config.texture_pallette.1.colors[255] {
                [0; 4]
            } else {
                [pixel[0], pixel[1], pixel[2], 255]
            }
        }).flatten().collect(),
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        tb_server.config.embedded_textures_asset_usages,
    )
}

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