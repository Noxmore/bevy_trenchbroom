use super::*;
use crate::*;

use std::future::Future;

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    utils::ConditionalSendFuture,
};

pub struct MapLoader {
    pub server: TrenchBroomServer,
}
impl FromWorld for MapLoader {
    fn from_world(world: &mut World) -> Self {
        Self {
            server: world.resource::<TrenchBroomServer>().clone(),
        }
    }
}
impl AssetLoader for MapLoader {
    type Asset = Map;
    type Settings = ();
    type Error = io::Error;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext,
    ) -> impl ConditionalSendFuture
           + Future<Output = Result<<Self as AssetLoader>::Asset, <Self as AssetLoader>::Error>>
    {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            Ok(qmap_to_map(
                parse_qmap(&bytes)?,
                load_context.path().to_string_lossy().into(),
                &self.server.config,
                |_| Ok(()),
            )?)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["map"]
    }
}

#[test]
fn map_loading() {
    let mut app = App::new();

    app.add_plugins((
        AssetPlugin::default(),
        TaskPoolPlugin::default(),
        TrenchBroomPlugin::new(default()),
    ))
    .init_asset::<Map>()
    .init_asset_loader::<MapLoader>();

    let map_handle = app
        .world()
        .resource::<AssetServer>()
        .load::<Map>("maps/example.map");

    for _ in 0..1000 {
        match app
            .world()
            .resource::<AssetServer>()
            .load_state(&map_handle)
        {
            bevy::asset::LoadState::Loaded => return,
            bevy::asset::LoadState::Failed(err) => panic!("{err}"),
            _ => std::thread::sleep(std::time::Duration::from_millis(5)),
        }

        app.update();
    }
    panic!("no loaded");
}
