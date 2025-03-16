use crate::*;
use bevy::ecs::{component::ComponentId, world::DeferredWorld};
use brush::ConvexHull;
use bsp::BspBrushesAsset;
use geometry::{BrushList, Brushes};

#[cfg(feature = "rapier")]
use bevy_rapier3d::prelude::*;

#[cfg(feature = "avian")]
use avian3d::prelude::*;

// We use component hooks rather than systems to ensure that colliders are available for things like observers.

/// Automatically creates convex colliders for entities with [`Brushes`].
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct ConvexCollision;
impl ConvexCollision {
	pub fn on_insert(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
		world.commands().queue(move |world: &mut World| {
			if world.get_entity(entity).is_err() {
				return;
			}

			let Some((brushes, transform)) = world.entity(entity).get_components::<(&Brushes, &Transform)>() else { return };

			let mut colliders = Vec::new();
			let Some(brush_vertices) = calculate_brushes_vertices(
				brushes,
				world.resource::<Assets<BrushList>>(),
				world.resource::<Assets<BspBrushesAsset>>(),
			) else {
				error!("Couldn't make collider for {entity}, brushes asset not found/loaded.");
				return;
			};

			for (brush_idx, vertices) in brush_vertices.into_iter().enumerate() {
				if vertices.is_empty() {
					continue;
				}

				macro_rules! fail {
					() => {
						error!("Entity {entity}'s brush (index {brush_idx}) is invalid (non-convex), and a collider could not be computed for it!");
						continue;
					};
				}

				#[cfg(feature = "avian")]
				let Some(collider) = Collider::convex_hull(vertices) else {
					fail!();
				};

				#[cfg(feature = "rapier")]
				let Some(collider) = Collider::convex_hull(&vertices) else {
					fail!();
				};

				#[rustfmt::skip]
				let position = if matches!(brushes, Brushes::Bsp(_)) { Vec3::ZERO } else { -transform.translation };
				colliders.push((position, Quat::IDENTITY, collider));
			}

			#[cfg(feature = "avian")]
			world
				.entity_mut(entity)
				.insert(Collider::compound(colliders))
				.insert_if_new(RigidBody::Static);

			#[cfg(feature = "rapier")]
			world.entity_mut(entity).insert(Collider::compound(colliders));
		});
	}
}

/// Automatically creates trimesh colliders for entities with [`Mesh3d`].
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct TrimeshCollision;
impl TrimeshCollision {
	pub fn on_insert(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
		world.commands().queue(move |world: &mut World| {
			if world.get_entity(entity).is_err() {
				return;
			}

			let Some(mesh3d) = world.entity(entity).get::<Mesh3d>() else { return };
			let Some(mesh) = world.resource::<Assets<Mesh>>().get(mesh3d.id()) else {
				error!("Couldn't make collider for {entity}, mesh not found/loaded.");
				return;
			};

			macro_rules! fail {
				() => {
					error!("Entity {entity} has TrimeshCollision, but index buffer or vertex buffer of the mesh are in an incompatible format.");
					return;
				};
			}

			#[cfg(feature = "avian")]
			{
				let Some(collider) = Collider::trimesh_from_mesh(mesh) else {
					fail!();
				};
				world.entity_mut(entity).insert(collider).insert_if_new(RigidBody::Static);
			}

			#[cfg(feature = "rapier")]
			{
				let Some(collider) = Collider::from_bevy_mesh(mesh, &ComputedColliderShape::TriMesh(default())) else {
					fail!();
				};
				world.entity_mut(entity).insert(collider);
			}
		});
	}
}

pub type BrushVertices = Vec<Vec3>;

/// Attempts to calculate vertices on the brushes contained within for use in physics, if it can find said brushes.
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
		;

		app.world_mut()
			.register_component_hooks::<TrimeshCollision>()
			.on_insert(TrimeshCollision::on_insert);

		app.world_mut()
			.register_component_hooks::<ConvexCollision>()
			.on_insert(ConvexCollision::on_insert);
	}
}
