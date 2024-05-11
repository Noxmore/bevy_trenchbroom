use bevy::prelude::*;
use bevy_trenchbroom::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin {
            default_sampler: repeating_image_sampler(false),
        }))
        .add_plugins(TrenchBroomPlugin::new(
            TrenchBroomConfig::new("bevy_trenchbroom_example").entity_definitions(
                entity_definitions! {
                    /// World Entity
                    Solid worldspawn {} |world, entity, view| {
                        view.spawn_brushes(world, entity, BrushSpawnSettings::new().pbr_mesh());
                    }
                },
            ),
        ))
        .add_systems(Startup, (setup_scene, write_config))
        .run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(7., 8., 7.).looking_at(Vec3::Y, Vec3::Y),
        ..default()
    });

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
