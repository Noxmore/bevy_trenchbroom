use brush::Brush;
#[cfg(feature = "bsp")]
use bsp::BspBrushesAsset;
#[cfg(all(feature = "client", feature = "bsp"))]
use bsp::lighting::AnimatedLighting;

use crate::*;

/// A good starting threshold in radians for interpolating similar normals, creating smoother curved surfaces.
pub const DEFAULT_NORMAL_SMOOTH_THRESHOLD: f32 = std::f32::consts::FRAC_PI_4;

pub struct GeometryPlugin;
impl Plugin for GeometryPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.init_asset::<BrushList>()
			.register_type::<Brushes>()
			.register_type::<MapGeometry>()
		;
	}
}

/// Contains the brushes that a solid entity is made of.
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
#[require(Transform)]
pub enum Brushes {
	/// Brushes are stored directly in the component itself, useful if you need to dynamically edit brushes.
	///
	/// NOTE: Dynamic brush mesh generation is not officially supported. ([see issue](https://github.com/Noxmore/bevy_trenchbroom/issues/25))
	Owned(BrushList),
	/// Reads an asset instead for completely static geometry.
	Shared(Handle<BrushList>),
	/// Used with the `BRUSHLIST` BSPX lump. Collision only.
	#[cfg(feature = "bsp")]
	Bsp(Handle<BspBrushesAsset>),
}

#[derive(Asset, Reflect, Debug, Clone)]
pub struct BrushList(pub Vec<Brush>);
impl std::ops::Deref for BrushList {
	type Target = [Brush];

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Reflect, Debug, Clone, PartialEq, Eq)]
pub struct MapGeometryTexture {
	pub name: String,
	pub material: Handle<GenericMaterial>,
	#[cfg(all(feature = "client", feature = "bsp"))]
	pub lightmap: Option<Handle<AnimatedLighting>>,
	/// If the texture should be full-bright
	#[cfg(feature = "bsp")]
	pub flags: BspTexFlags,
}

/// Marker component that marks meshes as level geometry produced by brushes.
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct MapGeometry;
