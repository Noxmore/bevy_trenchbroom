use std::collections::HashSet;

use bevy::{asset::embedded_asset, pbr::{ExtendedMaterial, MaterialExtension}, render::render_resource::AsBindGroup};
use bevy_materialize::animation::{GenericMaterialAnimationState, MaterialAnimation, MaterialAnimations};
use bsp::GENERIC_MATERIAL_PREFIX;
use geometry::GeometryProviderView;

use crate::*;

pub(crate) struct SpecialTexturesPlugin;
impl Plugin for SpecialTexturesPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "liquid.wgsl");
        embedded_asset!(app, "quake_sky.wgsl");
        
        app
            .add_plugins(MaterialPlugin::<LiquidMaterial>::default())
            .add_plugins(MaterialPlugin::<QuakeSkyMaterial>::default())
        ;
    }
}

/// Config for supporting quake special textures, such as animated textures and liquid.
#[derive(Debug, Clone, SmartDefault, DefaultBuilder)]
pub struct SpecialTexturesConfig {
    /// Default frames per second for animated textures. (Default: 5)
    #[default(5.)]
    pub texture_animation_fps: f32,

    /// Set of textures to made made invisible on map load. (Default: ["clip", "skip"])
    #[default(["clip".s(), "skip".s()].into())]
    #[builder(into)]
    pub invisible_textures: HashSet<String>,

    #[default(QuakeSkyMaterial::default)]
    pub default_quake_sky_material: fn() -> QuakeSkyMaterial,

    #[default(LiquidMaterialExt::default)]
    pub default_liquid_material: fn() -> LiquidMaterialExt,
}
impl SpecialTexturesConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a new invisible texture.
    pub fn invisible_texture(mut self, texture: impl ToString) -> Self {
        self.invisible_textures.insert(texture.to_string());
        self
    }
}

/// If a [SpecialTexturesConfig] is part of the config in `view`, this attempts to load [Quake special textures](https://quakewiki.org/wiki/Textures) using the material provided as a base.
pub fn load_special_texture(view: &mut TextureLoadView, material: &StandardMaterial) -> Option<GenericMaterial> {
    let Some(special_textures_config) = &view.tb_config.special_textures else { return None };

    fn load_internal(view: &mut TextureLoadView, material: &StandardMaterial, special_textures_config: &SpecialTexturesConfig) -> Option<GenericMaterial> {
        // We save a teeny tiny bit of time by only cloning if we need to :)
        let mut material = material.clone();
        if let Some(exposure) = view.tb_config.lightmap_exposure {
            material.lightmap_exposure = exposure;
        }
    
        if view.name.starts_with('*') {
            // TODO i think this should be 
            let water_alpha: f32 = view.map.worldspawn()
                .and_then(|worldspawn| worldspawn.get("water_alpha").ok())
                .unwrap_or(1.);
    
            if water_alpha < 1. {
                material.alpha_mode = AlphaMode::Blend;
                material.base_color = Color::srgba(1., 1., 1., water_alpha);
            }
    
            let handle = view.add_material(LiquidMaterial {
                base: material,
                extension: (special_textures_config.default_liquid_material)(),
            });
            
            return Some(GenericMaterial {
                handle: handle.into(),
                properties: default(),
            });
        } else if view.name.starts_with("sky") {
            let Some(texture) = material.base_color_texture else { return None };
    
            let handle = view.add_material(QuakeSkyMaterial {
                texture,
                ..(special_textures_config.default_quake_sky_material)()
            });
    
            return Some(GenericMaterial {
                handle: handle.into(),
                properties: default(),
            });
        } else if view.name.starts_with('+') {
            let Some(embedded_textures) = view.embedded_textures else { return None };
            
            let mut chars = view.name.chars();
            chars.next();
    
            let Some(texture_frame_idx) = chars.next().and_then(|c| c.to_digit(10)) else { return None };
            let name_content = &view.name[2..];
            
            let mut frames = Vec::new();
            let mut frame_num = 0;
            while let Some(frame) = embedded_textures.get(format!("+{frame_num}{name_content}").as_str()) {
                frames.push(frame.clone());
                frame_num += 1;
            }
            
            let handle = view.add_material(material);
    
            let mut generic_material = GenericMaterial::new(handle);
    
            generic_material.set_property(GenericMaterial::ANIMATION, MaterialAnimations {
                next: None,
                images: Some(MaterialAnimation {
                    fps: special_textures_config.texture_animation_fps,
                    value: bevy::utils::HashMap::from([
                        ("base_color_texture".s(), frames),
                    ]),
                    state: GenericMaterialAnimationState {
                        current_frame: texture_frame_idx.wrapping_sub(1) as usize,
                        next_frame_time: Instant::now(),
                    },
                }),
            });
    
            return Some(generic_material);
        }
    
        None
    }
    
    load_internal(view, material, special_textures_config).map(|mut material| {
        if special_textures_config.invisible_textures.contains(view.name) {
            material.set_property(GenericMaterial::VISIBILITY, Visibility::Hidden);
        }
        
        material
    })
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