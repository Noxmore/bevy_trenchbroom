use crate::*;
use super::*;

// NOTE: I use repr(C) here for structs where the size does matter, to protect it from rust optimization somehow causing the size to change in the future
//       Said structs are used in the read_lump function, where it splits up the data based on the size of the struct

/// Defines how a type should be read from a BSP file.
pub trait BspRead: Sized {
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
            Ok(Self { $($field: reader.read().map_err(add_msg!(concat!("Reading field \"", stringify!($field), "\" on type ", stringify!($ty))))?),+ })
        }
    }
};}
impl_bsp_read_primitive!(f32);
impl_bsp_read_primitive!(u32);
impl_bsp_read_primitive!(i32);

impl_bsp_read_simple!(LumpEntry, offset, len);

impl BspRead for Vec3 {
    fn bsp_read(reader: &mut ByteReader) -> io::Result<Self> {
        Ok(vec3(
            reader.read()?,
            reader.read()?,
            reader.read()?,
        ).trenchbroom_to_bevy_space())
    }
}

// We'd have to change this if we want to impl BspRead for u8
impl<T: BspRead + std::fmt::Debug, const NUM: usize> BspRead for [T; NUM] {
    fn bsp_read(reader: &mut ByteReader) -> io::Result<Self> {
        // Look ma, no heap allocations!
        let mut out = [(); NUM].map(|_| mem::MaybeUninit::uninit());
        for i in 0..NUM {
            out[i].write(reader.read()?);
        }
        Ok(out.map(|v| unsafe { v.assume_init() }))
    }
}
impl<const NUM: usize> BspRead for [u8; NUM] {
    fn bsp_read(reader: &mut ByteReader) -> io::Result<Self> {
        Ok(reader.read_bytes(NUM)?.try_into().unwrap())
    }
}

#[derive(Clone)]
pub struct FixedStr<const CHARS: usize> {
    data: [u8; CHARS],
}
impl<const CHARS: usize> BspRead for FixedStr<CHARS> {
    fn bsp_read(reader: &mut ByteReader) -> io::Result<Self> {
        let data: [u8; CHARS] = reader.read()?;
        std::str::from_utf8(&data).map_err(invalid_data)?;
        Ok(Self { data })
    }
}
impl<const CHARS: usize> FixedStr<CHARS> {
    pub fn as_str(&self) -> &str {
        // SAFETY: This is checked when a FixedStr is created
        unsafe { std::str::from_utf8_unchecked(&self.data) }.trim_end_matches('\0')
    }
}
impl<const CHARS: usize> std::fmt::Debug for FixedStr<CHARS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl<const CHARS: usize> std::fmt::Display for FixedStr<CHARS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}





#[derive(Debug)]
#[repr(C)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}
impl_bsp_read_simple!(BoundingBox, min, max);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BspEdge {
    /// The index to the first vertex this edge connects
    pub a: u32,
    /// The index to the second vertex this edge connects
    pub b: u32,
}
impl_bsp_read_simple!(BspEdge, a, b);

#[derive(Debug)]
#[repr(C)]
pub struct BspFace {
    /// Index of the plane the face is parallel to
    pub plane_idx: u32,
    /// If not zero, seems to indicate that the normal should be inverted when creating meshes
    pub plane_side: u32,

    /// Index of the first edge (in the face edge array)
    pub first_edge: u32,
    /// Number of consecutive edges (in the face edge array)
    pub num_edges: u32,

    /// Index of the texture info structure
    pub texture_info_idx: u32,

    /// Styles (bit flags) for the lightmaps
    pub lightmap_styles: [u8; 4],

    /// Offset of the lightmap (in bytes) in the lightmap lump, or -1 if no lightmap
    pub lightmap_offset: i32,
}
impl_bsp_read_simple!(BspFace, plane_idx, plane_side, first_edge, num_edges, texture_info_idx, lightmap_styles, lightmap_offset);
impl BspFace {
    /// The kind of lighting that should be applied to the face.
    /// - value 0 is the normal value, to be used with a light map.
    /// - value 0xFF is to be used when there is no light map.
    /// - value 1 produces a fast pulsating light
    /// - value 2 produces a slow pulsating light
    /// - value 3 to 10 produce various other lighting effects. (TODO implement these lighting effects?)
    pub fn light_type(&self) -> u8 {
        self.lightmap_styles[0]
    }

    /// Gives the base light level for the face, that is the minimum light level for the light map, or the constant light level in the absence of light map.
    /// Curiously, value 0xFF codes for minimum light, and value 0 codes for maximum light.
    pub fn base_light(&self) -> u8 {
        self.lightmap_styles[1]
    }
}

