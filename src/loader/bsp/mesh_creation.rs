use bevy::render::{mesh::{Indices, VertexAttributeValues}, render_asset::RenderAssetUsages, render_resource::Extent3d};

use crate::*;
use super::*;

impl Lumps {
    pub fn create_meshes(&self, model_idx: usize, load_context: &mut LoadContext) -> Vec<(MapEntityGeometryTexture, Mesh)> {
        let model = &self.models[model_idx];
        
        // let mut lightmap_atlas: Grid<[u8; 3]> = Grid::new(16, 16, [0; 3]);
        let mut lightmap_atlas: Grid<Option<[u8; 3]>> = Grid::new(1, 1);
        // let mut lightmap_atlas = Image::default();
        // lightmap_atlas.
        
        let mut grouped_faces: HashMap<&str, Vec<&BspFace>> = default();

        for i in model.first_face..model.first_face + model.num_faces {
            let face = &self.faces[i as usize];
            let tex_info = &self.tex_info[face.texture_info_idx as usize];
            let Some(texture) = &self.textures[tex_info.texture_idx as usize] else { continue };
            // let name = match std::str::from_utf8(&texture.texture_name) {
            //     Ok(s) => s,
            //     Err(err) => {
            //         error!("Failed to read texture name for {:?}: {err}", texture.texture_name);
            //         "MISSING"
            //     }
            // };

            grouped_faces.entry(texture.header.name.as_str()).or_default().push(face);
        }
        
        let mut meshes = Vec::new();
        
        for (texture, faces) in grouped_faces {
            let mut positions: Vec<Vec3> = default();
            let mut normals: Vec<Vec3> = default();
            let mut uvs: Vec<Vec2> = default();
            let mut uvs_light: Vec<Vec2> = default();
            let mut indices: Vec<u32> = default();
        
            for face in faces {
                let plane = &self.planes[face.plane_idx as usize];
                let tex_info = &self.tex_info[face.texture_info_idx as usize];
                let texture_size = self.textures[tex_info.texture_idx as usize].as_ref()
                    .map(|tex| vec2(tex.header.width as f32, tex.header.height as f32))
                    .unwrap_or(Vec2::ONE);

                
                // The uv coordinates of the face's lightmap in the world, rather than on a lightmap atlas
                let mut lightmap_world_uvs: Vec<Vec2> = default();
        
                let first_index = positions.len() as u32;
                for i in face.first_edge..face.first_edge + face.num_edges {
                    let surf_edge = self.surface_edges[i as usize];
                    let edge = self.edges[surf_edge.abs() as usize];
                    let vert_idx = if surf_edge.is_negative() { (edge.b, edge.a) } else { (edge.a, edge.b) };
    
                    let pos = self.vertices[vert_idx.0 as usize];
    
                    positions.push(pos);
                    normals.push(if face.plane_side == 0 { plane.normal } else { -plane.normal });
                    let scale = TrenchBroomConfigMirrorGuard::get().scale;

                    let uv = vec2(
                        // Counteract the trenchbroom_to_bevy_space conversion by multiplying by scale twice
                        // TODO is there a more elegant way of fixing this?
                        pos.dot(tex_info.u_axis * scale * scale) + tex_info.u_offset,
                        pos.dot(tex_info.v_axis * scale * scale) + tex_info.v_offset,
                    );
                    
                    uvs.push(uv / texture_size);
                    // Lightmap uvs have a constant scale of 16-units to 1 texel
                    lightmap_world_uvs.push(uv / 16.);
                }
    
                // Calculate indices
                for i in 1..face.num_edges - 1 {
                    indices.extend([0, i + 1, i].map(|x| first_index + x));
                }

                //////////////////////////////////////////////////////////////////////////////////
                //// LIGHTMAP
                //////////////////////////////////////////////////////////////////////////////////
                
                let Some(lighting) = &self.lighting else { continue };
                if face.lightmap_offset.is_negative() {
                    // Just in case only some faces are negative (Not sure why this happens)
                    uvs_light.extend(repeat_n(Vec2::ZERO, lightmap_world_uvs.len()));
                    continue;
                }
                // match lighting {
                //     BspLighting::White(_) => println!("white"),
                //     &BspLighting::Colored(_) => println!("colored"),
                // }
                
                let mut lightmap_rect = Rect::EMPTY;
                for uv in &lightmap_world_uvs {
                    lightmap_rect = lightmap_rect.union_point(*uv);
                }
                let size = lightmap_rect.size().ceil().as_uvec2();
                // println!("rect: {lightmap_rect:?}, size: {size}");
                // println!("rect: {:?}", Rect::EMPTY.union_point(vec2(13., 15.)));

                let mut target_pos: Option<UVec2> = None;
                // Brute force search for free space
                if lightmap_atlas.cols() as u32 >= size.x && lightmap_atlas.rows() as u32 >= size.y {
                    'find_loop: for x in 0..lightmap_atlas.cols().saturating_sub(size.x as usize) {
                        'next_position: for y in 0..lightmap_atlas.rows().saturating_sub(size.y as usize) {
                            // Check the rect against this position
                            for local_x in 0..size.x {
                                for local_y in 0..size.y {
                                    // If this cell has already been taken, continue
                                    if lightmap_atlas[(y + local_y as usize, x + local_x as usize)].is_some() {
                                        continue 'next_position;
                                    }
                                }
                            }
                            // If we get here, this spot is good.
                            target_pos = Some(uvec2(x as u32, y as u32));
                            
                            break 'find_loop;
                        }
                    }
                }
                let target_pos = match target_pos {
                    Some(pos) => pos,
                    None => {
                        // println!("Made room: {size}");
                        // We couldn't find a spot for this lightmap, let's make some room!
                        let prev_cols = lightmap_atlas.cols();
                        
                        // println!("From ({}, {})", lightmap_atlas.cols(), lightmap_atlas.rows());
                        for _ in 0..size.x {
                            lightmap_atlas.push_col(vec![None; lightmap_atlas.rows()]);
                        }
                        for _ in 0..size.y {
                            lightmap_atlas.push_row(vec![None; lightmap_atlas.cols()]);
                        }
                        // println!("To ({}, {})", lightmap_atlas.cols(), lightmap_atlas.rows());
                        // println!();

                        uvec2(prev_cols as u32, 0)
                    }
                };

                for x in 0..size.x {
                    for y in 0..size.y {
                        let pos = target_pos + uvec2(x, y);
                        // println!("pos: {pos}, size: {}, {}", lightmap_atlas.cols(), lightmap_atlas.rows());
                        lightmap_atlas[(pos.y as usize, pos.x as usize)] = lighting.get(face.lightmap_offset as usize + (y * size.x + x) as usize);
                    }
                }
                
                // Append lightmap uvs, since lightmap face size is calculated from the uvs bounds, we don't need to resize it, just move it into place
                // Atlas uvs will be in texture space until converted later
                uvs_light.extend(lightmap_world_uvs.into_iter().map(|uv| uv - lightmap_rect.min + target_pos.as_vec2()));
            }
            assert!(uvs_light.iter().all(|uv| uv.x <= lightmap_atlas.cols() as f32 && uv.y <= lightmap_atlas.rows() as f32));
    
