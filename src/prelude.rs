#[cfg(not(feature = "client"))]
pub(crate) use crate::util::{Aabb, Mesh3d};
pub(crate) use anyhow::anyhow;
pub(crate) use bevy::math::*;
#[cfg(feature = "client")]
pub(crate) use bevy::pbr::irradiance_volume::IrradianceVolume;
pub(crate) use bevy::prelude::*;
#[cfg(feature = "client")]
pub(crate) use bevy::render::primitives::Aabb;
pub(crate) use bevy_mesh::Mesh;
pub(crate) use default_struct_builder::*;
pub(crate) use itertools::*;
pub(crate) use nil::prelude::*;
pub(crate) use nil::std_prelude::*;
pub(crate) use qbsp::prelude::*;
pub(crate) use serde::*;
pub(crate) use thiserror::Error;

pub use bevy_materialize::prelude::*;
pub use qbsp::{
	self,
	data::bsp::{BspTexFlags, LightmapStyle},
	mesh::lighting::{ComputeLightmapSettings, LightmapAtlas, Lightmaps},
	Palette, QUAKE_PALETTE,
};

#[cfg(feature = "client")]
pub use crate::{
	bsp::{
		lighting::{LightingAnimator, LightingAnimators},
		loader::IrradianceVolumeMultipliers,
	},
	special_textures::{LiquidMaterial, LiquidMaterialExt, QuakeSkyMaterial},
};
pub use crate::{
	class::builtin::{Target, Targetable},
	config::TrenchBroomConfig,
	geometry::{GeometryProvider, GeometryProviderView},
	qmap::QuakeMapEntity,
	util::{IsSceneWorld, TrenchBroomGltfRotationFix},
	TrenchBroomPlugin, TrenchBroomServer,
};

pub use bevy_trenchbroom_macros::*;
