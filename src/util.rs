use crate::*;
use bevy::ecs::component::Mutable;
use bevy::{
	ecs::world::DeferredWorld,
	image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
};

pub struct UtilPlugin;
impl Plugin for UtilPlugin {
	fn build(&self, #[allow(unused)] app: &mut App) {
		#[cfg(not(feature = "client"))]
		app.register_type::<Mesh3d>().register_type::<Aabb>();
	}
}

/// Container for meshes used for headless environments. This can't be the regular `Mesh3d` as it is provided by `bevy_render`
#[cfg(not(feature = "client"))]
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
#[require(Transform)]
pub struct Mesh3d(pub Handle<Mesh>);

/// Bevy's `Aabb` type is provided by `bevy_render`, but we need it in a headless context, so this is a few copied parts of it.
#[cfg(not(feature = "client"))]
#[derive(Component, Clone, Copy, Debug, Default, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq)]
pub struct Aabb {
	pub center: Vec3A,
	pub half_extents: Vec3A,
}

#[cfg(not(feature = "client"))]
impl Aabb {
	#[inline]
	pub fn from_min_max(minimum: Vec3, maximum: Vec3) -> Self {
		let minimum = Vec3A::from(minimum);
		let maximum = Vec3A::from(maximum);
		let center = 0.5 * (maximum + minimum);
		let half_extents = 0.5 * (maximum - minimum);
		Self { center, half_extents }
	}

	#[inline]
	pub fn min(&self) -> Vec3A {
		self.center - self.half_extents
	}

	#[inline]
	pub fn max(&self) -> Vec3A {
		self.center + self.half_extents
	}
}

pub trait ImageSamplerRepeatExt {
	/// Sets the address mode of this sampler to repeat.
	fn repeat(self) -> Self;
}
impl ImageSamplerRepeatExt for ImageSamplerDescriptor {
	fn repeat(self) -> Self {
		Self {
			address_mode_u: ImageAddressMode::Repeat,
			address_mode_v: ImageAddressMode::Repeat,
			address_mode_w: ImageAddressMode::Repeat,
			..self
		}
	}
}
impl ImageSamplerRepeatExt for ImageSampler {
	fn repeat(mut self) -> Self {
		let descriptor = self.get_or_init_descriptor();
		descriptor.address_mode_u = ImageAddressMode::Repeat;
		descriptor.address_mode_v = ImageAddressMode::Repeat;
		descriptor.address_mode_w = ImageAddressMode::Repeat;
		self
	}
}

pub trait BevyTrenchbroomCoordinateConversions {
	/// Converts from a z-up, y-forward coordinate space to a y-up, negative-z-forward coordinate space.
	fn z_up_to_y_up(self) -> Self;

	/// Converts from a y-up, negative-z-forward coordinate space to a z-up, y-forward coordinate space.
	fn y_up_to_z_up(self) -> Self;
}

impl BevyTrenchbroomCoordinateConversions for DVec3 {
	#[inline]
	fn z_up_to_y_up(self) -> Self {
		dvec3(self.x, self.z, -self.y)
	}

	#[inline]
	fn y_up_to_z_up(self) -> Self {
		dvec3(self.x, -self.z, self.y)
	}
}
impl BevyTrenchbroomCoordinateConversions for Vec3 {
	#[inline]
	fn z_up_to_y_up(self) -> Self {
		vec3(self.x, self.z, -self.y)
	}

	#[inline]
	fn y_up_to_z_up(self) -> Self {
		vec3(self.x, -self.z, self.y)
	}
}

pub(crate) trait AlmostEqual<T> {
	type Margin;
	fn almost_eq(self, other: T, margin: Self::Margin) -> bool;
}

impl AlmostEqual<f32> for f32 {
	type Margin = f32;
	fn almost_eq(self, other: f32, margin: Self::Margin) -> bool {
		(other - self).abs() < margin
	}
}
impl AlmostEqual<Vec3> for Vec3 {
	type Margin = f32;
	fn almost_eq(self, other: Vec3, margin: Self::Margin) -> bool {
		self.x.almost_eq(other.x, margin) && self.y.almost_eq(other.y, margin) && self.z.almost_eq(other.z, margin)
	}
}

impl AlmostEqual<f64> for f64 {
	type Margin = f64;
	fn almost_eq(self, other: f64, margin: Self::Margin) -> bool {
		(other - self).abs() < margin
	}
}
impl AlmostEqual<DVec3> for DVec3 {
	type Margin = f64;
	fn almost_eq(self, other: DVec3, margin: Self::Margin) -> bool {
		self.x.almost_eq(other.x, margin) && self.y.almost_eq(other.y, margin) && self.z.almost_eq(other.z, margin)
	}
}

impl AlmostEqual<Quat> for Quat {
	type Margin = f32;
	fn almost_eq(self, other: Quat, margin: Self::Margin) -> bool {
		self.x.almost_eq(other.x, margin)
			&& self.y.almost_eq(other.y, margin)
			&& self.z.almost_eq(other.z, margin)
			&& self.w.almost_eq(other.w, margin)
	}
}

