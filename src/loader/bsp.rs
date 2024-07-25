use crate::*;
use super::*;

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext}, render::{mesh::Indices, render_asset::RenderAssetUsages}, utils::ConditionalSendFuture
};

macro_rules! add_msg {($($args:tt)+) => {
    move |err| io::Error::new(err.kind(), format!("{}: {}", format!($($args)+), err.into_inner().map(|err| err.to_string()).unwrap_or_default()))
};}

// pub static TMP_DEBUG: Lazy<Mutex<(Vec<Vec3>, Vec<BspEdge>)>> = Lazy::new(default);

#[derive(Default)]
pub struct BspLoader;
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
            // let mut cursor = io::Cursor::new(&bytes);
            let mut reader = ByteReader::new(&bytes);

            let magic = reader.read_bytes(4)?;
            if magic != b"BSP2" {
                let msg = format!("Wrong magic number/format! Expected BSP2, found {}", std::str::from_utf8(magic).unwrap_or(&format!("{magic:?}")));
                return Err(io::Error::new(io::ErrorKind::InvalidData, msg));
            }

            // let version: u32 = reader.read()?;
            let lump_entries: [LumpEntry; LUMP_COUNT] = reader.read()?;
            
            // for (i, lump) in lumps.iter().enumerate() {
            //     println!("{:?}: {lump:#?}", unsafe { mem::transmute::<_, Lump>(i as u8) });
            // }
            // let mut parser = quake_util::bsp::Parser::new(&mut cursor).map_err(err_map)?;
            // println!("parsing bsp version {version}");

            // let qmap = parser.parse_entities().map_err(err_map)?;

            let entities_entry = &lump_entries[Lump::Entities as usize];
            // We split off the null byte here since this is a C string.
            let qmap_input = entities_entry.get(&bytes)?
                .split_last().map(|(_, v)| v).unwrap_or(&[]);
            let mut map = qmap_to_map(parse_qmap(qmap_input).map_err(add_msg!("Parsing entities"))?, load_context.path().to_string_lossy().into())?;

            let mut lumps = Lumps::read(&bytes, &lump_entries)?;

            // *TMP_DEBUG.lock().unwrap() = (lumps.vertices.clone(), lumps.edges.clone());
            
            for map_entity in &mut map.entities {
                if map_entity.classname().map_err(invalid_data)? == "worldspawn" {
                    map_entity.geometry = MapEntityGeometry::Bsp(lumps.create_meshes(0));
                    continue;
                }

                let Some(model) = map_entity.properties.get("model") else { continue };
                let model_idx = model.trim_start_matches('*');
                // If there wasn't a * at the start, this is invalid
                if model_idx == model { continue }

                let Ok(model_idx) = model_idx.parse::<usize>() else { continue };

                map_entity.geometry = MapEntityGeometry::Bsp(lumps.create_meshes(model_idx));
            }
            
            // println!("{:#?}", lumps.edges);
            // for info in &tex_info {
            //     println!("{:?}", std::str::from_utf8(&info.texture_name));
            // }
            
            Ok(map)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["bsp"]
    }
}

fn read_lump<T: BspRead>(data: &[u8], lumps: &[LumpEntry; LUMP_COUNT], lump: Lump) -> io::Result<Vec<T>> {
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

fn invalid_data(err: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

// fn add_msg(msg: impl Into<String>) -> impl FnOnce(io::Error) -> io::Error {
//     move |err| io::Error::new(err.kind(), format!("{msg}: {}", err.into_inner().map(|err| err.to_string()).unwrap_or_default()))
// }

// fn err_map(err: quake_util::BinParseError) -> io::Error {
//     match err {
//         quake_util::BinParseError::Io(err) => err,
//         quake_util::BinParseError::Parse(err) => io::Error::new(io::ErrorKind::InvalidData, err),
//     }
// }

/// Like an [io::Cursor], but i don't have to constantly juggle buffers.
struct ByteReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> ByteReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn read<T: BspRead>(&mut self) -> io::Result<T> {
        T::bsp_read(self)
    }

    fn read_bytes(&mut self, count: usize) -> io::Result<&[u8]> {
        let (from, to) = (self.pos, self.pos + count);
        if to > self.bytes.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, format!("Tried to read bytes from {from} to {to} from buffer of size {}", self.bytes.len())));
        }
        let bytes = &self.bytes[from..to];
        self.pos += count;
        Ok(bytes)
    }
}

