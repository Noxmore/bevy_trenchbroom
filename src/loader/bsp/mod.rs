pub(self) mod lumps;
pub(self) use lumps::*;
pub(self) mod lump_data;
pub(self) use lump_data::*;

use crate::*;
use super::*;

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    utils::ConditionalSendFuture,
};

pub static QUAKE_PALETTE: Lazy<[[u8; 3]; 256]> = Lazy::new(|| {
    include_str!("quake_palette.txt").split(',').map(|col| Srgba::hex(col.trim()).unwrap().to_u8_array_no_alpha()).collect_vec().try_into().unwrap()
});

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

            
            let lumps = Lumps::read(&bytes, &lump_entries, load_context)?;
            

            // {
            //     let entry = &lump_entries[Lump::Textures as usize];
            //     println!("at {:x}", entry.offset);
            //     println!("to {:x}", entry.offset + entry.len);
            //     let lump_data = entry.get(&bytes)?;
            //     let mut reader = ByteReader::new(lump_data);

            //     let header: BspTextureHeader = reader.read()?;

            //     println!("{header:#?}");

            //     for offset in header.offsets {
            //         if offset.is_negative() { continue }

            //         reader.pos = offset as usize;

            //         let mip_texture: BspMipTexture = reader.read()?;

            //         println!("{mip_texture:#?}");
            //     }
            // }

            // println!("{:#?}", lumps.textures);

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

/// Like an [io::Cursor], but i don't have to constantly juggle buffers.
pub(self) struct ByteReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> ByteReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub fn read<T: BspRead>(&mut self) -> io::Result<T> {
        T::bsp_read(self)
    }

    pub fn read_bytes(&mut self, count: usize) -> io::Result<&[u8]> {
        let (from, to) = (self.pos, self.pos + count);
        if to > self.bytes.len() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, format!("Tried to read bytes from {from} to {to} from buffer of size {}", self.bytes.len())));
        }
        let bytes = &self.bytes[from..to];
        self.pos += count;
        Ok(bytes)
    }
}



#[test]
fn quake_palette_loading() {
    // Initializes the lazy
    let _ = QUAKE_PALETTE.iter();
}

#[test]
fn bsp_loading() {
    let mut app = App::new();

    app
        .add_plugins((AssetPlugin::default(), TaskPoolPlugin::default(), TrenchBroomPlugin::new(default())))
        .init_asset::<Map>()
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
    panic!("no loaded");
}