use bevy::{ecs::{component::ComponentId, world::DeferredWorld}, prelude::*};
use bevy_flycam::prelude::*;
use bevy_trenchbroom::{bsp::BspDataAsset, prelude::*};
use bevy::math::*;
use nil::prelude::*;
use bevy_trenchbroom::util::BevyTrenchbroomCoordinateConversions;

use avian3d::prelude::*;

#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
#[geometry(GeometryProvider::new().trimesh_collider().smooth_by_default_angle().render().with_lightmaps())]
pub struct Worldspawn;

#[derive(SolidClass, Component, Reflect)]
#[reflect(Component)]
#[geometry(GeometryProvider::new().trimesh_collider().smooth_by_default_angle().render().with_lightmaps())]
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
        let Some(asset_server) = world.get_resource::<AssetServer>() else { return };
        let cube = asset_server.add(Mesh::from(Cuboid::new(0.42, 0.42, 0.42)));
        let material = asset_server.add(StandardMaterial::default());

        world.commands().entity(entity).insert((
            Mesh3d(cube),
            MeshMaterial3d(material),
        ));
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
        .add_plugins(DefaultPlugins
            .set(ImagePlugin {
                default_sampler: repeating_image_sampler(false),
            })
        )

        // bevy_flycam setup so we can get a closer look at the scene, mainly for debugging
        .add_plugins(PlayerPlugin)
        .insert_resource(MovementSettings {
            sensitivity: 0.00005,
            speed: 6.,
        })

        .add_plugins((PhysicsPlugins::default(), PhysicsDebugPlugin::default()))
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::default())
        // .add_plugins(bevy::pbr::wireframe::WireframePlugin)
        // .insert_resource(bevy::pbr::wireframe::WireframeConfig { global: true, default_color: Color::WHITE })
        // .insert_resource(AmbientLight { color: Color::WHITE, brightness: 500. })
        // .insert_resource(bevy::pbr::DefaultOpaqueRendererMethod::deferred()) // TODO
        
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight::NONE)

        .add_plugins(TrenchBroomPlugin::new(
            TrenchBroomConfig::new("bevy_trenchbroom_example")
                .special_textures(SpecialTexturesConfig::new())
                .ignore_invalid_entity_definitions(true)
                // .loose
        ))
        .add_systems(PostStartup, (setup_scene, write_config))
        .add_systems(Update, visualize)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut projection_query: Query<(Entity, &mut Projection)>,
    mut lightmap_animators: ResMut<LightmapAnimators>,
    mut gizmo_configs: ResMut<GizmoConfigStore>,
) {
    gizmo_configs.config_mut::<DefaultGizmoConfigGroup>().0.depth_bias = -1.;
    
    // TODO TMP: For tears of the false god
    lightmap_animators.values.insert(LightmapStyle(5), LightmapAnimator::new(0.5, true, [0.2, 1.].map(Vec3::splat)));
    // lightmap_animators.values.clear();
    
    commands.spawn(SceneRoot(asset_server.load("maps/example.bsp#Scene")));
    // commands.spawn(SceneRoot(asset_server.load("maps/arcane/ad_tfuma.bsp#Scene")));
    
    let sphere_mesh = asset_server.add(Sphere::new(0.1).mesh().build());
    let material = asset_server.add(StandardMaterial::default());

    commands.spawn((
        RigidBody::Dynamic,
        Collider::cuboid(0.5, 0.5, 0.5),
        Transform::from_xyz(0.5, 2.5, 0.5),

        Mesh3d(asset_server.add(Cuboid::from_length(0.5).into())),
        MeshMaterial3d(material.clone()),
    ));

    // Wide FOV
    for (entity, mut projection) in &mut projection_query {
        *projection = Projection::Perspective(PerspectiveProjection {
            fov: 90_f32.to_radians(),
            ..default()
        });

        // TODO tmp
        let gi_tester = commands.spawn((
            Mesh3d(sphere_mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0., -0.2, -0.3),
        )).id();

        commands.entity(entity).add_child(gi_tester);
    }

    // TODO tmp
    /* for x in 1..=9 {
        for y in 1..=7 {
            for z in 0..9 {
                commands.spawn(PbrBundle {
                    mesh: sphere_mesh.clone(),
                    material: material.clone(),
                    transform: Transform::from_translation(vec3(x as f32, y as f32, z as f32 - 1.) * vec3(0.8128, 0.8128, -0.8128) + vec3(-1.6256, 0., 1.6256) - 0.8128),
                    ..default()
                });
            }
        }
    } */

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

fn visualize(
    mut gizmos: Gizmos,
    // irradiance_volume_query: Query<&Transform, With<IrradianceVolume>>,

    camera_query: Query<&Transform, With<Camera>>,
    // mouse_buttons: Res<ButtonInput<MouseButton>>,
    bsp_data_assets: Res<Assets<BspDataAsset>>,
    tb_server: Res<TrenchBroomServer>,

    // mut hit_point: Local<Option<Transform>>,
) {
    // for transform in &irradiance_volume_query {
        // gizmos.cuboid(*transform, Color::WHITE);
    // }

    // if mouse_buttons.just_pressed(MouseButton::Left) {
    for (_, BspDataAsset(bsp_data)) in bsp_data_assets.iter() {
        /* for model in &bsp_data.models {
            let bounding_box = model.bound;
        
            let aabb = Aabb::from_min_max(tb_server.config.to_bevy_space(bounding_box.min), tb_server.config.to_bevy_space(bounding_box.max));
                
            gizmos.cuboid(Transform::from_translation(aabb.center.into()).with_scale((aabb.half_extents * 2.).into()), Color::WHITE);
        } */
        
        let camera_transform = camera_query.single();

        let from = camera_transform.translation;
        let to = from + camera_transform.forward() * 50.;
        
        // println!("from {} to {}", tb_server.config.from_bevy_space(from), tb_server.config.from_bevy_space(to));
        
        let raycast = bsp_data.raycast(0, tb_server.config.from_bevy_space(from), tb_server.config.from_bevy_space(to));
        if let Some(hit) = raycast.impact {
            let position = from.lerp(to, hit.fraction);
            gizmos.axes(Transform::from_translation(position).looking_to(hit.normal.z_up_to_y_up(), Vec3::Y), 0.5);
            
            // println!("faces: {}", bsp_data.leaves[raycast.leaf_idx].face_num.bsp2());
            // let node = &bsp_data.nodes[hit.node_idx as usize];
            // let plane = bsp_data.planes[node.plane_idx as usize];
            // let transform = Transform::from_translation(tb_server.config.to_bevy_space(plane.normal * plane.dist)).looking_to(plane.normal.z_up_to_y_up(), Vec3::Y);

            // gizmos.rect(transform.to_isometry(), Vec2::splat(5.), Color::WHITE);
            // let bounding_box = node.bound.bsp2();
            // println!("{bounding_box:?}");
            // let aabb = Aabb::from_min_max(tb_server.config.to_bevy_space(bounding_box.min), tb_server.config.to_bevy_space(bounding_box.max));
            
            // gizmos.cuboid(Transform::from_translation(aabb.center.into()).with_scale((aabb.half_extents * 2.).into()), Color::WHITE);
        }
    }
        
        // *hit_point = Some(camera_transform.translation + camera_transform.forward() * 10.);
    // }

    // if let Some(hit_point) = *hit_point {
    //     gizmos.axes(hit_point, 0.5);
    // }
}

fn write_config(server: Res<TrenchBroomServer>) {
    #[cfg(not(target_arch = "wasm32"))] {
        std::fs::create_dir("target/example_config").ok();
        server.config.write_folder("target/example_config").unwrap();
    }
}