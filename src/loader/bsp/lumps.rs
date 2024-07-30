use bevy::render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::Extent3d};

use crate::*;
use super::*;

#[derive(Debug, Clone, Copy)]
pub enum Lump {
    Entities = 0,
    Planes,
    Textures,
    Vertices,
    Visibility,
    Nodes,
    TexInfo,
    Faces,
    Lighting,
    ClipNodes,
    Leafs,
    MarkSurfaces,
    Edges,
    SurfEdges,
    Models,
}
pub const LUMP_COUNT: usize = 15; // I don't want to bring in strum just for this

#[derive(Debug)]
#[repr(C)]
pub struct LumpEntry {
    pub offset: u32,
    pub len: u32,
}
// BspRead implemented in lump_data
impl LumpEntry {
    pub fn get<'a>(&self, data: &'a [u8]) -> io::Result<&'a [u8]> {
        let (from, to) = (self.offset as usize, self.offset as usize + self.len as usize);
        if to > data.len() {
            Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Lump ending not in file! Malformed/corrupted data?"))
        } else {
            Ok(&data[from..to])
        }
    }
}

pub fn read_lump<T: BspRead>(data: &[u8], lumps: &[LumpEntry; LUMP_COUNT], lump: Lump) -> io::Result<Vec<T>> {
    let entry = &lumps[lump as usize];
    let lump_data = entry.get(data)?;
    let lump_entries = entry.len as usize / mem::size_of::<T>();

    let mut reader = ByteReader::new(lump_data);
    let mut out = Vec::new();
    out.reserve(lump_entries);

    for i in 0..lump_entries {
        out.push(reader.read().map_err(add_msg!("Parsing lump \"{lump:?}\" entry {i}"))?);
    }

    Ok(out)
}

pub struct Lumps {
    pub vertices: Vec<Vec3>,
    pub planes: Vec<BspPlane>,
    pub edges: Vec<BspEdge>,
    pub surface_edges: Vec<i32>,
    pub faces: Vec<BspFace>,
    pub tex_info: Vec<BspTexInfo>,
    pub models: Vec<BspModel>,
    pub textures: Vec<Option<BspTexture>>,

    /// Textures from 
    pub embedded_textures: HashMap<String, Handle<StandardMaterial>>,
}
impl Lumps {
    pub fn read(data: &[u8], lump_entries: &[LumpEntry; LUMP_COUNT], load_context: &mut LoadContext) -> io::Result<Self> {
        let mut lumps = Self {
            vertices: read_lump(data, &lump_entries, Lump::Vertices)?,
            planes: read_lump(data, &lump_entries, Lump::Planes)?,
            edges: read_lump(data, &lump_entries, Lump::Edges)?,
            surface_edges: read_lump(data, &lump_entries, Lump::SurfEdges)?,
            faces: read_lump(data, &lump_entries, Lump::Faces)?,
            tex_info: read_lump(data, &lump_entries, Lump::TexInfo)?,
            models: read_lump(data, &lump_entries, Lump::Models)?,
            textures: read_texture_lump(&mut ByteReader::new(lump_entries[Lump::Textures as usize].get(data)?)).map_err(add_msg!("Reading texture lump"))?,

            embedded_textures: HashMap::new(),
        };

        // Read embedded textures
        // let entry = &lump_entries[Lump::Textures as usize];
        // let lump_data = entry.get(data)?;
        // for texture in lumps.textures.iter().flatten() {
        lumps.embedded_textures = lumps.textures.par_iter().flatten().filter(|texture| texture.data.is_some()).map(|texture| {
            // let Some(data) = &texture.data else { continue };
            let Some(data) = &texture.data else { unreachable!() };
            let mut alpha_mode = AlphaMode::Opaque;
            let image = Image::new(
                Extent3d { width: texture.header.width, height: texture.header.height, depth_or_array_layers: 1 },
                bevy::render::render_resource::TextureDimension::D2,
                // TODO can wads use custom palettes?
                data.iter().copied().map(|v| {
                    // According to https://quakewiki.org/wiki/Quake_palette, this is a special case, allowing transparency
                    if v == 255 {
                        // Only set alpha_mode to mask if we actually use transparency
                        alpha_mode = AlphaMode::Mask(0.5);
                        return [0, 0, 0, 0];
                    }
                    let [r, g, b] = QUAKE_PALETTE[v as usize];
                    [r, g, b, 255]
                }).flatten().collect_vec(),
                bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                RenderAssetUsages::all(),
            );
            
            let texture_name = texture.header.name.as_str().to_string();
            // println!("loading {texture_name}");
            // let image_handle = load_context.add_labeled_asset(format!("{texture_name}_color"), image);
            
            (
                texture_name.clone(),
                image,
                alpha_mode,
                // load_context.add_labeled_asset(texture_name, StandardMaterial { base_color_texture: Some(image_handle), perceptual_roughness: 1., alpha_mode, ..default() }),
            )
        }).collect::<Vec<_>>().into_iter().map(|(name, image, alpha_mode)| {
            let image_handle = load_context.add_labeled_asset(format!("{name}_color"), image);
            (
                name.clone(),
                load_context.add_labeled_asset(name, StandardMaterial { base_color_texture: Some(image_handle), perceptual_roughness: 1., alpha_mode, ..default() }),
            )
        }).collect();
        
        Ok(lumps)
    }

