use std::collections::HashSet;

use crate::*;

/// Plugin for supporting quake special textures, such as animated textures and liquid.
/// 
/// Add this along with your [TrenchBroomPlugin].
pub struct TrenchBroomSpecialTexturesPlugin;
impl Plugin for TrenchBroomSpecialTexturesPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, Self::animate_textures)
        ;
    }
}
impl TrenchBroomSpecialTexturesPlugin {
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
            *last_frame_update += tb_server.config.texture_animation_speed; // TODO configurable animation speed

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
    
                    // println!("parsing {frame_str}: {:?}", frame_str.trim().parse::<u8>());
                    // println!("parsing {frame_str}: expected: {:?}, found: {:?}", "0".as_bytes(), frame_str.as_bytes());
                    let Ok(mut frame_num) = frame_str.parse::<u8>() else { continue };
                    frame_num += 1;

                    let texture_name = chars.collect::<String>();
    
                    // Loop to run this code again if we need to loop back around.
                    for _ in 0..2 {
                        let new_file_name = format!("+{frame_num}{texture_name}");
                        match map.embedded_textures.get(&new_file_name).cloned().or_else(|| asset_server.get_handle::<Image>(&new_file_name)) {
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