use crate::*;
use brush::ConvexHull;
#[cfg(feature = "bsp")]
use bsp::BrushHullsAsset;
use geometry::{Brushes, BrushesAsset};

/// Generic physics engine interface.
pub trait PhysicsBackend: Send + Sync + 'static {
	type Vector: std::ops::SubAssign;
	const ZERO: Self::Vector;
	fn vec3(v: Vec3) -> Self::Vector;
	fn dvec3(v: DVec3) -> Self::Vector;

	type Collider: Component;
	fn cuboid_collider(half_extents: Self::Vector) -> Self::Collider;
	fn convex_collider(points: Vec<Self::Vector>) -> Option<Self::Collider>;
	fn trimesh_collider(mesh: &Mesh) -> Option<Self::Collider>;
	fn compound_collider(colliders: Vec<(Self::Vector, Quat, Self::Collider)>) -> Self::Collider;

	fn insert_static_collider(entity: EntityCommands, collider: Self::Collider);
}

/// Automatically creates convex colliders for entities with [`Brushes`].
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct ConvexCollision;

/// Automatically creates trimesh colliders for entities with [`Mesh3d`].
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct TrimeshCollision;

enum ConvexPhysicsGeometry<B: PhysicsBackend> {
	ConvexHull(Vec<B::Vector>),
	Cuboid { center: B::Vector, half_extents: B::Vector },
}

/// Attempts to calculate vertices on the brushes contained within for use in physics, if it can find said brushes.
///
/// If it can't find them (like if the asset isn't loaded), returns [`None`].
fn calculate_convex_physics_geometry<'l, 'w: 'l, B: PhysicsBackend>(
	brushes: &Brushes,
	brush_lists: &'w Assets<BrushesAsset>,
	#[cfg(feature = "bsp")] bsp_brushes: &'w Assets<BrushHullsAsset>,
) -> Option<Vec<ConvexPhysicsGeometry<B>>> {
	fn extract_vertices<B: PhysicsBackend, T: ConvexHull>(brush: &T) -> ConvexPhysicsGeometry<B> {
		match brush.as_cuboid() {
			Some((from, to)) => ConvexPhysicsGeometry::Cuboid {
				center: B::dvec3(0.5 * (from + to)),
				half_extents: B::dvec3(0.5 * (to - from)),
			},
			None => ConvexPhysicsGeometry::ConvexHull(brush.calculate_vertices().map(|(position, _)| B::dvec3(position)).collect()),
		}
	}

	match brushes {
		Brushes::Owned(list) => Some(list.iter().map(extract_vertices).collect()),
		Brushes::Shared(handle) => brush_lists.get(handle).map(|list| list.iter().map(extract_vertices).collect()),
		#[cfg(feature = "bsp")]
		Brushes::Bsp(handle) => bsp_brushes
			.get(handle)
			.map(|brushes_asset| brushes_asset.0.iter().map(extract_vertices).collect()),
	}
}

// Has the `TrenchBroom` prefix because it is meant to be commonly used in user code.
pub struct TrenchBroomPhysicsPlugin<B: PhysicsBackend> {
	pub backend: B,
}
impl<B: PhysicsBackend> Plugin for TrenchBroomPhysicsPlugin<B> {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.init_resource::<SceneCollidersReadyTests>()

			// PostUpdate to order right after scenes have been spawned
			.add_systems(PostUpdate, (
				Self::add_convex_colliders,
				Self::add_trimesh_colliders,
				Self::trigger_scene_colliders_ready,
			).chain())
		;
	}
}
impl<B: PhysicsBackend> TrenchBroomPhysicsPlugin<B> {
	pub fn new(backend: B) -> Self {
		Self { backend }
	}