            indices.dedup();
    
            let mut mesh = Mesh::new(bevy::render::mesh::PrimitiveTopology::TriangleList, RenderAssetUsages::all());
    
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            if self.lighting.is_some() {
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uvs_light);
            }
            mesh.insert_indices(Indices::U32(indices));

            meshes.push((
                MapEntityGeometryTexture {
                    name: texture.to_string(),
                    embedded: self.embedded_textures.get(texture).cloned(),
                    lightmap: None,
                },
                mesh,
            ));
        }

        // Finalize lightmap data
        if self.lighting.is_some() {
            // Convert lightmap_atlas grid into image
            let mut image = Image::new(
                Extent3d { width: lightmap_atlas.cols() as u32, height: lightmap_atlas.rows() as u32, depth_or_array_layers: 1 },
                bevy::render::render_resource::TextureDimension::D2,
                lightmap_atlas.iter().copied().map(Option::unwrap_or_default).map(|[r, g, b]| [r, g, b, 255]).flatten().collect(),
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::all(), // TODO should this be RENDER_WORLD?
            );

            // TODO
            // image.sampler = ImageSampler::linear();

            // TODO
            image.clone().try_into_dynamic().unwrap().save_with_format(format!("target/lightmap_{model_idx}.png"), image::ImageFormat::Png).unwrap();
            let lightmap_handle = load_context.add_labeled_asset(format!("model_{model_idx}_lightmap"), image);
            
            for (texture, mesh) in &mut meshes {
                texture.lightmap = Some(lightmap_handle.clone());

                // Normalize lightmap uvs from texture space
                let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_1) else { unreachable!() };
                let atlas_size = vec2(lightmap_atlas.cols() as f32, lightmap_atlas.rows() as f32);
                for uv in uvs {
                    uv[0] /= atlas_size.x;
                    uv[1] /= atlas_size.y;
                }
            }
            
            println!("lighting finalized");
        }
        
        meshes
    }
}