#[allow(unused)]
macro_rules! assert_almost_eq {
	($left:expr, $right:expr, $margin:expr) => {
		match ($left, $right, $margin) {
			(left, right, margin) => {
				if !left.almost_eq(right, margin) {
					panic!("assertion `left.almost_eq(right)` failed\n  left: {left}\n right: {right}");
				}
			}
		}
	};
	($left:expr, $right:expr, $margin:expr, $($arg:tt)+) => {
		match ($left, $right, $margin) {
			(left, right, margin) => {
				if !left.almost_eq(right, margin) {
					panic!($($arg)+);
				}
			}
		}
	};
}

pub trait ConvertZeroToOne {
	/// If this equals to zero, return it where it is one, created for use with division.
	fn convert_zero_to_one(self) -> Self;
}

impl ConvertZeroToOne for f32 {
	fn convert_zero_to_one(self) -> Self {
		if self == 0. { 1. } else { self }
	}
}

impl ConvertZeroToOne for Vec2 {
	fn convert_zero_to_one(self) -> Self {
		vec2(self.x.convert_zero_to_one(), self.y.convert_zero_to_one())
	}
}

pub trait IsSceneWorld {
	/// Shorthand for checking if there isn't an `AppTypeRegistry` resource (chosen somewhat arbitrarily).
	///
	/// This is for component hooks, where if they are in a scene, they shouldn't fire.
	fn is_scene_world(&self) -> bool;
}
impl IsSceneWorld for DeferredWorld<'_> {
	fn is_scene_world(&self) -> bool {
		!self.contains_resource::<AppTypeRegistry>()
	}
}

/// Band-aid fix for a [TrenchBroom bug](https://github.com/TrenchBroom/TrenchBroom/issues/4447) where GLTF models are rotated be 90 degrees on the Y axis.
///
/// Apply this on an entity to counteract the rotation.
///
/// If you want to use this on a command, there is a helpful extension method you can use like so
/// ```
/// # use bevy::prelude::*;
/// # use bevy_trenchbroom::prelude::*;
/// fn example(mut commands: Commands, entity: Entity) {
///     commands.entity(entity)
///         .trenchbroom_gltf_rotation_fix()
///         // ...
/// # ;
/// }
/// ```
#[allow(private_bounds)]
pub fn trenchbroom_gltf_rotation_fix(entity: &mut impl EntityMutOrEntityWorldMut) {
	if let Some(mut transform) = entity.get_mut::<Transform>() {
		transform.rotate_local_y(-FRAC_PI_2);
	}
}

/// I looked for an easier solution, like an [`Into`] implementation to turn an [`EntityWorldMut`] into an [`EntityMut`], but found nothing.
trait EntityMutOrEntityWorldMut {
	fn get_mut<T: Component<Mutability = Mutable>>(&mut self) -> Option<Mut<'_, T>>;
}
impl EntityMutOrEntityWorldMut for EntityMut<'_> {
	fn get_mut<T: Component<Mutability = Mutable>>(&mut self) -> Option<Mut<'_, T>> {
		EntityMut::get_mut(self)
	}
}
impl EntityMutOrEntityWorldMut for EntityWorldMut<'_> {
	fn get_mut<T: Component<Mutability = Mutable>>(&mut self) -> Option<Mut<'_, T>> {
		EntityWorldMut::get_mut(self)
	}
}

pub trait TrenchBroomGltfRotationFixEntityCommandsExt {
	/// Bundles [`trenchbroom_gltf_rotation_fix`] into a command.
	fn trenchbroom_gltf_rotation_fix(&mut self) -> &mut Self;
}
impl TrenchBroomGltfRotationFixEntityCommandsExt for EntityCommands<'_> {
	fn trenchbroom_gltf_rotation_fix(&mut self) -> &mut Self {
		self.queue(|mut entity: EntityWorldMut| trenchbroom_gltf_rotation_fix(&mut entity))
	}
}

/// Rotate from Quake's +X to Bevy's -Z.
#[inline]
pub fn quake_fwd_to_bevy_fwd() -> Quat {
	Quat::from_rotation_y(FRAC_PI_2)
}

/// `angles` is pitch, yaw, roll. Converts from degrees to radians. `0 0 0` [points east](https://www.gamers.org/dEngine/quake/QDP/qmapspec.html#2.1.1).
#[inline]
pub fn angles_to_quat(angles: Vec3) -> Quat {
	let pitch = angles.x.to_radians();
	let yaw = angles.y.to_radians();
	let roll = angles.z.to_radians();
	quake_fwd_to_bevy_fwd() * Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll)
}

