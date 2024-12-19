use std::collections::HashSet;

use bevy::{
    asset::embedded_asset,
    pbr::{ExtendedMaterial, MaterialExtension},
    render::render_resource::AsBindGroup,
};

use crate::*;

pub(crate) struct SpecialTexturesPlugin;
impl Plugin for SpecialTexturesPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "liquid.wgsl");
        embedded_asset!(app, "quake_sky.wgsl");

        app.add_plugins(MaterialPlugin::<LiquidMaterial>::default())
            .add_plugins(MaterialPlugin::<QuakeSkyMaterial>::default())
            .add_systems(Update, Self::animate_textures);
    }
}
impl SpecialTexturesPlugin {
    pub fn animate_textures(
        map_entity_query: Query<(&MapEntityRef, &Children), With<SpawnedMapEntity>>,
        mesh_query: Query<&MeshMaterial3d<StandardMaterial>, With<Mesh3d>>,
        maps: Res<Assets<Map>>,
        asset_server: Res<AssetServer>,
        mut materials: ResMut<Assets<StandardMaterial>>, // TODO support other user-made materials?

        tb_server: Res<TrenchBroomServer>,
        time: Res<Time>,
        mut last_frame_update: Local<f32>,
    ) {
        while *last_frame_update < time.elapsed_secs() {
            *last_frame_update += tb_server
                .config
                .special_textures_config()
                .texture_animation_speed;

            for (map_entity_ref, children) in &map_entity_query {
                let Some(map_handle) = &map_entity_ref.map_handle else {
                    continue;
                };
                let Some(map) = maps.get(map_handle) else {
                    continue;
                };

                let mut updated_materials = HashSet::new();

                for material_handle in mesh_query.iter_many(children) {
                    if updated_materials.contains(&material_handle.0) {
                        continue;
                    }

                    let Some(material) = materials.get_mut(material_handle) else {
                        continue;
                    };
                    let Some(image_handle) = &material.base_color_texture else {
                        continue;
                    };
                    let Some(image_path) = image_handle.path() else {
                        continue;
                    };
                    // We check the label first, because if it was loaded from a BSP, the path will be the BSP file's path
                    // If there is no label, the image was probably loaded from a loose file
                    let Some(file_name) = image_path
                        .label()
                        .or_else(|| image_path.path().file_name().map(|x| x.to_str()).flatten())
                    else {
                        continue;
                    };
                    let file_name = file_name.to_string();

                    let mut chars = file_name.chars();
                    if chars.next() != Some('+') {
                        continue;
                    }
                    let mut frame_str = [0; 4];
                    let Some(frame_char) = chars.next() else {
                        continue;
                    };
                    frame_char.encode_utf8(&mut frame_str);
                    // Trim trailing null bytes because `frame_str` has a size of 4 for safety.
                    // SAFETY: char is always valid utf-8
                    let frame_str =
                        unsafe { std::str::from_utf8_unchecked(&frame_str) }.trim_end_matches('\0');

                    let Ok(mut frame_num) = frame_str.parse::<u8>() else {
                        continue;
                    };
                    frame_num += 1;

                    let texture_name = chars.collect::<String>();

                    // Loop to run this code again if we need to loop back around.
                    for _ in 0..2 {
                        let new_file_name = format!("+{frame_num}{texture_name}");
                        match map
                            .embedded_textures
                            .get(&new_file_name)
                            .map(|embedded| &embedded.image_handle)
                            .cloned()
                            .or_else(|| asset_server.get_handle::<Image>(&new_file_name))
                        {
                            Some(new_handle) => {
                                material.base_color_texture = Some(new_handle);
                                break;
                            }
                            None => {
                                frame_num = 0;
                            }
                        }
                    }

                    updated_materials.insert(material_handle.0.clone());
                }
            }
        }
    }
}

/// Config for supporting quake special textures, such as animated textures and liquid.
#[derive(Debug, Clone, SmartDefault, DefaultBuilder)]
pub struct SpecialTexturesConfig {
    /// Seconds per frame for animated textures. (Default: 5 FPS)
    #[default(1. / 5.)]
    pub texture_animation_speed: f32,

    /// List of textures to made made invisible on map load.
    #[default(vec!["clip".s(), "skip".s()])]
    pub invisible_textures: Vec<String>,

    #[default(QuakeSkyMaterial::default)]
    pub default_quake_sky_material: fn() -> QuakeSkyMaterial,

