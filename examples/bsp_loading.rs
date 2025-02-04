use bevy::math::*;
use bevy::{
	ecs::{component::ComponentId, world::DeferredWorld},
	prelude::*,
};
use bevy_flycam::prelude::*;
use bevy_trenchbroom::prelude::*;
use nil::prelude::*;

#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
#[geometry(GeometryProvider::new().smooth_by_default_angle().render().with_lightmaps())]
pub struct Worldspawn;

#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
#[geometry(GeometryProvider::new().smooth_by_default_angle().render().with_lightmaps())]
pub struct FuncDoor;

#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
#[geometry(GeometryProvider::new().smooth_by_default_angle().render().with_lightmaps())]
pub struct FuncWall;

#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
#[geometry(GeometryProvider::new().smooth_by_default_angle().render().with_lightmaps())]
pub struct FuncIllusionary;

#[derive(PointClass, Component, Reflect)]
#[reflect(Component)]
#[require(Transform)]
#[component(on_add = Self::on_add)]
pub struct Cube;
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

#[derive(PointClass, Component, Reflect, SmartDefault)]
#[reflect(Component)]
#[require(Transform)]
pub struct Light {
	#[default(Color::srgb(1., 1., 1.))]
	pub _color: Color,
	#[default(300.)]
	pub light: f32,
	#[default(0)]
	pub delay: u8,
}

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(ImagePlugin {
			default_sampler: repeating_image_sampler(false),
		}))
		// bevy_flycam setup so we can get a closer look at the scene, mainly for debugging
		.add_plugins(PlayerPlugin)
		.insert_resource(MovementSettings {
			sensitivity: 0.00005,
			speed: 6.,
		})
		.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::default())
		.insert_resource(ClearColor(Color::BLACK))
		.insert_resource(AmbientLight::NONE)
		.add_plugins(TrenchBroomPlugin::new(
			TrenchBroomConfig::new("bevy_trenchbroom_example")
				.special_textures(SpecialTexturesConfig::new())
				.ignore_invalid_entity_definitions(true),
		))
		.add_systems(PostStartup, (setup_scene, write_config))
		.run();
}

fn setup_scene(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut projection_query: Query<(Entity, &mut Projection)>,
	mut lightmap_animators: ResMut<LightmapAnimators>,
) {
	// TODO TMP: For tears of the false god
	lightmap_animators
		.values
		.insert(LightmapStyle(5), LightmapAnimator::new(0.5, true, [0.2, 1.].map(Vec3::splat)));

	commands.spawn(SceneRoot(asset_server.load("maps/example.bsp#Scene")));
	// commands.spawn(SceneRoot(asset_server.load("maps/arcane/ad_tfuma.bsp#Scene")));

	let sphere_mesh = asset_server.add(Sphere::new(0.1).mesh().build());
	let material = asset_server.add(StandardMaterial::default());

	// Wide FOV
	for (entity, mut projection) in &mut projection_query {
		*projection = Projection::Perspective(PerspectiveProjection {
			fov: 90_f32.to_radians(),
			..default()
		});

		// TODO tmp
		let gi_tester = commands
			.spawn((
				Mesh3d(sphere_mesh.clone()),
				MeshMaterial3d(material.clone()),
				Transform::from_xyz(0., -0.2, -0.3),
			))
			.id();

		commands.entity(entity).add_child(gi_tester);
	}
}

fn write_config(server: Res<TrenchBroomServer>) {
	#[cfg(not(target_arch = "wasm32"))]
	{
		std::fs::create_dir("target/example_config").ok();
		server.config.write_folder("target/example_config").unwrap();
	}
}
