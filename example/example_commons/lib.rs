#[cfg(feature = "client")]
use bevy::camera_controller::free_camera;
use bevy::prelude::*;

pub struct ExampleCommonsPlugin;
impl Plugin for ExampleCommonsPlugin {
	fn build(&self, #[allow(unused)] app: &mut App) {
		#[cfg(feature = "client")]
		#[rustfmt::skip]
		app
			.add_plugins((
				bevy_inspector_egui::bevy_egui::EguiPlugin::default(),
				bevy_inspector_egui::quick::WorldInspectorPlugin::default(),
				free_camera::FreeCameraPlugin,
			))
			.add_systems(Update, (
				Self::draw_debug_transforms,
			))
		;
	}
}
impl ExampleCommonsPlugin {
	#[cfg(feature = "client")]
	pub fn draw_debug_transforms(
		mut gizmos: Gizmos,
		keyboard: Res<ButtonInput<KeyCode>>,
		query: Query<&GlobalTransform, Without<Camera>>,
		mut enabled: Local<bool>,
	) {
		if keyboard.just_pressed(KeyCode::KeyG) {
			*enabled = !*enabled;
		}

		if !*enabled {
			return;
		}

		for global_transform in &query {
			let translation = global_transform.translation();
			gizmos.line(translation, global_transform.transform_point(Vec3::X), Color::srgb(1., 0., 0.));
			gizmos.line(translation, global_transform.transform_point(Vec3::Y), Color::srgb(0., 1., 0.));
			gizmos.line(translation, global_transform.transform_point(Vec3::Z), Color::srgb(0., 0., 1.));
		}
	}
}

#[cfg(feature = "client")]
pub fn default_debug_camera_transform() -> Transform {
	Transform::from_xyz(-2.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y)
}

#[cfg(feature = "client")]
#[derive(Component)]
#[require(Transform, Camera3d, free_camera::FreeCamera)]
pub struct DebugCamera;
