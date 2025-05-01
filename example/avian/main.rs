use avian3d::prelude::*;
use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::math::*;
use bevy::prelude::*;
use bevy_flycam::prelude::*;
use bevy_trenchbroom::prelude::*;
use nil::prelude::*;

// TODO: We aren't using inventory to register here because it's broken on wasm. The `auto_register` feature is turned off.

#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
#[geometry(GeometryProvider::new().smooth_by_default_angle().convex_collider())]
pub struct Worldspawn;

#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
#[geometry(GeometryProvider::new().smooth_by_default_angle().convex_collider())]
pub struct FuncDoor;

#[derive(PointClass, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = Self::on_add)]
pub struct Cube;
impl Cube {
	fn on_add(mut world: DeferredWorld, ctx: HookContext) {
		let Some(asset_server) = world.get_resource::<AssetServer>() else { return };
		let cube = asset_server.add(Mesh::from(Cuboid::new(0.42, 0.42, 0.42)));
		let material = asset_server.add(StandardMaterial::default());

		world.commands().entity(ctx.entity).insert((Mesh3d(cube), MeshMaterial3d(material)));
	}
}

fn main() {
	App::new()
		.add_plugins((DefaultPlugins.set(AssetPlugin {
			file_path: "../../assets".s(),
			..default()
		}),))
		.add_plugins(avian3d::prelude::PhysicsPlugins::default())
		.add_plugins(avian3d::prelude::PhysicsDebugPlugin::default())
		.add_plugins(PlayerPlugin)
		.insert_resource(MovementSettings {
			sensitivity: 0.00005,
			speed: 6.,
		})
		.add_plugins(bevy_inspector_egui::bevy_egui::EguiPlugin {
			enable_multipass_for_primary_context: true,
		})
		.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::default())
		.add_systems(Update, make_unlit)
		.add_plugins(TrenchBroomPlugin(
			TrenchBroomConfig::new("bevy_trenchbroom_example")
				.no_bsp_lighting(true)
				.register_class::<Worldspawn>()
				.register_class::<Cube>()
				.register_class::<FuncDoor>(),
		))
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

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>, mut projection_query: Query<&mut Projection>) {
	commands.spawn((SceneRoot(asset_server.load("maps/example.map#Scene")), Transform::from_xyz(-5., 0., 0.)));
	commands.spawn((SceneRoot(asset_server.load("maps/example.bsp#Scene")), Transform::from_xyz(5., 0., 0.)));

	// Wide FOV

	for mut projection in &mut projection_query {
		*projection = Projection::Perspective(PerspectiveProjection {
			fov: 90_f32.to_radians(),
			..default()
		});
	}
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
