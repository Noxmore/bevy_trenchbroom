use crate::*;
use super::*;

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
            Ok(Self { $($field: reader.read().map_err(add_msg!(concat!("Reading field ", stringify!($field), " on type ", stringify!($ty))))?),+ })
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
    /// Set if the normal is parallel to the plane normal (???)
    pub plane_side: u32,

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
        let start_pos = reader.pos;
        let header: BspTextureHeader = reader.read()?;
        // From my testing, it seems the data starts at the end of the header, but this is just making sure
        reader.pos = start_pos + header.offset_full as usize;

        let data = if header.offset_full == 0 { None } else {
            Some(reader.read_bytes(header.width as usize * header.height as usize)?.to_vec())
        };

        Ok(Self { header, data })
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct BspTextureHeader {
    /// Ascii characters
    // pub texture_name: [u8; 16],
    pub name: FixedStr<16>,

    pub width: u32,
    pub height: u32,

    pub offset_full: u32,
    pub offset_half: u32,
    pub offset_quarter: u32,
    pub offset_eighth: u32,
}
impl_bsp_read_simple!(BspTextureHeader, name, width, height, offset_full, offset_half, offset_quarter, offset_eighth);