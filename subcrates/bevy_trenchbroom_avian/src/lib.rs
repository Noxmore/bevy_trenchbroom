use avian3d::{
	math::AdjustPrecision,
	parry::shape::SharedShape,
	prelude::{Collider, RigidBody},
};
use bevy::math::{DVec3, Vec3};
use bevy_trenchbroom::physics::PhysicsBackend;

pub struct AvianPhysicsBackend;
impl PhysicsBackend for AvianPhysicsBackend {
	type Vector = avian3d::math::Vector;
	const ZERO: Self::Vector = Self::Vector::ZERO;
	fn vec3(v: Vec3) -> Self::Vector {
		v.adjust_precision()
	}
	fn dvec3(v: DVec3) -> Self::Vector {
		v.adjust_precision()
	}

	type Collider = Collider;
	fn cuboid_collider(half_extents: Self::Vector) -> Self::Collider {
		SharedShape::cuboid(half_extents.x, half_extents.y, half_extents.z).into()
	}
	fn convex_collider(points: Vec<Self::Vector>) -> Option<Self::Collider> {
		Collider::convex_hull(points)
	}
	fn trimesh_collider(mesh: &bevy::mesh::Mesh) -> Option<Self::Collider> {
		Collider::trimesh_from_mesh(mesh)
	}
	fn compound_collider(colliders: Vec<(Self::Vector, bevy::math::Quat, Self::Collider)>) -> Self::Collider {
		Collider::compound(colliders)
	}

	fn insert_static_collider(mut entity: bevy::ecs::system::EntityCommands, collider: Self::Collider) {
		entity.insert(collider).insert_if_new(RigidBody::Static);
	}
}
