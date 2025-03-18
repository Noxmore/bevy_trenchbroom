use bevy::render::mesh::VertexAttributeValues;
use brush::Brush;
#[cfg(feature = "bevy_pbr")]
use bsp::lighting::{AnimatedLighting, AnimatedLightingHandle};
use bsp::BspBrushesAsset;
use qmap::QuakeMapEntity;

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
		;
	}
}

/// Contains the brushes that a solid entity is made of.
///
/// Unless [`Brushes::Bsp`], entities with this component have meshes generated for them at runtime.
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
#[require(Transform)]
pub enum Brushes {
	/// Brushes are stored directly in the component itself, useful if you need to dynamically edit brushes.
	///
	/// NOTE: TODO dynamic brush mesh generation currently does not work. ([see issue](https://github.com/Noxmore/bevy_trenchbroom/issues/25))
	Owned(BrushList),
	/// Reads an asset instead for completely static geometry.
	Shared(Handle<BrushList>),
	/// Used with the `BRUSHLIST` BSPX lump. Collision only.
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
	#[cfg(feature = "bevy_pbr")]
	pub lightmap: Option<Handle<AnimatedLighting>>,
	/// If the texture should be full-bright
	pub flags: BspTexFlags,
}

pub struct GeometryProviderMeshView<'l> {
	pub entity: Entity,
	pub mesh: &'l mut Mesh,
	pub texture: &'l mut MapGeometryTexture,
}

pub struct GeometryProviderView<'w, 'l> {
	pub world: &'w mut World,
	pub entity: Entity,
	pub tb_server: &'w TrenchBroomServer,
	pub map_entity: &'w QuakeMapEntity,
	pub map_entity_idx: usize,
	pub meshes: Vec<GeometryProviderMeshView<'l>>,
}

pub type GeometryProviderFn = dyn Fn(&mut GeometryProviderView) + Send + Sync;
pub type GeometryProviderFnOnce = dyn FnOnce(&mut GeometryProviderView) + Send + Sync;

#[derive(Default)]
pub struct GeometryProvider {
	pub providers: Vec<Box<GeometryProviderFnOnce>>,
}

impl GeometryProvider {
	pub fn new() -> Self {
		Self::default()
	}

	/// Add a function to the settings' spawner stack.
	pub fn push(mut self, provider: impl FnOnce(&mut GeometryProviderView) + Send + Sync + 'static) -> Self {
		self.providers.push(Box::new(provider));
		self
	}

	/// Any intersecting vertices where the angle between their normals in radians is less than [`DEFAULT_NORMAL_SMOOTH_THRESHOLD`] will have their normals interpolated, making curved surfaces look smooth.
	///
	/// Shorthand for `self.smooth_by_angle(DEFAULT_NORMAL_SMOOTH_THRESHOLD)` to reduce syntactic noise.
	pub fn smooth_by_default_angle(self) -> Self {
		self.smooth_by_angle(DEFAULT_NORMAL_SMOOTH_THRESHOLD)
	}