    #[default(LiquidMaterialExt::default)]
    pub default_liquid_material: fn() -> LiquidMaterialExt,
}
impl SpecialTexturesConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a new invisible texture.
    pub fn invisible_texture(mut self, texture: impl ToString) -> Self {
        self.invisible_textures.push(texture.to_string());
        self
    }

    pub fn default_material_application_hook(
        mut material: StandardMaterial,
        mesh_view: &BrushMeshView,
        world: &mut World,
        view: &BrushSpawnView,
    ) {
        if let Some(config) = &view.server.config.special_textures {
            if mesh_view.texture.name.starts_with('*') {
                let water_alpha = view.get("water_alpha").unwrap_or(1.);
                if water_alpha != 1. {
                    material.alpha_mode = AlphaMode::Blend;
                    material.base_color = Color::linear_rgba(1., 1., 1., water_alpha);
                }
                let handle = world
                    .resource_mut::<Assets<LiquidMaterial>>()
                    .add(LiquidMaterial {
                        base: material,
                        extension: (config.default_liquid_material)(),
                    });
                world
                    .entity_mut(mesh_view.entity)
                    .insert(MeshMaterial3d(handle));
                return;
            } else if mesh_view.texture.name.starts_with("sky") {
                if let Some(texture) = material.base_color_texture {
                    let handle =
                        world
                            .resource_mut::<Assets<QuakeSkyMaterial>>()
                            .add(QuakeSkyMaterial {
                                texture,
                                ..(config.default_quake_sky_material)()
                            });
                    world
                        .entity_mut(mesh_view.entity)
                        .insert(MeshMaterial3d(handle));
                    return;
                }
            }
        }

        TrenchBroomConfig::default_material_application_hook(material, mesh_view, world, view)
    }
}

impl TrenchBroomConfig {
    /// Retrieves the special textures config or panics.
    #[inline]
    #[track_caller]
    pub(crate) fn special_textures_config(&self) -> &SpecialTexturesConfig {
        self.special_textures
            .as_ref()
            .expect("Special textures config required! This is a bug!")
    }

    /// An optional configuration for supporting [Quake special textures](https://quakewiki.org/wiki/Textures),
    /// such as animated textures, skies, liquids, and invisible textures like clip and skip.
    pub fn special_textures(mut self, config: SpecialTexturesConfig) -> Self {
        self.special_textures = Some(config);

        self.global_brush_spawners.push(|world, _entity, view| {
            for mesh_view in &view.meshes {
                if view
                    .server
                    .config
                    .special_textures_config()
                    .invisible_textures
                    .contains(&mesh_view.texture.name)
                {
                    world
                        .entity_mut(mesh_view.entity)
                        .insert(Visibility::Hidden);
                }
            }
        });

        self.material_application_hook = SpecialTexturesConfig::default_material_application_hook;

        self
    }
}

/// Material extension to [StandardMaterial] that emulates the wave effect of Quake liquid.
pub type LiquidMaterial = ExtendedMaterial<StandardMaterial, LiquidMaterialExt>;

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, SmartDefault)]
pub struct LiquidMaterialExt {
    #[uniform(100)]
    #[default(0.1)]
    pub magnitude: f32,
    #[uniform(100)]
    #[default(PI)]
    pub cycles: f32,
}
impl MaterialExtension for LiquidMaterialExt {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "embedded://bevy_trenchbroom/liquid.wgsl".into()
    }
}

/// Material that emulates the Quake sky.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone, SmartDefault)]
pub struct QuakeSkyMaterial {
    /// The speed the foreground layer moves.
    #[uniform(0)]
    #[default(0.1)]
    pub fg_speed: f32,
    /// The speed the background layer moves.
    #[uniform(0)]
    #[default(0.05)]
    pub bg_speed: f32,
    /// The scale of the textures.
    #[uniform(0)]
    #[default(2.)]
    pub texture_scale: f32,
    /// Scales the sphere before it is re-normalized, used to shape it.
    #[uniform(0)]
    #[default(vec3(1., 3., 1.))]
    pub sphere_scale: Vec3,

    /// Must be twice as wide as it is tall. The left side is the foreground, where any pixels that are black will show the right side -- the background.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
}
impl Material for QuakeSkyMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        "embedded://bevy_trenchbroom/quake_sky.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }
}
