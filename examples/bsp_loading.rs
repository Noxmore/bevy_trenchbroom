use bevy::prelude::*;
use bevy_flycam::prelude::*;
use bevy_trenchbroom::prelude::*;

fn main() {
    std::fs::remove_file("tmp.txt").ok(); // TODO
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin {
            default_sampler: repeating_image_sampler(false),
        }))

        // bevy_flycam setup so we can get a closer look at the scene, mainly for debugging
        .add_plugins(PlayerPlugin)
        .insert_resource(MovementSettings {
            sensitivity: 0.00005,
            speed: 10.,
        })
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::default())
        // .add_plugins(bevy::pbr::wireframe::WireframePlugin)
        // .insert_resource(bevy::pbr::wireframe::WireframeConfig { global: true, default_color: Color::WHITE })
        // .insert_resource(AmbientLight { color: Color::WHITE, brightness: 500. })
        .insert_resource(AmbientLight::NONE)

        .add_plugins(TrenchBroomPlugin::new(
            // TODO
            TrenchBroomConfig::new("bevy_trenchbroom_example").special_textures(SpecialTexturesConfig::default()).entity_definitions(
                entity_definitions! {
                    /// World Entity
                    Solid worldspawn {} |world, entity, view| {
                        // The order here matters, we want to smooth out curved surfaces *before* spawning the mesh with `pbr_mesh`.
                        view.spawn_brushes(world, entity, BrushSpawnSettings::new().smooth_by_default_angle().pbr_mesh().with_lightmaps());
                    }

                    // TMP
                    Solid func_wall {} |world, entity, view| {
                        view.spawn_brushes(world, entity, BrushSpawnSettings::new().smooth_by_default_angle().pbr_mesh().with_lightmaps());
                    }

                    // TODO TMP
                    Solid func_door {} |world, entity, view| {
                        view.spawn_brushes(world, entity, BrushSpawnSettings::new().smooth_by_default_angle().pbr_mesh().with_lightmaps());
                    }

                    /// A simple point entity example
                    Point cube {} |world, entity, view| {
                        let asset_server = world.resource::<AssetServer>();
                        let cube = asset_server.add(Mesh::from(Cuboid::new(0.42, 0.42, 0.42)));
                        let material = asset_server.add(StandardMaterial::default());
                        world.entity_mut(entity).insert((
                            cube,
                            material,
                            VisibilityBundle::default(),
                        ));
                    }

                    /// Point light
                    Point light {
                        color: Color,
                        intensity: f32,
                    } |world, entity, view| {
                        // world.entity_mut(entity).insert(PointLightBundle {
                        //     point_light: PointLight {
                        //         color: view.get("color")?,
                        //         intensity: view.get("intensity")?,
                        //         shadows_enabled: true,
                        //         ..default()
                        //     },
                        //     ..default()
                        // });
                    }
                },
            ),
        ))
        .add_systems(PostStartup, (setup_scene, write_config))
        // .add_systems(Update, visualize_stuff)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut projection_query: Query<&mut Projection>,
) {
    commands.spawn(MapBundle {
        map: asset_server.load("maps/ad_crucial.bsp"), // ad_crucial
        ..default()
    });

    // Wide FOV
    for mut projection in &mut projection_query {
        *projection = Projection::Perspective(PerspectiveProjection {
            fov: 90_f32.to_radians(),
            ..default()
        });
    }
}

fn write_config(server: Res<TrenchBroomServer>) {
    std::fs::create_dir("target/example_config").ok();
    server.config.write_folder("target/example_config").unwrap();
    // tb_config.write_wad("target/example_config/textures.wad").unwrap();
}

// fn visualize_stuff(mut gizmos: Gizmos) {
//     let tmp_debug = TMP_DEBUG.lock().unwrap();
    
//     for vertex in &tmp_debug.0 {
//         gizmos.sphere(*vertex, default(), 0.03, Color::WHITE);
//     }

//     for edge in tmp_debug.1.iter().take(12) {
//         gizmos.line(tmp_debug.0[edge.a as usize], tmp_debug.0[edge.b as usize], Color::WHITE);
//     }
// }