use bevy::prelude::*;
#[cfg(feature = "client")]
use bevy::{
	input::mouse::AccumulatedMouseMotion,
	window::{CursorGrabMode, PrimaryWindow},
};

// These are hardcoded because this is only for examples.

#[cfg(feature = "client")]
pub const SENSITIVITY: f32 = 0.0007;
#[cfg(feature = "client")]
pub const BASE_MOVEMENT_SPEED: f32 = 4.;

pub struct ExampleCommonsPlugin;
impl Plugin for ExampleCommonsPlugin {
	fn build(&self, #[allow(unused)] app: &mut App) {
		#[cfg(feature = "client")]
		#[rustfmt::skip]
		app
			.add_plugins((
				bevy_inspector_egui::bevy_egui::EguiPlugin {
					enable_multipass_for_primary_context: true,
				},
				bevy_inspector_egui::quick::WorldInspectorPlugin::default(),
			))
			.add_systems(Update, (
				Self::toggle_focus,
				Self::move_debug_camera,
				Self::draw_debug_transforms,
			))
		;
	}
}
impl ExampleCommonsPlugin {
	#[cfg(feature = "client")]
	pub fn move_debug_camera(
		mouse_motion: Res<AccumulatedMouseMotion>,
		keyboard: Res<ButtonInput<KeyCode>>,
		window: Single<&Window, With<PrimaryWindow>>,
		mut camera_query: Query<&mut Transform, With<DebugCamera>>,
		time: Res<Time>,
	) {
		if window.cursor_options.grab_mode == CursorGrabMode::None {
			return;
		}

		for mut transform in &mut camera_query {
			// Mouse movement
			let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);

			pitch -= mouse_motion.delta.y * SENSITIVITY;
			yaw -= mouse_motion.delta.x * SENSITIVITY;

			pitch = pitch.clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);

			transform.rotation = Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);

			// Keyboard movement
			let mut movement = Vec3::ZERO;
			if keyboard.pressed(KeyCode::KeyW) {
				movement += *transform.forward()
			}
			if keyboard.pressed(KeyCode::KeyS) {
				movement += *transform.back()
			}
			if keyboard.pressed(KeyCode::KeyA) {
				movement += *transform.left()
			}
			if keyboard.pressed(KeyCode::KeyD) {
				movement += *transform.right()
			}
			movement = movement.normalize_or_zero();
			if keyboard.pressed(KeyCode::Space) {
				movement.y += 1.
			}
			if keyboard.pressed(KeyCode::ControlLeft) {
				movement.y -= 1.
			}

			movement *= BASE_MOVEMENT_SPEED;

			if keyboard.pressed(KeyCode::AltLeft) {
				movement *= 10.;
			} else if keyboard.pressed(KeyCode::ShiftLeft) {
				movement *= 3.;
			}

			transform.translation += movement * time.delta_secs();
		}
	}

	#[cfg(feature = "client")]
	pub fn toggle_focus(mut window: Single<&mut Window, With<PrimaryWindow>>, keyboard: Res<ButtonInput<KeyCode>>) {
		if !keyboard.just_pressed(KeyCode::Escape) {
			return;
		}

		match window.cursor_options.grab_mode {
			CursorGrabMode::None => {
				window.cursor_options.grab_mode = CursorGrabMode::Locked;
				window.cursor_options.visible = false;
			}
			_ => {
				window.cursor_options.grab_mode = CursorGrabMode::None;
				window.cursor_options.visible = true;
			}
		}
	}

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
#[require(Transform, Camera3d)]
pub struct DebugCamera;
