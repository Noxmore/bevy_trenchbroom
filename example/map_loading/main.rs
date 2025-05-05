#![allow(unexpected_cfgs)]

#[cfg(feature = "example_client")]
use bevy::ecs::{component::HookContext, world::DeferredWorld};
use bevy::math::*;
use bevy::prelude::*;
#[cfg(feature = "example_client")]
use bevy_flycam::prelude::*;
use bevy_trenchbroom::prelude::*;
use nil::prelude::*;

#[derive(SolidClass, Component, Reflect)]
#[reflect(QuakeClass, Component)]
#[geometry(GeometryProvider::new().smooth_by_default_angle())]
pub struct Worldspawn;

#[derive(SolidClass, Component, Reflect)]
#[reflect(QuakeClass, Component)]
#[geometry(GeometryProvider::new().smooth_by_default_angle())]
pub struct FuncDoor;

#[derive(PointClass, Component, Reflect)]
#[reflect(QuakeClass, Component)]
#[cfg_attr(feature = "example_client", component(on_add = Self::on_add))]
pub struct Cube;
#[cfg(feature = "example_client")]
impl Cube {
	fn on_add(mut world: DeferredWorld, ctx: HookContext) {
		let Some(asset_server) = world.get_resource::<AssetServer>() else { return };
		let cube = asset_server.add(Mesh::from(Cuboid::new(0.42, 0.42, 0.42)));
		let material = asset_server.add(StandardMaterial::default());

		world.commands().entity(ctx.entity).insert((Mesh3d(cube), MeshMaterial3d(material)));
	}
}

#[derive(PointClass, Component, Reflect)]
#[reflect(QuakeClass, Component)]
#[model("models/mushroom.glb")]
#[size(-4 -4 0, 4 4 16)]
#[spawn_hook(spawn_class_gltf::<Self>)]
pub struct Mushroom;

// This is a custom light class for parity with bsp_loading, if you don't support bsps, you should use `PointLight` as base class instead.
#[derive(PointClass, Component, Reflect, Clone, Copy, SmartDefault)]
#[reflect(QuakeClass, Component)]
#[cfg_attr(feature = "example_client", component(on_add = Self::on_add))]
pub struct Light {
	#[default(Color::srgb(1., 1., 1.))]
	pub _color: Color,
	#[default(300.)]
	pub light: f32,
}

#[cfg(feature = "example_client")]
impl Light {
	pub fn on_add(mut world: DeferredWorld, ctx: HookContext) {
		world.commands().entity(ctx.entity).queue(|mut entity: EntityWorldMut| {
			let Some(light) = entity.get::<Self>() else { return };

			entity.insert(PointLight {
				color: light._color,
				intensity: light.light * 1000.,
				shadows_enabled: true,
				..default()
			});
		});
	}
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
			.add_plugins(bevy_inspector_egui::bevy_egui::EguiPlugin { enable_multipass_for_primary_context: true })
			.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::default())
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
			TrenchBroomConfig::new("bevy_trenchbroom_example").no_bsp_lighting(true),
		))
		.register_type::<Worldspawn>()
		.register_type::<Cube>()
		.register_type::<Mushroom>()
		.register_type::<Light>()
		.register_type::<FuncDoor>()
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
