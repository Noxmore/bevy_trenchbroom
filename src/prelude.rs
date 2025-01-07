pub(crate) use bevy::math::*;
pub(crate) use bevy::prelude::*;
pub(crate) use bevy::render::primitives::Aabb;
pub(crate) use bevy::pbr::irradiance_volume::IrradianceVolume;
pub(crate) use default_struct_builder::*;
pub(crate) use indexmap::*;
pub(crate) use itertools::*;
pub(crate) use serde::*;
pub(crate) use serde::de::DeserializeOwned;
pub(crate) use nil::prelude::*;
pub(crate) use nil::std_prelude::*;
pub(crate) use thiserror::Error;
pub(crate) use q1bsp::prelude::*;
pub(crate) use bevy_materialize::{prelude::*, ErasedMaterialHandle};
pub(crate) use anyhow::{anyhow, Context};

pub use q1bsp::{self, Palette, QUAKE_PALETTE, data::bsp::LightmapStyle, mesh::lighting::{Lightmaps, LightmapAtlas, ComputeLightmapSettings}};
pub use bevy_materialize::prelude::*;

// TODO prelude should probably be more specific with what it re-exports
pub use crate::{
    brush::*,
    config::*,
    load::*,
    util::*,
    special_textures::*,
    bsp_lighting::*,
    *,
};

pub use bevy_trenchbroom_macros::*;