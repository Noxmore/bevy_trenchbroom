use crate::*;
use brush::ConvexHull;
#[cfg(feature = "bsp")]
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

/// Automatically creates trimesh colliders for entities with [`Mesh3d`].
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct TrimeshCollision;

pub type BrushVertices = Vec<Vec3>;

/// Attempts to calculate vertices on the brushes contained within for use in physics, if it can find said brushes.
///
/// If it can't find them (like if the asset isn't loaded), returns [`None`].
pub fn calculate_brushes_vertices<'l, 'w: 'l>(
	brushes: &Brushes,
	brush_lists: &'w Assets<BrushList>,
	#[cfg(feature = "bsp")] bsp_brushes: &'w Assets<BspBrushesAsset>,
) -> Option<Vec<BrushVertices>> {
	fn extract_vertices<T: ConvexHull>(brush: &T) -> Vec<Vec3> {
		brush.calculate_vertices().map(|(position, _)| position.as_vec3()).collect()
	}

	match brushes {
		Brushes::Owned(list) => Some(list.iter().map(extract_vertices).collect()),
		Brushes::Shared(handle) => brush_lists.get(handle).map(|list| list.iter().map(extract_vertices).collect()),
		#[cfg(feature = "bsp")]
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

			.init_resource::<SceneCollidersReadyTests>()

			.add_systems(PostUpdate, (
				Self::add_convex_colliders,
				Self::add_trimesh_colliders,
				Self::trigger_scene_colliders_ready,
			).chain())
		;
	}
}
impl PhysicsPlugin {
	pub fn add_convex_colliders(
		mut commands: Commands,
		query: Query<(Entity, &Brushes, &Transform), (With<ConvexCollision>, Without<Collider>)>,
		brush_lists: Res<Assets<BrushList>>,
		#[cfg(feature = "bsp")] brush_assets: Res<Assets<BspBrushesAsset>>,
		mut tests: ResMut<SceneCollidersReadyTests>,
	) {
		#[allow(unused)]
		for (entity, brushes, transform) in &query {
			let mut colliders = Vec::new();
			let Some(brush_vertices) = calculate_brushes_vertices(
				brushes,
				&brush_lists,
				#[cfg(feature = "bsp")]
				&brush_assets,
			) else {
				continue;
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

				#[cfg(feature = "bsp")]
				#[rustfmt::skip]
				let position = if matches!(brushes, Brushes::Bsp(_)) { Vec3::ZERO } else { -transform.translation };
				#[cfg(not(feature = "bsp"))]
				let position = -transform.translation;
				colliders.push((position, Quat::IDENTITY, collider));
			}

			if colliders.is_empty() {
				error!(
					"No colliders produced by brushes for entity {entity}, removing ConvexCollision component. If this is expected behavior, make an issue and i will remove this message."
				);
				commands.entity(entity).remove::<ConvexCollision>();
				continue;
			}

			#[cfg(feature = "avian")]
			commands
				.entity(entity)
				.insert(Collider::compound(colliders))
				.insert_if_new(RigidBody::Static);

			#[cfg(feature = "rapier")]
			commands.entity(entity).insert(Collider::compound(colliders));

			tests.added_colliders_to_entities.insert(entity);
		}
	}

	pub fn add_trimesh_colliders(
		mut commands: Commands,
		query: Query<(Entity, &Mesh3d), (With<TrimeshCollision>, Without<Collider>)>,
		meshes: Res<Assets<Mesh>>,
		mut tests: ResMut<SceneCollidersReadyTests>,
	) {
		for (entity, mesh3d) in &query {
			let Some(mesh) = meshes.get(mesh3d.id()) else {
				continue;
			};

			macro_rules! fail {
				() => {
					error!("Entity {entity} has TrimeshCollision, but index buffer or vertex buffer of the mesh are in an incompatible format.");
					continue;
				};
			}

			#[cfg(feature = "avian")]
			{
				let Some(collider) = Collider::trimesh_from_mesh(mesh) else {
					fail!();
				};
				commands.entity(entity).insert(collider).insert_if_new(RigidBody::Static);
			}

			#[cfg(feature = "rapier")]
			{
				let Some(collider) = Collider::from_bevy_mesh(mesh, &ComputedColliderShape::TriMesh(default())) else {
					fail!();
				};
				commands.entity(entity).insert(collider);
			}

			tests.added_colliders_to_entities.insert(entity);
		}
	}

	pub fn trigger_scene_colliders_ready(
		mut commands: Commands,
		mut tests: ResMut<SceneCollidersReadyTests>,

		parent_query: Query<&ChildOf>,
		has_scene_root: Query<(), With<SceneRoot>>,

		children_query: Query<&Children>,
		has_collider: Query<(), With<Collider>>,
		still_not_collider_query: Query<(), (Or<(With<ConvexCollision>, With<TrimeshCollision>)>, Without<Collider>)>,
	) {
		let mut scene_roots = HashSet::new();

		for entity in tests.added_colliders_to_entities.iter().copied() {
			// Go up the hierarchy and collect the scene root if it exists.
			if let Some(entity) = parent_query.iter_ancestors(entity).find(|entity| has_scene_root.contains(*entity)) {
				scene_roots.insert(entity);
			}
		}

		tests.added_colliders_to_entities.clear();

		'scene_root_loop: for scene_root_entity in scene_roots {
			let mut collider_entities = Vec::new();

			for entity in children_query.iter_descendants(scene_root_entity) {
				if still_not_collider_query.contains(entity) {
					continue 'scene_root_loop;
				} else if has_collider.contains(entity) {
					collider_entities.push(entity);
				}
			}

			commands.trigger_targets(SceneCollidersReady { collider_entities }, scene_root_entity);
		}
	}
}

/// Used to mark which entities need to be tested to produce [`SceneCollidersReady`]. You probably shouldn't interact with this unless you know what you're doing.
#[derive(Resource, Default)]
pub struct SceneCollidersReadyTests {
	pub added_colliders_to_entities: HashSet<Entity>,
}

/// Triggered when all the colliders of a scene are done constructing.
#[derive(Event, Debug, Clone)]
pub struct SceneCollidersReady {
	pub collider_entities: Vec<Entity>,
}
