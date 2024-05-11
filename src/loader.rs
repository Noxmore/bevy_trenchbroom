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
                    ent_index: Some(i),
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

            Ok(map)
        })
    }
    fn extensions(&self) -> &[&str] {
        &["map"]
    }
}
