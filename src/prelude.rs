pub(crate) use bevy::math::*;
pub(crate) use bevy::prelude::*;
pub(crate) use bevy::render::primitives::Aabb;
pub(crate) use bevy::pbr::irradiance_volume::IrradianceVolume;
pub(crate) use default_struct_builder::*;
pub(crate) use indexmap::*;
pub(crate) use itertools::*;
pub(crate) use serde::*;
pub(crate) use nil::prelude::*;
pub(crate) use nil::std_prelude::*;
pub(crate) use thiserror::Error;
pub(crate) use q1bsp::prelude::*;
pub(crate) use anyhow::anyhow;

pub use q1bsp::{self, Palette, QUAKE_PALETTE, data::bsp::LightmapStyle, mesh::lighting::{Lightmaps, LightmapAtlas, ComputeLightmapSettings}};
pub use bevy_materialize::prelude::*;

pub use crate::{
    TrenchBroomPlugin,
    TrenchBroomServer,
    bsp::{
        lighting::{
            LightmapAnimators,
            LightmapAnimator,
        },
        util::IrradianceVolumeMultipliers,
    },
    config::TrenchBroomConfig,
    geometry::{
        GeometryProvider,
        GeometryProviderView,
    },
    qmap::QuakeMapEntity,
    special_textures::{
        SpecialTexturesConfig,
        LiquidMaterial,
        LiquidMaterialExt,
        QuakeSkyMaterial,
    },
    util::{
        TrenchBroomGltfRotationFix,
        repeating_image_sampler,
    },
};

pub use bevy_trenchbroom_macros::*;