/// Defines how a type should be read from a BSP file.
trait BspRead: Sized {
    fn bsp_read(reader: &mut ByteReader) -> io::Result<Self>;
}
macro_rules! impl_bsp_read_primitive {($ty:ty) => {
    impl BspRead for $ty {
        fn bsp_read(reader: &mut ByteReader) -> io::Result<Self> {
            Ok(<$ty>::from_le_bytes(reader.read()?))
        }
    }
};}
/// It would be nicer to do this with a proc macro, but this is specific to this file, and that sounds like a lot of work
macro_rules! impl_bsp_read_simple {($ty:ty, $($field:ident),+ $(,)?) => {
    impl BspRead for $ty {
        fn bsp_read(reader: &mut ByteReader) -> io::Result<Self> {
            Ok(Self { $($field: reader.read()?),+ })
        }
    }
};}
impl_bsp_read_primitive!(f32);
impl_bsp_read_primitive!(u32);
impl_bsp_read_primitive!(i32);

impl BspRead for Vec3 {
    fn bsp_read(reader: &mut ByteReader) -> io::Result<Self> {
        Ok(vec3(
            reader.read()?,
            reader.read()?,
            reader.read()?,
        ).trenchbroom_to_bevy_space())
        // So far I've not encountered a Vec3 that doesn't represent a point in space, so we just always transform it here
    }
}
// We'd have to change this if we want to impl BspRead for u8
impl<T: BspRead + std::fmt::Debug, const NUM: usize> BspRead for [T; NUM] {
    fn bsp_read(reader: &mut ByteReader) -> io::Result<Self> {
        // Look ma, no heap allocations!
        // let mut out = Vec::new();
        let mut out = [(); NUM].map(|_| mem::MaybeUninit::uninit());
        for i in 0..NUM {
            out[i].write(reader.read()?);
            // out.push(reader.read()?);
        }
        // Ok(out.try_into().unwrap())
        Ok(out.map(|v| unsafe { v.assume_init() }))
    }
}
impl<const NUM: usize> BspRead for [u8; NUM] {
    fn bsp_read(reader: &mut ByteReader) -> io::Result<Self> {
        Ok(reader.read_bytes(NUM)?.try_into().unwrap())
    }
}


#[derive(Debug, Clone, Copy)]
enum Lump {
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
const LUMP_COUNT: usize = 15; // I don't want to bring in strum just for this

#[derive(Debug)]
#[repr(C)]
struct LumpEntry {
    pub offset: u32,
    pub len: u32,
}
impl_bsp_read_simple!(LumpEntry, offset, len);
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

struct Lumps {
    pub vertices: Vec<Vec3>,
    pub planes: Vec<BspPlane>,
    pub edges: Vec<BspEdge>,
    pub surface_edges: Vec<i32>,
    pub faces: Vec<BspFace>,
    pub tex_info: Vec<BspTexInfo>,
    pub models: Vec<BspModel>,
}
impl Lumps {
    pub fn read(data: &[u8], lump_entries: &[LumpEntry; LUMP_COUNT]) -> io::Result<Self> {
        Ok(Self {
            vertices: read_lump(data, &lump_entries, Lump::Vertices)?,
            planes: read_lump(data, &lump_entries, Lump::Planes)?,
            edges: read_lump(data, &lump_entries, Lump::Edges)?,
            surface_edges: read_lump(data, &lump_entries, Lump::SurfEdges)?,
            faces: read_lump(data, &lump_entries, Lump::Faces)?,
            tex_info: read_lump(data, &lump_entries, Lump::TexInfo)?,
            models: read_lump(data, &lump_entries, Lump::Models)?,
        })
    }

