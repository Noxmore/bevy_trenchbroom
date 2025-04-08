use avian3d::prelude::*;
use bevy::ecs::{component::ComponentId, world::DeferredWorld};
use bevy::math::*;
use bevy::prelude::*;
use bevy_flycam::prelude::*;
use bevy_trenchbroom::prelude::*;
use nil::prelude::*;

// TODO: We aren't using inventory to register here because it's broken on wasm.

#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
#[class(
	geometry(GeometryProvider::new().smooth_by_default_angle().convex_collider()),
	no_register,
)]
pub struct Worldspawn;

#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
#[require(Transform)]
#[class(
	base(Transform),
	geometry(GeometryProvider::new().smooth_by_default_angle().convex_collider()),
	no_register,
)]
pub struct FuncDoor;

#[derive(PointClass, Component, Reflect)]
#[reflect(Component)]
#[require(Transform)]
#[class(base(Transform), no_register)]
#[component(on_add = Self::on_add)]
pub struct Cube;
impl Cube {
	fn on_add(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
		let Some(asset_server) = world.get_resource::<AssetServer>() else { return };
		let cube = asset_server.add(Mesh::from(Cuboid::new(0.42, 0.42, 0.42)));
		let material = asset_server.add(StandardMaterial::default());

		world.commands().entity(entity).insert((Mesh3d(cube), MeshMaterial3d(material)));
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
		.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::default())
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
		commands.spawn((
			Transform::from_translation(Vec3::new(0., 10., 0.)),
			RigidBody::Dynamic,
			Collider::cuboid(0.5, 0.5, 0.5),
		));
	}
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>, mut projection_query: Query<&mut Projection>) {
	commands.spawn(SceneRoot(asset_server.load("maps/example.map#Scene")));

	// Wide FOV

	for mut projection in &mut projection_query {
		*projection = Projection::Perspective(PerspectiveProjection {
			fov: 90_f32.to_radians(),
			..default()
		});
	}
}
