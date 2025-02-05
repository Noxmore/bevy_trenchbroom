pub(crate) use anyhow::anyhow;
pub(crate) use bevy::math::*;
pub(crate) use bevy::pbr::irradiance_volume::IrradianceVolume;
pub(crate) use bevy::prelude::*;
pub(crate) use bevy::render::primitives::Aabb;
pub(crate) use default_struct_builder::*;
pub(crate) use indexmap::*;
pub(crate) use itertools::*;
pub(crate) use nil::prelude::*;
pub(crate) use nil::std_prelude::*;
pub(crate) use qbsp::prelude::*;
pub(crate) use serde::*;
pub(crate) use thiserror::Error;

pub use bevy_materialize::prelude::*;
pub use qbsp::{
	self,
	data::bsp::LightmapStyle,
	mesh::lighting::{ComputeLightmapSettings, LightmapAtlas, Lightmaps},
	Palette, QUAKE_PALETTE,
};

pub use crate::{
	bsp::{
		lighting::{LightmapAnimator, LightmapAnimators},
		util::IrradianceVolumeMultipliers,
	},
	config::TrenchBroomConfig,
	geometry::{GeometryProvider, GeometryProviderView},
	qmap::QuakeMapEntity,
	special_textures::{LiquidMaterial, LiquidMaterialExt, QuakeSkyMaterial, SpecialTexturesConfig},
	util::{repeating_image_sampler, IsSceneWorld, TrenchBroomGltfRotationFix},
	TrenchBroomPlugin, TrenchBroomServer,
};

pub use bevy_trenchbroom_macros::*;
