use std::collections::HashSet;

use crate::*;

/// Config for supporting quake special textures, such as animated textures and liquid.
#[derive(Debug, Clone, SmartDefault)]
pub struct SpecialTexturesConfig {
    /// Seconds per frame for special animated textures. (Default: 5 FPS)
    #[default(1. / 5.)]
    pub texture_animation_speed: f32,

    #[default(vec!["clip".s(), "skip".s()])]
    pub invisible_textures: Vec<String>,
}

pub(crate) struct SpecialTexturesPlugin;
impl Plugin for SpecialTexturesPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, Self::animate_textures)
        ;
    }
}
impl SpecialTexturesPlugin {
    pub fn animate_textures(
        map_entity_query: Query<(&MapEntityRef, &Children), With<SpawnedMapEntity>>,
        mesh_query: Query<&Handle<StandardMaterial>, With<Handle<Mesh>>>,
        maps: Res<Assets<Map>>,
        asset_server: Res<AssetServer>,
        mut materials: ResMut<Assets<StandardMaterial>>, // TODO support other user-made materials?

        tb_server: Res<TrenchBroomServer>,
        time: Res<Time>,
        mut last_frame_update: Local<f32>,
    ) {
        while *last_frame_update < time.elapsed_seconds() {
            *last_frame_update += tb_server.config.special_textures_config().texture_animation_speed;

            for (map_entity_ref, children) in &map_entity_query {
                let Some(map_handle) = &map_entity_ref.map_handle else { continue };
                let Some(map) = maps.get(map_handle) else { continue };

                let mut updated_materials = HashSet::new();
    
                for material_handle in mesh_query.iter_many(children) {
                    if updated_materials.contains(material_handle) { continue }

                    let Some(material) = materials.get_mut(material_handle) else { continue };
                    let Some(image_handle) = &material.base_color_texture else { continue };
                    let Some(image_path) = image_handle.path() else { continue };
                    // We check the label first, because if it was loaded from a BSP, the path will be the BSP file's path
                    // If there is no label, the image was probably loaded from a loose file
                    let Some(file_name) = image_path.label().or_else(|| image_path.path().file_name().map(|x| x.to_str()).flatten()) else { continue };
                    let file_name = file_name.to_string();
                    
                    let mut chars = file_name.chars();
                    if chars.next() != Some('+') { continue }
                    let mut frame_str = [0; 4];
                    let Some(frame_char) = chars.next() else { continue };
                    frame_char.encode_utf8(&mut frame_str);
                    // Trim trailing null bytes because `frame_str` has a size of 4 for safety.
                    // SAFETY: char is always valid utf-8
                    let frame_str = unsafe { std::str::from_utf8_unchecked(&frame_str) }.trim_end_matches('\0');
    
                    let Ok(mut frame_num) = frame_str.parse::<u8>() else { continue };
                    frame_num += 1;

                    let texture_name = chars.collect::<String>();
    
                    // Loop to run this code again if we need to loop back around.
                    for _ in 0..2 {
                        let new_file_name = format!("+{frame_num}{texture_name}");
                        match map.embedded_textures.get(&new_file_name).map(|embedded| &embedded.image_handle).cloned().or_else(|| asset_server.get_handle::<Image>(&new_file_name)) {
                            Some(new_handle) => {
                                material.base_color_texture = Some(new_handle);
                                break;
                            },
                            None => {
                                frame_num = 0;
                            }
                        }
                    }

                    updated_materials.insert(material_handle);
                }
            }
        }
    }
}
impl TrenchBroomConfig {
    /// Retrieves the special textures config or panics.
    #[inline]
    #[track_caller]
    pub(crate) fn special_textures_config(&self) -> &SpecialTexturesConfig {
        self.special_textures.as_ref().expect("Special textures config required! This is a bug!")
    }
    
    /// An optional configuration for supporting [Quake special textures](https://quakewiki.org/wiki/Textures),
    /// such as animated textures, liquids, invisible textures like clip and skip.
    pub fn special_textures(mut self, config: SpecialTexturesConfig) -> Self {
        self.special_textures = Some(config);

        self.global_brush_spawners.push(|world, _entity, view| {
            for mesh_view in &view.meshes {
                if view.server.config.special_textures_config().invisible_textures.contains(&mesh_view.texture.name) {
                    world.entity_mut(mesh_view.entity).insert(Visibility::Hidden);
                }
            }
        });

        self
    }
}