use bevy::prelude::*;
use bevy_flycam::prelude::*;
use bevy_trenchbroom::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin {
            default_sampler: repeating_image_sampler(false),
        }))

        // bevy_flycam setup so we can get a closer look at the scene, mainly for debugging
        .add_plugins(PlayerPlugin)
        .insert_resource(MovementSettings {
            sensitivity: 0.0001,
            speed: 4.,
        })

        .add_plugins(TrenchBroomPlugin::new(
            TrenchBroomConfig::new("bevy_trenchbroom_example").entity_definitions(
                entity_definitions! {
                    /// World Entity
                    Solid worldspawn {} |world, entity, view| {
                        // The order here matters, we want to smooth out curved surfaces *before* spawning the mesh with `pbr_mesh`.
                        view.spawn_brushes(world, entity, BrushSpawnSettings::new().smooth_by_default_angle().pbr_mesh());
                    }

                    /// A simple point entity example
                    Point test {} |world, entity, view| {
                        let asset_server = world.resource::<AssetServer>();
                        let cube = asset_server.add(Mesh::from(Cuboid::new(0.3, 0.3, 0.3)));
                        let material = asset_server.add(StandardMaterial::default());
                        world.entity_mut(entity).insert((
                            cube,
                            material,
                            VisibilityBundle::default(),
                        ));
                    }
                },
            ),
        ))
        .add_systems(Startup, (setup_scene, write_config))
        .run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(2., 3., 1.),
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });

    commands.spawn(MapBundle {
        map: asset_server.load("maps/example.map"),
        ..default()
    });
}

fn write_config(tb_config: Res<TrenchBroomConfig>) {
    std::fs::create_dir("target/example_config").ok();
    tb_config.write_folder("target/example_config").unwrap();
}