	/// Any intersecting vertices where the angle between their normals in radians is less than `normal_smooth_threshold` will have their normals interpolated, making curved surfaces look smooth.
	/// [`DEFAULT_NORMAL_SMOOTH_THRESHOLD`] is a good starting value for this, shorthanded by [`Self::smooth_by_default_angle`].
	///
	/// if `normal_smooth_threshold` is <= 0, nothing will happen.
	pub fn smooth_by_angle(self, normal_smooth_threshold: f32) -> Self {
		self.push(move |view| {
			if normal_smooth_threshold <= 0. {
				return; // The user doesn't want to smooth after all!
			}

			#[derive(Clone, Copy, PartialEq, Eq, Hash)]
			struct Vec3Ord([FloatOrd; 3]);

			// It's either a map or a doubly-connected edge list, the prior seems to work well enough.
			let mut vertex_map: HashMap<Vec3Ord, Vec<&mut [f32; 3]>> = default();


			let ent_index = view.map_entity_idx; // Borrow checker
			// We go through all the meshes and add all their normals into vertex_map
			for mesh_view in &mut view.meshes {
				// SAFETY: Getting ATTRIBUTE_POSITION and ATTRIBUTE_NORMAL gives us 2 different attributes, but the borrow checker doesn't know that!
				let mesh2 = unsafe { &mut *std::ptr::from_mut(&mut mesh_view.mesh) };

				let Some(positions) = mesh_view.mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(VertexAttributeValues::as_float3) else {
					error!("[entity {} (map entity {:?})] Tried to smooth by angle, but the ATTRIBUTE_POSITION doesn't exist on mesh!", mesh_view.entity, ent_index);
					return;
				};
				let positions_len = positions.len();

				let Some(normals) = mesh2.attribute_mut(Mesh::ATTRIBUTE_NORMAL).and_then(|values| match values {
					VertexAttributeValues::Float32x3(v) => Some(v),
					_ => None,
				}) else {
					error!("[entity {} (map entity {:?})] Tried to smooth by angle, but the ATTRIBUTE_NORMAL doesn't exist on mesh!", mesh_view.entity, ent_index);
					return;
				};
				let normals_len = normals.len();

				if normals_len != positions_len {
					error!("[entity {} (map entity {:?})] Tried to smooth by angle, but ATTRIBUTE_NORMAL len doesn't match ATTRIBUTE_POSITION len! ({} and {})", mesh_view.entity, ent_index, normals_len, positions_len);
					return;
				}

				for (i, normal) in normals.iter_mut().enumerate() {
					// Let's make this lower precision, just in case
					let position = Vec3Ord(positions[i].map(|v| FloatOrd((v * 10000.).round() / 10000.)));

					vertex_map.entry(position).or_default().push(normal);
				}
			}


			for (_position, mut normals) in vertex_map {
				use disjoint_sets::*;

				if normals.len() <= 1 { // There are no duplicates
					continue;
				}

				// Group normals to be smoothed
				let mut uf = UnionFind::new(normals.len());

				for ((a_i, a), (b_i, b)) in normals.iter().map(|v| Vec3::from(**v)).enumerate().tuple_combinations() {
					if a.angle_between(b) < normal_smooth_threshold {
						uf.union(a_i, b_i);
					}
				}

				// Put the groups into an easily iterable structure, then average the normals in each group
				let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
				for i in 0..normals.len() {
					let root = uf.find(i);
					groups.entry(root).or_default().push(i);
				}

				for (_, group) in groups {
					let new_normal = group.iter().map(|idx| Vec3::from(*normals[*idx])).sum::<Vec3>() / normals.len() as f32;

					for idx in group {
						*normals[idx] = new_normal.to_array();
					}
				}
			}
		})
	}

	/// Puts materials on mesh entities.
	pub fn render(self) -> Self {
		self.push(|view| {
			for mesh_view in &view.meshes {
				view.world
					.entity_mut(mesh_view.entity)
					.insert(GenericMaterial3d(mesh_view.texture.material.clone()));
			}
		})
	}

	/// Inserts lightmaps if available.
	#[cfg(feature = "bevy_pbr")]
	pub fn with_lightmaps(self) -> Self {
		self.push(|view| {
			for mesh_view in &view.meshes {
				let Some(animated_lighting_handle) = &mesh_view.texture.lightmap else { continue };

				view.world
					.entity_mut(mesh_view.entity)
					.insert(AnimatedLightingHandle(animated_lighting_handle.clone()));
			}
		})
	}
	#[cfg(not(feature = "bevy_pbr"))]
	pub fn with_lightmaps(self) -> Self {
		self
	}

	/// Inserts a bundle onto the entity.
	///
	/// This is a convenience function, you should generally use required components instead of this if at all possible.
	pub fn with(self, bundle: impl Bundle) -> Self {
		self.push(move |view| {
			view.world.entity_mut(view.entity).insert(bundle);
		})
	}
	/// Inserts a bundle onto all mesh entities.
	pub fn meshes_with(self, bundle: impl Bundle + Clone) -> Self {
		self.push(move |view| {
			for mesh_view in &view.meshes {
				view.world.entity_mut(mesh_view.entity).insert(bundle.clone());
			}
		})
	}

	/// Inserts trimesh colliders on each mesh of this entity. This means that brushes will be hollow. Not recommended to use on physics objects.
	#[cfg(any(feature = "rapier", feature = "avian"))]
	pub fn trimesh_collider(self) -> Self {
		self.meshes_with(physics::TrimeshCollision)
	}

	/// Inserts a compound collider of every brush in this entity into said entity. Brushes will be fully solid.
	#[cfg(any(feature = "rapier", feature = "avian"))]
	pub fn convex_collider(self) -> Self {
		self.with(physics::ConvexCollision)
	}
}
