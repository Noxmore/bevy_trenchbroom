use bevy::render::{render_asset::RenderAssetUsages, render_resource::Extent3d};

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
    pub lighting: Option<BspLighting>,

    /// Textures from 
    pub embedded_textures: HashMap<String, Handle<StandardMaterial>>,
}
impl Lumps {
    pub fn read(data: &[u8], lump_entries: &[LumpEntry; LUMP_COUNT], load_context: &mut LoadContext) -> io::Result<Self> {
        let lit_path = TrenchBroomConfigMirrorGuard::get().assets_path.join(load_context.path().with_extension("lit"));
        
        let mut lumps = Self {
            vertices: read_lump(data, &lump_entries, Lump::Vertices)?,
            planes: read_lump(data, &lump_entries, Lump::Planes)?,
            edges: read_lump(data, &lump_entries, Lump::Edges)?,
            surface_edges: read_lump(data, &lump_entries, Lump::SurfEdges)?,
            faces: read_lump(data, &lump_entries, Lump::Faces)?,
            tex_info: read_lump(data, &lump_entries, Lump::TexInfo)?,
            models: read_lump(data, &lump_entries, Lump::Models)?,
            textures: read_texture_lump(&mut ByteReader::new(lump_entries[Lump::Textures as usize].get(data)?)).map_err(add_msg!("Reading texture lump"))?,
            lighting: if lit_path.exists() {
                Some(BspLighting::read_lit(&fs::read(lit_path).map_err(add_msg!("Reading .lit file"))?).map_err(add_msg!("Parsing .lit file"))?)
                // TODO BSPX (DECOUPLED_LM && RGBLIGHTING)
            } else {
                let lighting = lump_entries[Lump::Lighting as usize].get(data)?;

                if lighting.is_empty() {
                    None
                } else {
                    Some(BspLighting::White(lighting.to_vec()))
                }
            },

            embedded_textures: HashMap::new(),
        };

        // println!("{:x}", lump_entries[Lump::Lighting as usize].offset);

        // If any texture is embedded, let's load the palette
        let palette: Option<[[u8; 3]; 256]> = if lumps.textures.iter().flatten().any(|texture| texture.data.is_some()) {
            let mirror = TrenchBroomConfigMirrorGuard::get();
            let path = mirror.assets_path.join(&mirror.texture_palette);
            let palette_data = fs::read(&path).map_err(add_msg!("Reading {}", path.display()))?;

            if palette_data.len() != 768 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Invalid palette from {} length, expected 768, found {}", path.display(), palette_data.len())));
            }
            
            Some(palette_data.chunks_exact(3).map(|col| [col[0], col[1], col[2]]).collect_vec().try_into().unwrap())
        } else {
            None
        };
        
        // Read embedded textures
        lumps.embedded_textures = lumps.textures.par_iter().flatten().filter(|texture| texture.data.is_some()).map(|texture| {
            let Some(data) = &texture.data else { unreachable!() };
            // Since this texture is embedded, palette must be Some
            let palette = palette.as_ref().unwrap();

            let mut alpha_mode = AlphaMode::Opaque;
            let image = Image::new(
                Extent3d { width: texture.header.width, height: texture.header.height, depth_or_array_layers: 1 },
                bevy::render::render_resource::TextureDimension::D2,
                data.iter().copied().map(|v| {
                    // According to https://quakewiki.org/wiki/Quake_palette, this is a special case, allowing transparency
                    if v == 255 {
                        // Only set alpha_mode to mask if we actually use transparency
                        alpha_mode = AlphaMode::Mask(0.5);
                        return [0, 0, 0, 0];
                    }
                    let [r, g, b] = palette[v as usize];
                    [r, g, b, 255]
                }).flatten().collect_vec(),
                bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                RenderAssetUsages::all(),
            );
            
            let texture_name = texture.header.name.as_str().to_string();
            
            (
                texture_name.clone(),
                image,
                alpha_mode,
            )
        }).collect::<Vec<_>>().into_iter().map(|(name, image, alpha_mode)| {
            // For adding assets, we have to do that synchronously
            let image_handle = load_context.add_labeled_asset(format!("{name}_color"), image);
            (
                name.clone(),
                load_context.add_labeled_asset(name, StandardMaterial {
                    base_color_texture: Some(image_handle),
                    perceptual_roughness: 1.,
                    alpha_mode,
                    lightmap_exposure: DEFAULT_LIGHTMAP_EXPOSURE,
                    ..default()
                }),
            )
        }).collect();
        
        Ok(lumps)
    }
}