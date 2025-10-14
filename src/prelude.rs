#[cfg(not(feature = "client"))]
pub(crate) use crate::util::{Aabb, Mesh3d};
pub(crate) use anyhow::anyhow;
#[cfg(feature = "client")]
pub(crate) use bevy::camera::primitives::Aabb;
#[cfg(all(feature = "client", feature = "bsp"))]
pub(crate) use bevy::light::IrradianceVolume;
pub(crate) use bevy::math::*;
pub(crate) use bevy::platform::collections::HashMap;
pub(crate) use bevy::prelude::*;
pub(crate) use bevy_mesh::Mesh;
pub(crate) use default_struct_builder::*;
pub(crate) use itertools::*;
pub(crate) use nil::prelude::*;
pub(crate) use nil::std_prelude::*;
#[cfg(feature = "bsp")]
pub(crate) use qbsp::prelude::*;
pub(crate) use serde::*;
pub(crate) use thiserror::Error;

pub use bevy_materialize::prelude::*;
#[cfg(feature = "bsp")]
pub use qbsp::{
	self, Palette, QUAKE_PALETTE,
	data::bsp::{BspTexFlags, LightmapStyle},
	mesh::lightmap::{ComputeLightmapSettings, LightmapAtlas},
};

#[cfg(feature = "physics-integration")]
pub use crate::physics::TrenchBroomPhysicsPlugin;

#[cfg(all(feature = "client", feature = "bsp"))]
pub use crate::bsp::{
	lighting::{LightingAnimator, LightingAnimators},
	loader::IrradianceVolumeMultipliers,
};
pub use crate::{
	TrenchBroomPlugins, TrenchBroomServer,
	class::{
		QuakeClass, QuakeClassAppExt, ReflectQuakeClass,
		builtin::{Target, Targetable},
		spawn_hooks::*,
	},
	config::TrenchBroomConfig,
	qmap::QuakeMapEntity,
	util::IsSceneWorld,
};

pub use bevy_trenchbroom_macros::*;