    pub fn create_meshes(&self, model_idx: usize) -> HashMap<String, Mesh> {
        let model = &self.models[model_idx];
        
        let mut positions: Vec<Vec3> = default();
        let mut normals: Vec<Vec3> = default();
        let mut uvs: Vec<Vec2> = default();
        let mut indices: Vec<u32> = default();
    
        for i in model.first_face..model.first_face + model.num_faces {
            let face = &self.faces[i as usize];
            let plane = &self.planes[face.plane_idx as usize];
            let tex_info = &self.tex_info[face.texture_info_idx as usize];
    
            let first_index = positions.len() as u32;
            for i in face.first_edge..face.first_edge + face.num_edges {
                let surf_edge = self.surface_edges[i as usize];
                let edge = self.edges[surf_edge.abs() as usize];
                let vert_idx = if surf_edge.is_negative() { (edge.b, edge.a) } else { (edge.a, edge.b) };

                let pos = self.vertices[vert_idx.0 as usize];

                positions.push(pos);
                normals.push(plane.normal);
                let scale = trenchbroom_config_mirror!().scale;
                uvs.push(vec2(tex_info.u_axis.dot(pos) + tex_info.u_offset, tex_info.v_axis.dot(pos) + tex_info.v_offset) * scale);
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
        
        // TODO
        HashMap::from([("bricks".into(), mesh)])
    }
}




#[derive(Debug)]
#[repr(C)]
struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}
impl_bsp_read_simple!(BoundingBox, min, max);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct BspEdge {
    /// The index to the first vertex this edge connects
    pub a: u32,
    /// The index to the second vertex this edge connects
    pub b: u32,
}
impl_bsp_read_simple!(BspEdge, a, b);

#[derive(Debug)]
#[repr(C)]
struct BspFace {
    /// Index of the plane the face is parallel to
    pub plane_idx: u32,
    /// Set if the normal is parallel to the plane normal (???)
    pub plane_side: u32,

    // TODO the face edge array doesn't exist
    /// Index of the first edge (in the face edge array)
    pub first_edge: u32,
    /// Number of consecutive edges (in the face edge array)
    pub num_edges: u32,

    /// Index of the texture info structure
    pub texture_info_idx: u32,

    /// Styles (bit flags) for the lightmaps
    pub lightmap_styles: [u8; 4],
    /// Offset of the lightmap (in bytes) in the lightmap lump
    pub lightmap_offset: u32,
}
impl_bsp_read_simple!(BspFace, plane_idx, plane_side, first_edge, num_edges, texture_info_idx, lightmap_styles, lightmap_offset);

#[derive(Debug)]
#[repr(C)]
struct BspTexInfo {
    pub u_axis: Vec3,
    pub u_offset: f32,

    pub v_axis: Vec3,
    pub v_offset: f32,

    pub texture_idx: u32,
    pub flags: u32,
    
    // pub flags: u32,
    // pub value: u32,

    // pub texture_name: [u8; 32], // ASCII text

    // pub next_tex_info: u32,
}
impl_bsp_read_simple!(BspTexInfo, u_axis, u_offset, v_axis, v_offset, texture_idx, flags);
// impl_bsp_read_simple!(BspTexInfo, u_axis, u_offset, v_axis, v_offset, flags, value, texture_name, next_tex_info);

#[derive(Debug)]
#[repr(C)]
struct BspModel {
    pub bound: BoundingBox,
    /// Origin of model, usually (0,0,0)
    pub origin: Vec3,

    pub head_node: [u32; 4],

    pub visleafs: u32,
    pub first_face: u32,
    pub num_faces: u32,

    // /// index of first BSP node
    // pub bsp_node_idx: u32,
    // /// index of the first Clip node
    // pub first_clip_node_idx: u32,
    // /// index of the second Clip node
    // pub second_clip_node_idx: u32,
    // /// usually zero (?)
    // pub node_id3_idx: u32,
    // /// number of BSP leaves
    // pub num_leafs: u32,
    // /// index of Faces
    // pub face_idx: u32,
    // /// number of Faces
    // pub face_num: u32,
}
impl_bsp_read_simple!(BspModel, bound, origin, head_node, visleafs, first_face, num_faces);
// impl_bsp_read_simple!(BspModel, bound, origin, bsp_node_idx, first_clip_node_idx, second_clip_node_idx, node_id3_idx, num_leafs, face_idx, face_num);

#[derive(Debug)]
#[repr(C)]
struct BspPlane {
    pub normal: Vec3,
    pub dist: f32,
    /// Not really sure what this is, not used anywhere
    pub ty: u32,
}
impl_bsp_read_simple!(BspPlane, normal, dist, ty);

#[derive(Debug)]
#[repr(C)]
struct BspTextureHeader {
    pub num_mip_textures: u32,
    pub offset: u32, // TODO long offset[num_mip_textures];???
}
impl_bsp_read_simple!(BspTextureHeader, num_mip_textures, offset);

#[derive(Debug)]
#[repr(C)]
struct BspMipTexture {
    /// Ascii characters
    pub texture_name: [u8; 16],

    pub width: u32,
    pub height: u32,

    pub offset_full: u32,
    pub offset_half: u32,
    pub offset_quarter: u32,
    pub offset_eighth: u32,
}
impl_bsp_read_simple!(BspMipTexture, texture_name, width, height, offset_full, offset_half, offset_quarter, offset_eighth);