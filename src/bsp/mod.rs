#[cfg(feature = "client")]
pub mod lighting;
pub mod loader;

use brush::{BrushPlane, ConvexHull};
use class::ErasedQuakeClass;
use config::{EmbeddedTextureLoadView, TextureLoadView};
use geometry::{Brushes, MapGeometryTexture};
#[cfg(feature = "client")]
use lighting::AnimatedLighting;
use loader::BspLoader;
use qmap::{QuakeMapEntities, QuakeMapEntity};

use crate::{util::BevyTrenchbroomCoordinateConversions, *};

pub struct BspPlugin;
impl Plugin for BspPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.init_asset::<BspBrushesAsset>()
			.init_asset::<Bsp>()
			.init_asset_loader::<BspLoader>()
		;

		#[cfg(feature = "client")]
		if !app.world().resource::<TrenchBroomServer>().config.no_bsp_lighting {
			app.add_plugins(lighting::BspLightingPlugin);
		}
	}
}

pub static GENERIC_MATERIAL_PREFIX: &str = "GenericMaterial_";
pub static TEXTURE_PREFIX: &str = "Texture_";

/// Quake level loaded from a `.bsp` file.
#[derive(Asset, Reflect, Debug)]
pub struct Bsp {
	/// The main scene of everything put together.
	pub scene: Handle<Scene>,
	pub embedded_textures: HashMap<String, BspEmbeddedTexture>,
	#[cfg(feature = "client")]
	pub lightmap: Option<Handle<AnimatedLighting>>,
	#[cfg(feature = "client")]
	pub irradiance_volume: Option<Handle<AnimatedLighting>>,
	/// Models for brush entities (world geometry).
	pub models: Vec<BspModel>,
	/// The source data this BSP's assets was created from.
	pub data: BspData,
	/// The entities parsed from the map that was used to construct the scene.
	pub entities: QuakeMapEntities,
}

/// Geometry and brushes of a `SolidClass` entity.
#[derive(Reflect, Debug)]
pub struct BspModel {
	/// Maps texture names to mesh handles.
	pub meshes: Vec<(String, Handle<Mesh>)>,

	/// If the BSP contains the `BRUSHLIST` BSPX lump, this will be [`Some`] containing a handle to the brushes for this model.
	pub brushes: Option<Handle<BspBrushesAsset>>,
}

/// Wrapper for a `Vec<`[`BspBrush`]`>` in an asset so that it can be easily referenced from other places without referencing the [`Bsp`] (such as in the [`Bsp`]'s scene).
#[derive(Asset, Reflect, Debug, Clone, Default)]
pub struct BspBrushesAsset {
	pub brushes: Vec<BspBrush>,
}

/// Like a [`Brush`](crate::brush::Brush), but only contains the hull geometry, no texture information.
#[derive(Reflect, Debug, Clone, Default)]
pub struct BspBrush {
	pub planes: Vec<BrushPlane>,
}

impl ConvexHull for BspBrush {
	#[inline]
	fn plane_count(&self) -> usize {
		self.planes.len()
	}

	#[inline]
	fn planes(&self) -> impl Iterator<Item = &BrushPlane> + Clone {
		self.planes.iter()
	}
}

/// A reference to a texture loaded from a BSP file. Stores the handle to the [`Image`], and to the [`GenericMaterial`] that will be applied to mesh entities.
#[derive(Reflect, Debug)]
pub struct BspEmbeddedTexture {
	pub image: Handle<Image>,
	pub material: Handle<GenericMaterial>,
}

fn get_model_idx(map_entity: &QuakeMapEntity, class: &ErasedQuakeClass) -> Option<usize> {
	// Worldspawn always has model 0
	if class.info.name == "worldspawn" {
		return Some(0);
	}

	let model_property = map_entity.get::<String>("model").ok()?;
	let model_property_trimmed = model_property.trim_start_matches('*');
	// If there wasn't a * at the start, this is invalid
	if model_property_trimmed == model_property {
		return None;
	}
	model_property_trimmed.parse::<usize>().ok()
}
