use crate::*;
use geometry::{Brushes, BrushList};
use q1bsp::data::bsp::BspLeafContents;
use util::BevyTrenchbroomCoordinateConversions;

#[cfg(feature = "rapier")]
use bevy_rapier3d::prelude::*;

#[cfg(feature = "avian")]
use avian3d::prelude::*;

#[cfg(feature = "avian")]
use avian3d::parry;
#[cfg(feature = "rapier")]
use bevy_rapier3d::rapier::parry;


/// Automatically creates convex colliders for entities with [Brushes].
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct ConvexCollision;

/// Automatically creates trimesh colliders for entities with [Mesh3d].
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct TrimeshCollision;

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct BspHullCollision {
    pub bsp: Handle<BspDataAsset>,
    pub model_idx: usize,
}

pub(crate) struct PhysicsPlugin;
impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .register_type::<ConvexCollision>()
            .register_type::<TrimeshCollision>()
            .register_type::<BspHullCollision>()

            .add_systems(Update, (
                Self::create_convex_colliders,
                Self::create_trimesh_colliders,
                Self::create_bsp_hull_colliders,
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
                let vertices: Vec<Vec3> = brush.calculate_vertices()
                    .into_iter()
                    .map(|(pos, _)| pos.as_vec3())
                    .collect();
                
                let Some(collider) = Collider::convex_hull(&vertices) else {
                    error!("Entity {entity}'s brush (index {brush_idx}) is invalid (non-convex), and a collider could not be computed for it!");
                    continue;
                };
                colliders.push((
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    collider,
                ));
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
                let vertices: Vec<Vec3> = brush.calculate_vertices()
                    .into_iter()
                    .map(|(pos, _)| pos.as_vec3())
                    .collect();
                
                let Some(collider) = Collider::convex_hull(vertices) else {
                    error!("Entity {entity}'s brush (index {brush_idx}) is invalid (non-convex), and a collider could not be computed for it!");
                    continue;
                };
                colliders.push((
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    collider,
                ));
            }

            commands.entity(entity)
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

            commands.entity(entity)
                .insert(collider)
                .insert_if_new(RigidBody::Static);
        }
    }

    pub fn create_bsp_hull_colliders(
        mut commands: Commands,
        query: Query<(Entity, &BspHullCollision), Without<Collider>>,
        bsp_data_assets: Res<Assets<BspDataAsset>>,
        tb_server: Res<TrenchBroomServer>,
    ) {
        for (entity, collision) in &query {
            let Some(BspDataAsset(bsp_data)) = bsp_data_assets.get(&collision.bsp) else { continue };
            
            let shape = BspHullShape {
                tb_server: tb_server.clone(),
                bsp_data: bsp_data.clone(),
                model_idx: collision.model_idx,
            };
            
            commands.entity(entity)
                .insert(Collider::from(parry::shape::SharedShape::new(shape)))
                .insert_if_new(RigidBody::Static);
        }
    }
}

#[derive(Debug, Clone)]
pub struct BspHullShape {
    pub tb_server: TrenchBroomServer,
    pub bsp_data: Arc<BspData>,
    pub model_idx: usize,
}
impl parry::query::PointQuery for BspHullShape {
    fn project_local_point(&self, pt: &parry::math::Point<f32>, solid: bool) -> parry::query::PointProjection {
        todo!()
    }

    fn project_local_point_and_get_feature(&self, pt: &parry::math::Point<f32>) -> (parry::query::PointProjection, parry::shape::FeatureId) {
        (self.project_local_point(pt, true), parry::shape::FeatureId::Unknown)
    }
}
impl parry::query::RayCast for BspHullShape {
    fn cast_local_ray_and_get_normal(
        &self,
        ray: &parry::query::Ray,
        max_time_of_impact: f32,
        solid: bool,
    ) -> Option<parry::query::RayIntersection> {
        let origin: Vec3 = ray.origin.clone().into();
        let dir: Vec3 = ray.dir.into();
        
        let from = self.tb_server.config.from_bevy_space(origin);
        let to = self.tb_server.config.from_bevy_space(origin + dir * max_time_of_impact);

        // TODO this probably isn't the correct functionality
        //      see https://rapier.rs/docs/user_guides/rust/scene_queries
        if solid && self.bsp_data.leaves[self.bsp_data.leaf_at_point(self.model_idx, from)].contents == BspLeafContents::Solid {
            return Some(parry::query::RayIntersection {
                time_of_impact: 0.,
                normal: (-dir).into(),
                feature: parry::shape::FeatureId::Unknown,
            });
        }

        self.bsp_data.raycast(self.model_idx, from, to).impact
            .map(|impact| {
                parry::query::RayIntersection {
                    time_of_impact: impact.fraction,
                    normal: impact.normal.z_up_to_y_up().into(),
                    // TODO
                    feature: parry::shape::FeatureId::Unknown,
                }
            })
    }
}
impl parry::shape::Shape for BspHullShape {
    // TODO technically, BSP trees represent all of space. do we want these? maybe only if it's enclosing? they aren't optional...
    fn compute_local_aabb(&self) -> parry::bounding_volume::Aabb {
        let bounding_box = self.bsp_data.models[self.model_idx].bound;
        let min = self.tb_server.config.to_bevy_space(bounding_box.min);
        let max = self.tb_server.config.to_bevy_space(bounding_box.max);
        
        parry::bounding_volume::Aabb::new(min.into(), max.into())
    }

    fn compute_local_bounding_sphere(&self) -> parry::bounding_volume::BoundingSphere {
        // Performant, but not very accurate. Probably fine for now.
        let model = &self.bsp_data.models[self.model_idx];
        let bounding_box = model.bound;
        let min = self.tb_server.config.to_bevy_space(bounding_box.min);
        let max = self.tb_server.config.to_bevy_space(bounding_box.max);

        let center = (min + max) / 2.;
        // Distance to min and max will be the same because center is between them.
        let radius = center.distance(min);
        
        parry::bounding_volume::BoundingSphere::new(center.into(), radius)
    }

    fn clone_dyn(&self) -> Box<dyn parry::shape::Shape> {
        Box::new(self.clone())
    }

    fn scale_dyn(&self, _scale: &parry::math::Vector<f32>, _num_subdivisions: u32) -> Option<Box<dyn parry::shape::Shape>> {
        None // TODO ?
    }

    fn mass_properties(&self, density: f32) -> parry::mass_properties::MassProperties {
        // TODO this probably isn't very accurate but oh well!
        parry::mass_properties::MassProperties::from_cuboid(density, self.compute_local_aabb().half_extents())
    }

    fn shape_type(&self) -> parry::shape::ShapeType {
        parry::shape::ShapeType::Custom
    }

    fn as_typed_shape(&self) -> parry::shape::TypedShape {
        parry::shape::TypedShape::Custom(self)
    }

    // Not really sure these two functions do, just copying from other implementations and hoping things work out!
    fn ccd_thickness(&self) -> f32 {
        self.compute_local_aabb().half_extents().min()
    }

    fn ccd_angular_thickness(&self) -> f32 {
        std::f32::consts::FRAC_PI_2
    }
}

// TODO test collider creation