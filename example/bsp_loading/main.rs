#![allow(unexpected_cfgs)]

#[cfg(feature = "example_client")]
use bevy::ecs::{component::ComponentId, world::DeferredWorld};
use bevy::math::*;
use bevy::prelude::*;
#[cfg(feature = "example_client")]
use bevy_flycam::prelude::*;
use bevy_trenchbroom::bsp::base_classes::*;
use bevy_trenchbroom::fgd::FgdFlags;
use bevy_trenchbroom::prelude::*;
use enumflags2::*;
use nil::prelude::*;

// TODO: We aren't using inventory to register here because it's broken on wasm.

#[derive(SolidClass, Component, Reflect)]
#[no_register]
#[reflect(Component)]
#[require(BspWorldspawn)]
#[geometry(GeometryProvider::new().smooth_by_default_angle().with_lightmaps())]
pub struct Worldspawn;

#[derive(SolidClass, Component, Reflect, Default)]
#[no_register]
#[reflect(Component)]
#[require(BspSolidEntity, Transform)]
#[geometry(GeometryProvider::new().smooth_by_default_angle().with_lightmaps())]
pub struct FuncDoor {
	pub awesome: FgdFlags<FlagsTest>,
}

#[bitflags(default = Beep | Bap)]
#[derive(Reflect, Debug, Clone, Copy)]
#[repr(u16)]
pub enum FlagsTest {
	/// Boop flag title
	Boop = 1,
	Beep = 1 << 1,
	Bap = 1 << 2,
}

#[derive(SolidClass, Component, Reflect)]
#[no_register]
#[reflect(Component)]
#[require(BspSolidEntity)]
#[geometry(GeometryProvider::new().smooth_by_default_angle().with_lightmaps())]
pub struct FuncWall;

#[derive(SolidClass, Component, Reflect)]
#[no_register]
#[reflect(Component)]
#[require(BspSolidEntity)]
#[geometry(GeometryProvider::new())] // Compiler-handled
pub struct FuncDetail;

#[derive(SolidClass, Component, Reflect)]
#[no_register]
#[reflect(Component)]
#[require(BspSolidEntity)]
#[geometry(GeometryProvider::new().smooth_by_default_angle().with_lightmaps())]
pub struct FuncIllusionary;

#[derive(PointClass, Component, Reflect)]
#[no_register]
#[reflect(Component)]
#[require(Transform)]
#[cfg_attr(feature = "example_client", component(on_add = Self::on_add))]
pub struct Cube;
#[cfg(feature = "example_client")]
impl Cube {
	fn on_add(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
		// This isn't necessary because we get AssetServer here, this is mainly for example.
		if world.is_scene_world() {
			return;
		}
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

#[derive(PointClass, Component, Reflect, SmartDefault)]
#[no_register]
#[reflect(Component)]
#[require(BspLight, Transform)]
// This is the default size, this is just to make sure it produces a valid fgd.
#[size(-8 -8 -8, 8 8 8)]
#[iconsprite({ path: "point_light.png", scale: 0.1 })]
pub struct Light;

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
				.suppress_invalid_entity_definitions(true)
				.register_class::<Worldspawn>()
				.register_class::<Cube>()
				.register_class::<Mushroom>()
				.register_class::<FuncWall>()
				.register_class::<Light>()
				.register_class::<FuncDoor>(),
		))
		.add_systems(PostStartup, (setup_scene, write_config))
		.run();
}

fn setup_scene(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	#[cfg(feature = "example_client")] mut projection_query: Query<&mut Projection>,
	#[cfg(feature = "example_client")] mut lightmap_animators: ResMut<LightingAnimators>,
) {
	#[cfg(feature = "example_client")]
	{
		commands.insert_resource(AmbientLight::NONE);

		lightmap_animators.values.insert(
			LightmapStyle(1),
			LightingAnimator::new(6., 0.7, [0.8, 0.75, 1., 0.7, 0.8, 0.7, 0.9, 0.7, 0.6, 0.7, 0.9, 1., 0.7].map(Vec3::splat)),
		);
		lightmap_animators
			.values
			.insert(LightmapStyle(2), LightingAnimator::new(0.5, 1., [0., 1.].map(Vec3::splat)));
		lightmap_animators
			.values
			.insert(LightmapStyle(5), LightingAnimator::new(0.5, 1., [0.2, 1.].map(Vec3::splat)));
	}

	commands.spawn(SceneRoot(asset_server.load("maps/example.bsp#Scene")));
	// commands.spawn(SceneRoot(asset_server.load("maps/arcane/ad_tfuma.bsp#Scene")));

	// Wide FOV
	#[cfg(feature = "example_client")]
	for mut projection in &mut projection_query {
		*projection = Projection::Perspective(PerspectiveProjection {
			fov: 90_f32.to_radians(),
			..default()
		});
	}
}

fn write_config(#[allow(unused)] server: Res<TrenchBroomServer>) {
	#[cfg(not(target_family = "wasm"))]
	{
		server.config.write_game_config_to_default_directory().unwrap();
		server.config.add_game_to_preferences_in_default_directory().unwrap();
	}
}
