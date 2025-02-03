use crate::*;
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
	) {
		for (entity, brushes) in &query {
			let mut colliders = Vec::new();
			let Some(brushes) = brushes.get(&brush_lists) else { continue };

			for (brush_idx, brush) in brushes.iter().enumerate() {
				let vertices: Vec<Vec3> = brush.calculate_vertices().into_iter().map(|(pos, _)| pos.as_vec3()).collect();

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
	) {
		for (entity, brushes) in &query {
			let mut colliders = Vec::new();
			let Some(brushes) = brushes.get(&brush_lists) else { continue };

			for (brush_idx, brush) in brushes.iter().enumerate() {
				let vertices: Vec<Vec3> = brush.calculate_vertices().into_iter().map(|(pos, _)| pos.as_vec3()).collect();

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
