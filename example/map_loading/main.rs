#![allow(unexpected_cfgs)]

#[cfg(feature = "example_client")]
use bevy::ecs::{lifecycle::HookContext, world::DeferredWorld};
use bevy::math::*;
use bevy::prelude::*;
use bevy_trenchbroom::class::builtin::LightingClassesPlugin;
use bevy_trenchbroom::class::builtin::LightingWorkflow;
#[allow(unused)]
use bevy_trenchbroom::config::WriteTrenchBroomConfigOnStartPlugin;
use bevy_trenchbroom::prelude::*;
use nil::prelude::*;

#[solid_class]
pub struct FuncDetail;

#[solid_class]
pub struct FuncDoor;

#[point_class]
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

#[point_class(
	model("models/mushroom.glb"),
	size(-4 -4 0, 4 4 16),
	hooks(SceneHooks::new().spawn_class_gltf::<Self>()),
)]
pub struct Mushroom;

// This is a custom light class for parity with bsp_loading, if you don't support bsps, you should use `PointLight` as base class instead.
#[point_class]
#[derive(Clone, Copy, SmartDefault)]
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

fn main() {
	// Unfortunately, due to Rust limitations on attributes, we have to build this through a variable.
	#[allow(unused_mut)]
	let mut trenchbroom_plugins = TrenchBroomPlugins(
		TrenchBroomConfig::new("bevy_trenchbroom_example").default_solid_scene_hooks(|| SceneHooks::new().smooth_by_default_angle()),
	)
	.build()
	// This is because we use a custom light class for parity with bsp_loading.
	.set(LightingClassesPlugin(LightingWorkflow::Custom));

	// I use bsp_loading to write the config.
	// This is feature locked for headless to work.
	#[cfg(all(not(target_family = "wasm"), feature = "example_client"))]
	{
		trenchbroom_plugins = trenchbroom_plugins.disable::<WriteTrenchBroomConfigOnStartPlugin>();
	}

	App::new()
		.add_plugins(DefaultPlugins.set(AssetPlugin {
			file_path: "../../assets".s(),
			..default()
		}))
		.add_plugins(trenchbroom_plugins)
		.add_plugins(example_commons::ExampleCommonsPlugin)
		.add_systems(PostStartup, setup_scene)
		.run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
	commands.spawn(SceneRoot(asset_server.load("maps/example.map#Scene")));

	#[cfg(feature = "example_client")]
	commands.spawn((
		example_commons::DebugCamera,
		Projection::Perspective(PerspectiveProjection {
			fov: 90_f32.to_radians(),
			..default()
		}),
		example_commons::default_debug_camera_transform(),
	));
}