	pub fn add_convex_colliders(
		mut commands: Commands,
		query: Query<(Entity, Option<&Brushes>, &Transform), (With<ConvexCollision>, Without<B::Collider>)>,
		brush_lists: Res<Assets<BrushesAsset>>,
		#[cfg(feature = "bsp")] brush_assets: Res<Assets<BrushHullsAsset>>,
		mut tests: ResMut<SceneCollidersReadyTests>,
	) {
		#[allow(unused)]
		for (entity, brushes, transform) in &query {
			let Some(brushes) = brushes else {
				error!(
					"Entity {entity} has `ConvexCollision`, but no `Brushes`! If you're using Q1 BSPs, you may have forgotten to add the `-wrbrushesonly` flag to qbsp. Removing ConvexCollision component..."
				);
				commands.entity(entity).remove::<ConvexCollision>();
				continue;
			};
			let mut colliders = Vec::new();
			let Some(brush_geometries) = calculate_convex_physics_geometry::<B>(
				brushes,
				&brush_lists,
				#[cfg(feature = "bsp")]
				&brush_assets,
			) else {
				continue;
			};

			for (brush_idx, physics_geometry) in brush_geometries.into_iter().enumerate() {
				match physics_geometry {
					ConvexPhysicsGeometry::Cuboid { center, half_extents } => {
						colliders.push((center, transform.rotation.inverse(), B::cuboid_collider(half_extents)));
					}

					ConvexPhysicsGeometry::ConvexHull(mut vertices) => {
						if vertices.is_empty() {
							continue;
						}

						// Bring the vertices to the origin if they're generated in world-space (non-bsp)
						#[cfg(feature = "bsp")]
						if !matches!(brushes, Brushes::Bsp(_)) {
							for vertex in &mut vertices {
								*vertex -= B::vec3(transform.translation);
							}
						}

						let Some(collider) = B::convex_collider(vertices) else {
							error!(
								"Entity {entity}'s brush (index {brush_idx}) is invalid (non-convex), and a collider could not be computed for it!"
							);
							continue;
						};

						colliders.push((B::ZERO, transform.rotation.inverse(), collider));
					}
				}
			}

			if colliders.is_empty() {
				error!(
					"No colliders produced by brushes for entity {entity}, removing ConvexCollision component. If this is expected behavior, make an issue and i will remove this message."
				);
				commands.entity(entity).remove::<ConvexCollision>();
				continue;
			}

			B::insert_static_collider(commands.entity(entity), B::compound_collider(colliders));

			tests.added_colliders_to_entities.insert(entity);
		}
	}

	pub fn add_trimesh_colliders(
		mut commands: Commands,
		query: Query<(Entity, &Mesh3d), (With<TrimeshCollision>, Without<B::Collider>)>,
		meshes: Res<Assets<Mesh>>,
		mut tests: ResMut<SceneCollidersReadyTests>,
	) {
		for (entity, mesh3d) in &query {
			let Some(mesh) = meshes.get(mesh3d.id()) else {
				continue;
			};

			let Some(collider) = B::trimesh_collider(mesh) else {
				error!("Entity {entity} has TrimeshCollision, but index buffer or vertex buffer of the mesh are in an incompatible format.");
				continue;
			};
			B::insert_static_collider(commands.entity(entity), collider);

			tests.added_colliders_to_entities.insert(entity);
		}
	}

	pub fn trigger_scene_colliders_ready(
		mut commands: Commands,
		mut tests: ResMut<SceneCollidersReadyTests>,

		parent_query: Query<&ChildOf>,
		has_scene_root: Query<(), With<SceneRoot>>,

		children_query: Query<&Children>,
		has_collider: Query<(), With<B::Collider>>,
		still_not_collider_query: Query<(), (Or<(With<ConvexCollision>, With<TrimeshCollision>)>, Without<B::Collider>)>,
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

			commands.trigger(SceneCollidersReady {
				scene_root_entity,
				collider_entities,
			});
		}
	}
}

/// Used to mark which entities need to be tested to produce [`SceneCollidersReady`]. You probably shouldn't interact with this unless you know what you're doing.
#[derive(Resource, Default)]
pub struct SceneCollidersReadyTests {
	pub added_colliders_to_entities: HashSet<Entity>,
}

/// Triggered when all the colliders of a scene are done constructing.
#[derive(EntityEvent, Debug, Clone)]
pub struct SceneCollidersReady {
	#[event_target]
	pub scene_root_entity: Entity,
	pub collider_entities: Vec<Entity>,
}
