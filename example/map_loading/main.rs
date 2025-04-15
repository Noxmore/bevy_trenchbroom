#![allow(unexpected_cfgs)]

#[cfg(feature = "example_client")]
use bevy::ecs::{component::ComponentId, world::DeferredWorld};
use bevy::math::*;
use bevy::prelude::*;
#[cfg(feature = "example_client")]
use bevy_flycam::prelude::*;
use bevy_trenchbroom::prelude::*;
use nil::prelude::*;

// TODO: We aren't using inventory to register here because it's broken on wasm.

#[derive(SolidClass, Component, Reflect)]
#[no_register]
#[reflect(Component)]
#[geometry(GeometryProvider::new().smooth_by_default_angle())]
pub struct Worldspawn;

#[derive(SolidClass, Component, Reflect)]
#[no_register]
#[reflect(Component)]
#[require(Transform)]
#[geometry(GeometryProvider::new().smooth_by_default_angle())]
pub struct FuncDoor;

#[derive(PointClass, Component, Reflect)]
#[no_register]
#[reflect(Component)]
#[require(Transform)]
#[cfg_attr(feature = "example_client", component(on_add = Self::on_add))]
pub struct Cube;
#[cfg(feature = "example_client")]
impl Cube {
	fn on_add(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
		let Some(asset_server) = world.get_resource::<AssetServer>() else { return };
		let cube = asset_server.add(Mesh::from(Cuboid::new(0.42, 0.42, 0.42)));
		let material = asset_server.add(StandardMaterial::default());

		world.commands().entity(entity).insert((Mesh3d(cube), MeshMaterial3d(material)));
	}
}

#[derive(PointClass, Component, Reflect)]
#[no_register]
#[reflect(Component)]
#[require(Transform)]
#[model("models/mushroom.glb")]
#[size(-4 -4 0, 4 4 16)]
#[component(on_add = spawn_class_gltf::<Mushroom>)]
pub struct Mushroom;

// This is a custom light class for parity with bsp_loading, if you don't support bsps, you should use `PointLight` as base class instead.
#[derive(PointClass, Component, Reflect, Clone, Copy, SmartDefault)]
#[no_register]
#[reflect(Component)]
#[require(Transform)]
pub struct Light {
	#[default(Color::srgb(1., 1., 1.))]
	pub _color: Color,
	#[default(300.)]
	pub light: f32,
}

struct ClientPlugin;
impl Plugin for ClientPlugin {
	fn build(&self, #[allow(unused)] app: &mut App) {
		#[cfg(feature = "example_client")]
		#[rustfmt::skip]
		app
			// bevy_flycam setup so we can get a closer look at the scene, mainly for debugging
			.add_plugins(PlayerPlugin)
			.insert_resource(MovementSettings {
				sensitivity: 0.00005,
				speed: 6.,
			})
			.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::default())
			.add_systems(Update, spawn_lights)
		;
	}
}

fn main() {
	App::new()
		.add_plugins((
			DefaultPlugins.set(AssetPlugin {
				file_path: "../../assets".s(),
				..default()
			}),
			ClientPlugin,
		))
		.add_plugins(TrenchBroomPlugin(
			TrenchBroomConfig::new("bevy_trenchbroom_example")
				.no_bsp_lighting(true)
				.register_class::<Worldspawn>()
				.register_class::<Cube>()
				.register_class::<Mushroom>()
				.register_class::<Light>()
				.register_class::<FuncDoor>(),
		))
		.add_systems(PostStartup, setup_scene)
		.run();
}

#[rustfmt::skip]
fn setup_scene(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	#[cfg(feature = "example_client")]
	mut projection_query: Query<&mut Projection>,
) {
	commands.spawn(SceneRoot(asset_server.load("maps/example.map#Scene")));

	// Wide FOV
	#[cfg(feature = "example_client")]
	for mut projection in &mut projection_query {
		*projection = Projection::Perspective(PerspectiveProjection {
			fov: 90_f32.to_radians(),
			..default()
		});
	}
}

#[cfg(feature = "example_client")]
#[rustfmt::skip]
fn spawn_lights(
	mut commands: Commands,
	query: Query<(Entity, &Light), Changed<Light>>,
) {
	for (entity, light) in &query {
		commands.entity(entity).insert(PointLight {
			color: light._color,
			intensity: light.light * 1000.,
			shadows_enabled: true,
			..default()
		});
	}
}
