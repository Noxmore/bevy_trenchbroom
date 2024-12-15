pub mod bsp;
pub mod map;

use crate::*;

pub(self) fn parse_qmap(bytes: &[u8]) -> io::Result<quake_util::qmap::QuakeMap> {
    quake_util::qmap::parse(&mut io::BufReader::new(bytes))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

pub(self) fn qmap_to_map(qmap: quake_util::qmap::QuakeMap, name: String, config: &TrenchBroomConfig, mut modifier: impl FnMut(&mut MapEntity) -> io::Result<()>) -> io::Result<Map> {
    let mut map = Map::default();
    map.name = name;

    map.entities.reserve(qmap.entities.len());

    for (i, ent) in qmap.entities.into_iter().enumerate() {
        let properties = ent
            .edict
            .into_iter()
            .map(|(k, v)| (k.to_string_lossy().into(), v.to_string_lossy().into()))
            .collect::<HashMap<String, String>>();

        let mut entity = MapEntity {
            ent_index: Some(i),
            properties,
            geometry: MapEntityGeometry::Map(ent.brushes.iter().map(|brush | Brush::from_quake_util(brush, config)).collect()),
        };

        modifier(&mut entity)?;

        map.entities.push(Arc::new(entity));
    }

    if map.worldspawn().is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "worldspawn not defined",
        ));
    }

    Ok(map)
}