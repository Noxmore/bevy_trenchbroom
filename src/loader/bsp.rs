use crate::*;
use super::*;
use q1bsp::prelude::*;

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext}, render::{mesh::{Indices, PrimitiveTopology}, render_asset::RenderAssetUsages, render_resource::Extent3d, texture::ImageSampler}, utils::ConditionalSendFuture
};

// TODO this should be configurable?
pub(crate) const DEFAULT_LIGHTMAP_EXPOSURE: f32 = 5000.;

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
            
            // TODO cache atlas?
            let data = BspData::parse(BspParseInput { bsp: &bytes, lit: lit.as_ref().map(Vec::as_slice) }).map_err(io::Error::other)?;
            let mut map = qmap_to_map(parse_qmap(data.entities.as_bytes()).map_err(add_msg!("Parsing entities"))?, load_context.path().to_string_lossy().into(), &self.server.config)?;
            let textures = data.parse_embedded_textures(&QUAKE_PALETTE)
                .into_iter()
                .map(|(name, image)| {
                    let image_handle = load_context.add_labeled_asset(name.clone(), rgb_image_to_bevy_image(&image));

                    // TODO store images not materials
                    (name.clone(), load_context.add_labeled_asset(name + "_material", StandardMaterial {
                        base_color_texture: Some(image_handle),
                        perceptual_roughness: 1.,
                        lightmap_exposure: DEFAULT_LIGHTMAP_EXPOSURE,
                        ..default()
                    }))
                })
                .collect();

            for map_entity in &mut map.entities {
                if map_entity.classname().map_err(invalid_data)? == "worldspawn" {
                    map_entity.geometry = MapEntityGeometry::Bsp(self.convert_q1bsp_mesh(&data, 0, &textures, load_context));
                    continue;
                }

                let Some(model) = map_entity.properties.get("model") else { continue };
                let model_idx = model.trim_start_matches('*');
                // If there wasn't a * at the start, this is invalid
                if model_idx == model { continue }

                let Ok(model_idx) = model_idx.parse::<usize>() else { continue };

                map_entity.geometry = MapEntityGeometry::Bsp(self.convert_q1bsp_mesh(&data, model_idx, &textures, load_context));
            }

            Ok(map)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["bsp"]
    }
}
impl BspLoader {
    fn convert_q1bsp_mesh(&self, data: &BspData, model_idx: usize, textures: &HashMap<String, Handle<StandardMaterial>>, load_context: &mut LoadContext) -> Vec<(MapEntityGeometryTexture, Mesh)> {
        let output = data.mesh_model(model_idx);
        let mut meshes = Vec::with_capacity(output.meshes.len());
    
        let lightmap = output.lightmap_atlas.map(|atlas| {
            // Convert lightmap_atlas grid into image
            let mut image = rgb_image_to_bevy_image(&atlas);
    
            image.sampler = ImageSampler::linear();
    
            // TODO
            // image.clone().try_into_dynamic().unwrap().save_with_format(format!("target/lightmap_{model_idx}.png"), image::ImageFormat::Png).ok();
    
            load_context.add_labeled_asset(format!("model_{model_idx}_lightmap"), image)
        });
    
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
                embedded: textures.get(&exported_mesh.texture).cloned(),
                name: exported_mesh.texture,
                lightmap: lightmap.clone(),
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

fn rgb_image_to_bevy_image(image: &image::RgbImage) -> Image {
    Image::new(
        Extent3d { width: image.width(), height: image.height(), depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        image.pixels().map(|pixel| [pixel[0], pixel[1], pixel[2], 255]).flatten().collect(),
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(), // TODO should this be RENDER_WORLD?
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