/// TODO Document
pub struct BspFaceExtents {
    pub texture_mins: U16Vec2,
    pub extents: U16Vec2,
}
impl BspFaceExtents {
    pub fn calculate(lumps: &Lumps, face: &BspFace) -> Self {
        // Implantation referenced from vkQuake (https://github.com/Novum/vkQuake/blob/b6eb0cf5812c09c661d51e3b95fc08d88da2288a/Quake/gl_model.c#L1287)
        let tex_info = &lumps.tex_info[face.texture_info_idx as usize];

        let mut rect = Rect::EMPTY;

        // let tex_vecs = [
        //     tex_info.u_axis.xyzx().with_w(tex_info.u_offset),
        //     tex_info.v_axis.xyzx().with_w(tex_info.v_offset),
        // ];
        let tex_axis = [ tex_info.u_axis, tex_info.v_axis ];
        let tex_offsets = [ tex_info.u_offset, tex_info.v_offset ];

        for edge_idx in 0..face.num_edges {
            let edge_idx = lumps.surface_edges[(face.first_edge + edge_idx) as usize];

            let vertex = if edge_idx.is_negative() {
                lumps.vertices[lumps.edges[-edge_idx as usize].a as usize]
            } else {
                lumps.vertices[lumps.edges[edge_idx as usize].b as usize]
            };

            for axis_idx in 0..2 {
                // TODO coordinate system translation?
                // let val = (
                //     vertex.x as f64 * tex_vecs[tex_vec_idx].x as f64 +
                //     vertex.y as f64 * tex_vecs[tex_vec_idx].x as f64 +
                //     vertex.z as f64 * tex_vecs[tex_vec_idx].z as f64) +
                //     tex_vecs[tex_vec_idx].w as f64;

                let val = vertex.as_dvec3().dot(tex_axis[axis_idx].as_dvec3()) + tex_offsets[axis_idx] as f64;

                rect.min[axis_idx] = rect.min[axis_idx].min(val as f32);
                rect.max[axis_idx] = rect.max[axis_idx].max(val as f32);
            }
        }

        rect.min = (rect.min / 16.).floor();
        rect.max = (rect.max / 16.).ceil();

        Self {
            texture_mins: (rect.min * 16.).as_u16vec2(),
            extents: (rect.size() * 16.).as_u16vec2(),
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct BspTexInfo {
    pub u_axis: Vec3,
    pub u_offset: f32,

    pub v_axis: Vec3,
    pub v_offset: f32,

    pub texture_idx: u32,
    pub flags: u32,
}
impl_bsp_read_simple!(BspTexInfo, u_axis, u_offset, v_axis, v_offset, texture_idx, flags);

#[derive(Debug)]
#[repr(C)]
pub struct BspModel {
    pub bound: BoundingBox,
    /// Origin of model, usually (0,0,0)
    pub origin: Vec3,

    pub head_node: [u32; 4],

    pub visleafs: u32,
    pub first_face: u32,
    pub num_faces: u32,
}
impl_bsp_read_simple!(BspModel, bound, origin, head_node, visleafs, first_face, num_faces);

#[derive(Debug)]
#[repr(C)]
pub struct BspPlane {
    pub normal: Vec3,
    pub dist: f32,
    /// Not really sure what this is, not used anywhere
    pub ty: u32,
}
impl_bsp_read_simple!(BspPlane, normal, dist, ty);

pub fn read_texture_lump(reader: &mut ByteReader) -> io::Result<Vec<Option<BspTexture>>> {
    let mut textures = Vec::new();
    let num_mip_textures: u32 = reader.read()?;

    for _ in 0..num_mip_textures {
        let offset: i32 = reader.read()?;
        if offset.is_negative() {
            textures.push(None);
            continue;
        }
        textures.push(Some(BspTexture::bsp_read(&mut ByteReader { bytes: reader.bytes, pos: offset as usize })?));
    }

    Ok(textures)
}

pub struct BspTexture {
    pub header: BspTextureHeader,
    pub data: Option<Vec<u8>>,
}
impl BspRead for BspTexture {
    fn bsp_read(reader: &mut ByteReader) -> io::Result<Self> {
        // TODO animated textures and the like
        let start_pos = reader.pos;
        let header: BspTextureHeader = reader.read()?;
        
        // From my testing, it seems the data starts at the end of the header, but this is just making sure
        reader.pos = start_pos + header.offset_full as usize;

        let data = if header.offset_full == 0 { None } else {
            Some(reader.read_bytes(header.width as usize * header.height as usize).map_err(add_msg!("Reading texture with header {header:#?}"))?.to_vec())
        };

        Ok(Self { header, data })
    }
}

#[derive(Debug, Clone)]
pub struct BspTextureHeader {
    /// Ascii characters
    // pub texture_name: [u8; 16],
    pub name: FixedStr<16>,

    pub width: u32,
    pub height: u32,

    pub offset_full: u32,
    #[allow(unused)] pub offset_half: u32,
    #[allow(unused)] pub offset_quarter: u32,
    #[allow(unused)] pub offset_eighth: u32,
}
impl_bsp_read_simple!(BspTextureHeader, name, width, height, offset_full, offset_half, offset_quarter, offset_eighth);

pub enum BspLighting {
    White(Vec<u8>),
    Colored(Vec<[u8; 3]>),
}
impl BspLighting {
    pub fn read_lit(data: &[u8]) -> io::Result<Self> {
        let mut reader = ByteReader::new(data);
        
        let magic = reader.read_bytes(4)?;
        if magic != b"QLIT" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Wrong magic number! Expected QLIT, found {}", display_magic_number(magic))));
        }

        let _version: i32 = reader.read()?;

        if data[reader.pos..].len() % 3 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Invalid color data, size {} is not devisable by 3!", data[reader.pos..].len())));
        }

        Ok(Self::Colored(data[reader.pos..].chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect()))
    }

    /// Convince function to get a location as an RGB color.
    pub fn get(&self, i: usize) -> Option<[u8; 3]> {
        match self {
            Self::White(v) => {
                let v = *v.get(i)?;
                Some([v, v, v])
            },
            Self::Colored(v) => v.get(i).copied(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::White(vec) => vec.len(),
            Self::Colored(vec) => vec.len(),
        }
    }
}