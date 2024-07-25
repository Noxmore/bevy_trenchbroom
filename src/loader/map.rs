use crate::*;
use super::*;

use std::future::Future;

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    utils::ConditionalSendFuture,
};

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
        load_context: &'a mut LoadContext,
    ) -> impl ConditionalSendFuture + Future<Output = Result<<Self as AssetLoader>::Asset, <Self as AssetLoader>::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;

            Ok(qmap_to_map(parse_qmap(&bytes)?, load_context.path().to_string_lossy().into())?)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["map"]
    }
}