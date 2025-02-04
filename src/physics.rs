use crate::*;
use brush::ConvexHull;
use bsp::BspBrushesAsset;
use geometry::{BrushList, Brushes};

#[cfg(feature = "rapier")]
use bevy_rapier3d::prelude::*;

#[cfg(feature = "avian")]
use avian3d::prelude::*;

/// Automatically creates convex colliders for entities with [Brushes].
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct ConvexCollision;

/// Automatically creates trimesh colliders for entities with [Mesh3d].
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct TrimeshCollision;

pub type BrushVertices = Vec<Vec3>;

/// This *abomination* of a function attempts to calculate vertices on the brushes contained within for use in physics, if it can find said brushes.
///
/// If it can't find them (like if the asset isn't loaded), returns [`None`].
pub fn calculate_brushes_vertices<'l, 'w: 'l>(
	brushes: &Brushes,
	brush_lists: &'w Assets<BrushList>,
	bsp_brushes: &'w Assets<BspBrushesAsset>,
) -> Option<Vec<BrushVertices>> {
	fn extract_vertices<T: ConvexHull>(brush: &T) -> Vec<Vec3> {
		brush.calculate_vertices().map(|(position, _)| position.as_vec3()).collect()
	}

	match brushes {
		Brushes::Owned(list) => Some(list.iter().map(extract_vertices).collect()),
		Brushes::Shared(handle) => brush_lists.get(handle).map(|list| list.iter().map(extract_vertices).collect()),
		Brushes::Bsp(handle) => bsp_brushes
			.get(handle)
			.map(|brushes_asset| brushes_asset.brushes.iter().map(extract_vertices).collect()),
	}
}

pub(crate) struct PhysicsPlugin;
impl Plugin for PhysicsPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.register_type::<ConvexCollision>()
			.register_type::<TrimeshCollision>()
			.add_systems(Update, (
				Self::create_convex_colliders,
				Self::create_trimesh_colliders,
			))
		;
	}
}
impl PhysicsPlugin {
	#[cfg(feature = "rapier")]
	pub fn create_convex_colliders(
		mut commands: Commands,
		query: Query<(Entity, &Brushes), (With<ConvexCollision>, Without<Collider>)>,
		brush_lists: Res<Assets<BrushList>>,
		bsp_brush_assets: Res<Assets<BspBrushesAsset>>,
	) {
		for (entity, brushes) in &query {
			let mut colliders = Vec::new();
			let Some(brush_vertices) = calculate_brushes_vertices(brushes, &brush_lists, &bsp_brush_assets) else { continue };

			for (brush_idx, vertices) in brush_vertices.into_iter().enumerate() {
				if vertices.is_empty() {
					continue;
				}

				let Some(collider) = Collider::convex_hull(&vertices) else {
					error!("Entity {entity}'s brush (index {brush_idx}) is invalid (non-convex), and a collider could not be computed for it!");
					continue;
				};
				colliders.push((Vec3::ZERO, Quat::IDENTITY, collider));
			}

			commands.entity(entity).insert(Collider::compound(colliders));
		}
	}

	#[cfg(feature = "avian")]
	pub fn create_convex_colliders(
		mut commands: Commands,
		query: Query<(Entity, &Brushes), (With<ConvexCollision>, Without<Collider>)>,
		brush_lists: Res<Assets<BrushList>>,
		bsp_brush_assets: Res<Assets<BspBrushesAsset>>,
	) {
		for (entity, brushes) in &query {
			let mut colliders = Vec::new();
			let Some(brush_vertices) = calculate_brushes_vertices(brushes, &brush_lists, &bsp_brush_assets) else { continue };

			for (brush_idx, vertices) in brush_vertices.into_iter().enumerate() {
				if vertices.is_empty() {
					continue;
				}

				let Some(collider) = Collider::convex_hull(vertices) else {
					error!("Entity {entity}'s brush (index {brush_idx}) is invalid (non-convex), and a collider could not be computed for it!");
					continue;
				};
				colliders.push((Vec3::ZERO, Quat::IDENTITY, collider));
			}

			commands
				.entity(entity)
				.insert(Collider::compound(colliders))
				.insert_if_new(RigidBody::Static);
		}
	}

	#[cfg(feature = "rapier")]
	pub fn create_trimesh_colliders(
		mut commands: Commands,
		query: Query<(Entity, &Mesh3d), (With<TrimeshCollision>, Without<Collider>)>,
		meshes: Res<Assets<Mesh>>,
	) {
		for (entity, mesh3d) in &query {
			let Some(mesh) = meshes.get(mesh3d.id()) else { continue };

			let Some(collider) = Collider::from_bevy_mesh(mesh, &ComputedColliderShape::TriMesh(default())) else {
				error!("Entity {entity} has TrimeshCollision, but index buffer or vertex buffer of the mesh are in an incompatible format. TrimeshCollision component removed to not clutter logs.");
				commands.entity(entity).remove::<TrimeshCollision>();
				continue;
			};

			// TODO test if we need a RigidBody::Fixed
			commands.entity(entity).insert(collider);
		}
	}

	#[cfg(feature = "avian")]
	pub fn create_trimesh_colliders(
		mut commands: Commands,
		query: Query<(Entity, &Mesh3d), (With<TrimeshCollision>, Without<Collider>)>,
		meshes: Res<Assets<Mesh>>,
	) {
		for (entity, mesh3d) in &query {
			let Some(mesh) = meshes.get(mesh3d.id()) else { continue };

			let Some(collider) = Collider::trimesh_from_mesh(mesh) else {
				error!("Entity {entity} has TrimeshCollision, but index buffer or vertex buffer of the mesh are in an incompatible format. TrimeshCollision component removed to not clutter logs.");
				commands.entity(entity).remove::<TrimeshCollision>();
				continue;
			};

			commands.entity(entity).insert(collider).insert_if_new(RigidBody::Static);
		}
	}
}

// TODO test collider creation