/// `mangle` is yaw, negative pitch, roll. Converts from degrees to radians. `0 0 0` [points east](https://www.gamers.org/dEngine/quake/QDP/qmapspec.html#2.1.1).
///
/// NOTE: TrenchBroom docs dictate that this function should only be called when the entity classname begins with "light", otherwise "mangle" is a synonym for “angles”.
#[inline]
pub fn mangle_to_quat(mangle: Vec3) -> Quat {
	let yaw = mangle.x.to_radians();
	let pitch = -mangle.y.to_radians();
	let roll = mangle.z.to_radians();
	quake_fwd_to_bevy_fwd() * Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll)
}

/// `angle` is the rotation around the Y axis. Converts from degrees to radians. `0` [points east](https://www.gamers.org/dEngine/quake/QDP/qmapspec.html#2.1.1).
/// # Special Values
/// - -1: Up
/// - -2: Down
#[inline]
pub fn angle_to_quat(angle: f32) -> Quat {
	quake_fwd_to_bevy_fwd()
		* match angle {
			-1. => Quat::from_rotation_z(FRAC_PI_2),
			-2. => Quat::from_rotation_z(-FRAC_PI_2),
			angle => Quat::from_rotation_y(angle.to_radians()),
		}
}

pub const QUAKE_LIGHT_TO_LUX_DIVISOR: f32 = 50_000.;
/// Quake light (such as the `light` property used in light entities) conversion to lux (lumens per square meter).
///
/// NOTE: This is only a rough estimation, based on what i've personally found looks right.
#[inline]
pub fn quake_light_to_lux(light: f32) -> f32 {
	light / QUAKE_LIGHT_TO_LUX_DIVISOR
}

#[test]
fn coordinate_conversions() {
	assert_eq!(Vec3::X.z_up_to_y_up(), Vec3::X);
	assert_eq!(Vec3::Y.z_up_to_y_up(), Vec3::NEG_Z);
	assert_eq!(Vec3::Z.z_up_to_y_up(), Vec3::Y);

	assert_eq!(Vec3::X.z_up_to_y_up().y_up_to_z_up(), Vec3::X);
	assert_eq!(Vec3::Y.z_up_to_y_up().y_up_to_z_up(), Vec3::Y);
	assert_eq!(Vec3::Z.z_up_to_y_up().y_up_to_z_up(), Vec3::Z);
}

#[test]
fn rotation_property_to_quat() {
	const MARGIN: f32 = 0.0001;

	// angle
	assert_almost_eq!(angle_to_quat(0.) * Vec3::X, Vec3::NEG_Z, MARGIN);
	assert_almost_eq!(angle_to_quat(90.) * Vec3::X, Vec3::NEG_X, MARGIN);
	assert_almost_eq!(angle_to_quat(0.) * Vec3::Y, Vec3::Y, MARGIN);
	assert_almost_eq!(angle_to_quat(-1.) * Vec3::X, Vec3::Y, MARGIN);
	assert_almost_eq!(angle_to_quat(-2.) * Vec3::X, Vec3::NEG_Y, MARGIN);
	assert_almost_eq!(angle_to_quat(-2.) * Vec3::Y, Vec3::NEG_Z, MARGIN);

	// mangle
	assert_almost_eq!(mangle_to_quat(vec3(0., 0., 0.)) * Vec3::X, Vec3::NEG_Z, MARGIN);
	assert_almost_eq!(mangle_to_quat(vec3(0., 0., 0.)) * Vec3::Y, Vec3::Y, MARGIN);

	assert_almost_eq!(mangle_to_quat(vec3(90., 0., 0.)) * Vec3::X, Vec3::NEG_X, MARGIN);
	assert_almost_eq!(mangle_to_quat(vec3(0., -90., 0.)) * Vec3::X, Vec3::NEG_Z, MARGIN);
	assert_almost_eq!(mangle_to_quat(vec3(0., 90., 0.)) * Vec3::X, Vec3::NEG_Z, MARGIN);
	assert_almost_eq!(mangle_to_quat(vec3(0., 0., 90.)) * Vec3::Y, Vec3::Z, MARGIN);
	assert_almost_eq!(mangle_to_quat(vec3(45., 45., 0.)) * Vec3::X, vec3(-1., 0., -1.).normalize(), MARGIN);

	// angles
	assert_almost_eq!(angles_to_quat(vec3(0., 0., 0.)) * Vec3::X, Vec3::NEG_Z, MARGIN);
	assert_almost_eq!(angles_to_quat(vec3(0., 0., 0.)) * Vec3::Y, Vec3::Y, MARGIN);

	assert_almost_eq!(angles_to_quat(vec3(0., 90., 0.)) * Vec3::X, Vec3::NEG_X, MARGIN);
	assert_almost_eq!(angles_to_quat(vec3(90., 0., 0.)) * Vec3::X, Vec3::NEG_Z, MARGIN);
	assert_almost_eq!(angles_to_quat(vec3(0., 0., 90.)) * Vec3::Y, Vec3::Z, MARGIN);
	assert_almost_eq!(angles_to_quat(vec3(-45., -45., 0.)) * Vec3::X, vec3(1.0, 0.0, -1.0).normalize(), MARGIN);
}