    pub fn create_meshes(&self, model_idx: usize) -> HashMap<MapEntityGeometryTexture, Mesh> {
        let model = &self.models[model_idx];
        
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
        
        let mut grouped_meshes = HashMap::new();
        
        for (texture, faces) in grouped_faces {
            let mut positions: Vec<Vec3> = default();
            let mut normals: Vec<Vec3> = default();
            let mut uvs: Vec<Vec2> = default();
            let mut indices: Vec<u32> = default();
        
            for face in faces {
                let plane = &self.planes[face.plane_idx as usize];
                let tex_info = &self.tex_info[face.texture_info_idx as usize];
                let texture_size = self.textures[tex_info.texture_idx as usize].as_ref()
                    .map(|tex| vec2(tex.header.width as f32, tex.header.height as f32))
                    .unwrap_or(Vec2::ONE);
        
                let first_index = positions.len() as u32;
                for i in face.first_edge..face.first_edge + face.num_edges {
                    let surf_edge = self.surface_edges[i as usize];
                    let edge = self.edges[surf_edge.abs() as usize];
                    let vert_idx = if surf_edge.is_negative() { (edge.b, edge.a) } else { (edge.a, edge.b) };
    
                    let pos = self.vertices[vert_idx.0 as usize];
    
                    positions.push(pos);
                    normals.push(plane.normal);
                    let scale = trenchbroom_config_mirror!().scale;
                    uvs.push(vec2(
                        // Counteract the trenchbroom_to_bevy_space conversion by multiplying by scale twice
                        // TODO is there a more elegant way of fixing this?
                        pos.dot(tex_info.u_axis * scale * scale) + tex_info.u_offset,
                        pos.dot(tex_info.v_axis * scale * scale) + tex_info.v_offset,
                    ) / texture_size);
                }
    
                // Calculate indices
                for i in 1..face.num_edges - 1 {
                    indices.extend([0, i + 1, i].map(|x| first_index + x));
                }
            }
    
            indices.dedup();
    
            let mut mesh = Mesh::new(bevy::render::mesh::PrimitiveTopology::TriangleList, RenderAssetUsages::all());
    
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            mesh.insert_indices(Indices::U32(indices));

            grouped_meshes.insert(
                MapEntityGeometryTexture {
                    name: texture.to_string(),
                    embedded: self.embedded_textures.get(texture).cloned(),
                },
                mesh,
            );
        }
        
        grouped_meshes
    }
}