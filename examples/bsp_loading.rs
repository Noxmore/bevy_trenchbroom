use bevy::prelude::*;
use bevy_flycam::prelude::*;
use bevy_trenchbroom::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin {
            default_sampler: repeating_image_sampler(true),
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
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight::NONE)

        .add_plugins(TrenchBroomPlugin::new(
            // TODO
            TrenchBroomConfig::new("bevy_trenchbroom_example").special_textures(SpecialTexturesConfig::new()).entity_definitions(
                entity_definitions! {
                    /// World Entity
                    Solid worldspawn {} |world, entity, view| {
                        // The order here matters, we want to smooth out curved surfaces *before* spawning the mesh with `pbr_mesh`.
                        view.spawn_brushes(world, entity, BrushSpawnSettings::new().smooth_by_default_angle().pbr_mesh().with_lightmaps());
                        // bevy::pbr::irradiance_volume
                        // bevy::pbr::environment_map
                        // Light

                        /* if let Ok(fog_settings) = (|| -> Result<FogSettings, MapEntitySpawnError> {
                            let fog = view.get::<Vec4>("fog");
                            Ok(FogSettings {
                                color: Color::srgb_from_array(fog.clone().map(|fog| fog.yzw()).or_else(|_| view.get("fog_colour")).or_else(|_| view.get("fog_color"))?.to_array()),
                                // TODO this doesn't quite match
                                // falloff: FogFalloff::ExponentialSquared { density: 0.0005 },
                                falloff: FogFalloff::ExponentialSquared { density: fog.map(|fog| fog.x).or_else(|_| view.get("fog_density"))? / view.server.config.scale },
                                ..default()
                            })
                        })() {
                            for entity in world.query_filtered::<Entity, With<Camera3d>>().iter(world).collect_vec() {
                                world.entity_mut(entity).insert(fog_settings.clone());
                            }
                        } */

                        /* let directional_light: Result<(DirectionalLight, Transform), MapEntitySpawnError> = (|| {
                            let mangle = view.get::<Vec3>("_sunlight_mangle").or_else(|_| view.get::<Vec3>("_sun_mangle"))?;
                            Ok((
                                DirectionalLight {
                                    color: view.get("_sunlight_color")?,
                                    illuminance: quake_light_to_lux(view.get::<f32>("_sunlight")?),
                                    shadows_enabled: true,
                                    ..default()
                                },
                                Transform::from_rotation(mangle_to_quat(mangle)),
                            ))
                        })();
                        if let Ok((directional_light, transform)) = directional_light {
                            let directional_light = world.spawn(DirectionalLightBundle {
                                directional_light,
                                transform,
                                ..default()
                            }).id();
    
                            world.entity_mut(entity).add_child(directional_light);
                        } */
                    }

                    Solid func_wall {} |world, entity, view| {
                        view.spawn_brushes(world, entity, BrushSpawnSettings::new().smooth_by_default_angle().pbr_mesh().with_lightmaps());
                    }

                    Solid func_door {} |world, entity, view| {
                        view.spawn_brushes(world, entity, BrushSpawnSettings::new().smooth_by_default_angle().pbr_mesh().with_lightmaps());
                        // panic!("{:#?}", view.map_entity);
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
    mut projection_query: Query<(Entity, &mut Projection)>,
) {
    commands.spawn(MapBundle {
        map: asset_server.load("maps/ad_crucial.bsp"),
        ..default()
    });

    // Wide FOV
    for (entity, mut projection) in &mut projection_query {
        *projection = Projection::Perspective(PerspectiveProjection {
            fov: 90_f32.to_radians(),
            ..default()
        });

        // TODO tmp
        let gi_tester = commands.spawn(PbrBundle {
            mesh: asset_server.add(Sphere::new(0.1).mesh().build()),
            material: asset_server.add(StandardMaterial::default()),
            transform: Transform::from_xyz(0., -0.2, -0.3),
            ..default()
        }).id();

        commands.entity(entity).add_child(gi_tester);
    }

    /* commands.spawn(MaterialMeshBundle {
        mesh: asset_server.add(Cuboid::from_length(0.5).mesh().build()),
        material: asset_server.add(LiquidMaterial {
            base: StandardMaterial {
                base_color_texture: Some(asset_server.load("textures/bricks.png")),
                // emissive: LinearRgba::WHITE / 2.,
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                ..default()
            },
            extension: LiquidMaterialExt::default(),
        }),
        // material: asset_server.add(SkyMaterial {
        //     texture: asset_server.load("textures/bricks.png"),
        //     speed: 1.,
        // }),
        transform: Transform::from_xyz(0., 5., 0.),
        ..default()
    }); */
    // commands.spawn(MaterialMeshBundle::<StandardMaterial> {
    //     mesh: meshes.add(Cuboid::from_length(0.5).mesh().build()),
    //     material: asset_server.add(StandardMaterial {
    //         base_color_texture: Some(asset_server.load("textures/bricks.png")),
    //         // emissive: LinearRgba::WHITE / 2.,
    //         unlit: true,
    //         ..default()
    //     }),
    //     transform: Transform::from_xyz(0., 5., 0.),
    //     ..default()
    // });
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