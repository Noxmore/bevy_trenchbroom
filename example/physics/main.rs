#[cfg(all(not(feature = "rapier"), not(feature = "avian")))]
compile_error!("Please enable either the `rapier` or `avian` feature flags to test a specific physics engine!");

#[cfg(feature = "avian")]
use avian3d::prelude::*;
use bevy::math::*;
use bevy::prelude::*;
#[cfg(feature = "rapier")]
use bevy_rapier3d::prelude::*;
use bevy_trenchbroom::config::WriteTrenchBroomConfigOnStartPlugin;
use bevy_trenchbroom::prelude::*;
use nil::prelude::*;

#[solid_class]
pub struct FuncDoor;

#[cfg(feature = "avian")]
fn physics_plugins(app: &mut App) {
	app.add_plugins((PhysicsPlugins::default(), PhysicsDebugPlugin::default()));
}

#[cfg(feature = "rapier")]
fn physics_plugins(app: &mut App) {
	app.add_plugins((RapierPhysicsPlugin::<()>::default(), RapierDebugRenderPlugin::default()));
}

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(AssetPlugin {
			file_path: "../../assets".s(),
			..default()
		}))
		.add_plugins(physics_plugins)
		.add_plugins(example_commons::ExampleCommonsPlugin)
		.add_systems(Update, make_unlit)
		.add_plugins(
			TrenchBroomPlugins(
				TrenchBroomConfig::new("bevy_trenchbroom_example")
					.default_solid_spawn_hooks(|| SpawnHooks::new().convex_collider())
					.no_bsp_lighting(true),
			)
			.build()
			// I use bsp_loading to write the config.
			.disable::<WriteTrenchBroomConfigOnStartPlugin>(),
		)
		.register_type::<FuncDoor>()
		.add_systems(PostStartup, setup_scene)
		.add_systems(FixedUpdate, spawn_cubes)
		.run();
}

fn spawn_cubes(mut commands: Commands, time: Res<Time>, mut local: Local<Option<Timer>>) {
	if local.is_none() {
		*local = Some(Timer::from_seconds(1.0, TimerMode::Repeating));
	}

	let Some(timer) = (*local).as_mut() else {
		return;
	};

	timer.tick(time.delta());

	if timer.just_finished() {
		commands.spawn((Transform::from_xyz(5., 10., 0.), RigidBody::Dynamic, Collider::cuboid(0.5, 0.5, 0.5)));

		commands.spawn((Transform::from_xyz(-5., 10., 0.), RigidBody::Dynamic, Collider::cuboid(0.5, 0.5, 0.5)));
	}
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands.spawn((SceneRoot(asset_server.load("maps/example.map#Scene")), Transform::from_xyz(-5., 0., 0.)));
	commands.spawn((SceneRoot(asset_server.load("maps/example.bsp#Scene")), Transform::from_xyz(5., 0., 0.)));

	commands.spawn((
		example_commons::DebugCamera,
		Projection::Perspective(PerspectiveProjection {
			fov: 90_f32.to_radians(),
			..default()
		}),
		example_commons::default_debug_camera_transform(),
	));
}

// We don't care about lighting, just physics.
fn make_unlit(mut materials: ResMut<Assets<StandardMaterial>>) {
	if !materials.is_changed() {
		return;
	}

	for (_, material) in materials.iter_mut() {
		material.unlit = true;
	}
}
