use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    utils::BoxedFuture,
};

use crate::*;

#[derive(Default)]
pub struct MapLoader;
impl AssetLoader for MapLoader {
    type Asset = Map;
    type Settings = ();
    type Error = io::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        ctx: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            let mut map = Map::default();
            map.name = ctx.path().to_string_lossy().into();
            let qmap = quake_util::qmap::parse(&mut io::BufReader::new(bytes.as_slice()))
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

            map.entities.reserve(qmap.entities.len());

            for (i, ent) in qmap.entities.into_iter().enumerate() {
                let properties = ent
                    .edict
                    .into_iter()
                    .map(|(k, v)| (k.to_string_lossy().into(), v.to_string_lossy().into()))
                    .collect::<HashMap<String, String>>();

                let entity = MapEntity {
                    ent_index: i,
                    properties,
                    brushes: ent.brushes.iter().map(Brush::from_quake_util).collect(),
                };

                map.entities.push(entity);
            }

            if map.worldspawn().is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "worldspawn not defined",
                ));
            }

            // Start preloading some properties
            let texture_root = trenchbroom_config_mirror!().texture_root.clone();
            let assets_path = trenchbroom_config_mirror!().assets_path.clone();
            let mut preloaded_textures: Vec<&str> = default();
            for ent in &map.entities {
                for brush in &ent.brushes {
                    for surface in &brush.surfaces {
                        if preloaded_textures.contains(&surface.texture.as_str()) { continue }

                        let mat_properties_path = PathBuf::from(&surface.texture).with_extension(MATERIAL_PROPERTIES_EXTENSION);
                        if assets_path.join(&texture_root).join(&mat_properties_path).exists() {
                            map.material_properties_map.insert(surface.texture.to_string(), ctx.load::<MaterialProperties>(texture_root.join(&mat_properties_path)));
                            preloaded_textures.push(&surface.texture);
                        }
                    }
                }
            }

            Ok(map)
        })
    }
    fn extensions(&self) -> &[&str] {
        &["map"]
    }
}
