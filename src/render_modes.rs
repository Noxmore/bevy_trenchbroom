//! Render modes allow you to change how a brush entity renders based on its properties.
//!
//! This comes from, and is meant for GoldSrc BSPs, but you can also you these in `.map` loading, or other BSPs if you wish.

use crate::{class::QuakeClassSpawnView, *};

/// Adds [GoldSrc render modes](https://developer.valvesoftware.com/wiki/Render_modes_(GoldSrc)) support.
#[derive(Default)]
pub struct GoldSrcRenderModesPlugin;
impl Plugin for GoldSrcRenderModesPlugin {
	fn build(&self, app: &mut App) {
		#[rustfmt::skip]
		app
			.register_type::<RenderModes>()
			.add_systems(Update, Self::apply_render_modes_on_standard_material)
		;
	}
}
impl GoldSrcRenderModesPlugin {
	// TODO: Non-standard materials? We would probably have to have a registry for that.
	pub fn apply_render_modes_on_standard_material(
		mut commands: Commands,
		material_query: Query<(Entity, &GenericMaterial3d, &RenderModes), Without<RenderModesApplied>>,
		mut generic_materials: ResMut<Assets<GenericMaterial>>,
		mut standard_materials: ResMut<Assets<StandardMaterial>>,
		// This is an optimization to make sure we don't have one material for each non-standard brush entity.
		mut material_map: Local<HashMap<(AssetId<GenericMaterial>, GoldSrcRenderMode), Handle<GenericMaterial>>>,
	) {
		for (entity, generic_material_3d, render_modes) in &material_query {
			let Some(render_mode) = render_modes.mode else {
				commands.entity(entity).insert(RenderModesApplied);
				continue;
			};

			// Check if we've already cached a modified material.
			if let Some(modified_material_handle) = material_map.get(&(generic_material_3d.id(), render_mode)) {
				commands
					.entity(entity)
					.insert(GenericMaterial3d(modified_material_handle.clone()))
					.insert(RenderModesApplied);
				continue;
			}

			let Some(generic_material) = generic_materials.get(&generic_material_3d.0) else { continue };
			let Ok(material_id) = generic_material.handle.id().try_typed::<StandardMaterial>() else { continue };
			let Some(mut modified_material) = standard_materials.get(material_id).cloned() else { continue };

			match render_mode {
				GoldSrcRenderMode::Normal => modified_material.alpha_mode = AlphaMode::Opaque,
				GoldSrcRenderMode::Color => modified_material.base_color = render_modes.color,
				GoldSrcRenderMode::Texture => {
					if render_modes.amt != u8::MAX {
						modified_material.alpha_mode = AlphaMode::Blend;
						modified_material.base_color.set_alpha(render_modes.amt as f32 / u8::MAX as f32);
					}
					modified_material.unlit = true;
				}
				GoldSrcRenderMode::Solid => {
					if render_modes.amt == 0 {
						modified_material.base_color.set_alpha(0.);
					}
					modified_material.alpha_mode = AlphaMode::Mask(0.5);
				}
				GoldSrcRenderMode::Additive | GoldSrcRenderMode::Glow => {
					modified_material.base_color.set_alpha(render_modes.amt as f32 / u8::MAX as f32);
					modified_material.alpha_mode = AlphaMode::Add;
				}
			}

			let modified_standard_material_handle = standard_materials.add(modified_material);
			let modified_material_handle = generic_materials.add(GenericMaterial::new(modified_standard_material_handle));

			material_map.insert((generic_material_3d.id(), render_mode), modified_material_handle.clone());

			commands
				.entity(entity)
				.insert(GenericMaterial3d(modified_material_handle))
				.insert(RenderModesApplied);
		}
	}
}

#[derive(Component)]
pub struct RenderModesApplied;

/// [GoldSrc render modes](https://developer.valvesoftware.com/wiki/Render_modes_(GoldSrc)). Changes how brush entities appear visually.
///
/// TODO: Currently only affects entities with `StandardMaterial`.
#[base_class(
	classname("__render_modes"),
	hooks((view.tb_config.default_solid_scene_hooks)().push(Self::apply_to_meshes))
)]
#[derive(Debug, Clone, Copy, SmartDefault)]
#[reflect(no_auto_register)] // This should only be available if the plugin has been added.
pub struct RenderModes {
	/// Changes how the brush entity appears visually. From GoldSrc.
	///
	/// If not set, uses default rendering.
	#[class(rename = "rendermode")]
	pub mode: Option<GoldSrcRenderMode>,
	/// Extra argument for `rendermode`.
	#[class(rename = "rendercolor", default = "255 255 255")]
	#[default(Color::WHITE)]
	pub color: Color,
	/// Extra argument for `rendermode`.
	#[class(rename = "renderamt")]
	#[default(255)]
	pub amt: u8,
}
impl RenderModes {
	/// Scene hook to make querying for meshes that should abide by render modes easier.
	pub fn apply_to_meshes(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		if let Some(file_type) = view.tb_config.limit_render_modes_to_file_type
			&& view.file_type != file_type
		{
			return Ok(());
		}

		let render_modes = view.world.entity(view.entity).get::<Self>().copied().unwrap();
		for mesh_view in view.meshes.iter() {
			view.world.entity_mut(mesh_view.entity).insert(render_modes);
		}

		Ok(())
	}
}

/// GoldSrc modes of rendering a brush entity.
#[derive(FgdType, Reflect, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[number_key]
#[reflect(no_auto_register)] // This should only be available if the plugin has been added.
pub enum GoldSrcRenderMode {
	/// ## [Valve Developer Community Documentation](https://developer.valvesoftware.com/wiki/Render_modes_(GoldSrc))
	///
	/// Brushes are lightmapped and opaque, regardless of if they have '{' textures. `renderamt` and `rendercolor` are unused outside of scrolling textures. `worldspawn` brushes always use this render mode.
	Normal = 0,
	/// ## [Valve Developer Community Documentation](https://developer.valvesoftware.com/wiki/Render_modes_(GoldSrc))
	///
	/// Allows sprites to be tinted with `rendercolor`. Same as texture on brushes and MDLs.
	Color = 1,
	/// ## [Valve Developer Community Documentation](https://developer.valvesoftware.com/wiki/Render_modes_(GoldSrc))
	///
	/// Enables translucency via `renderamt`, and alpha-tests { textures on brushes. Causes brushes to be fullbright.
	Texture = 2,
	/// ## [Valve Developer Community Documentation](https://developer.valvesoftware.com/wiki/Render_modes_(GoldSrc))
	///
	/// Only valid on sprites. Like additive, but renders the sprite at the same size regardless of distance from the camera.
	Glow = 3,
	/// ## [Valve Developer Community Documentation](https://developer.valvesoftware.com/wiki/Render_modes_(GoldSrc))
	///
	/// Same as `Normal`, but alpha-tests { textures on brushes. `renderamt` 0 (or absent) is treated as invisible, but all other values are full opacity for non-alphatested texels.
	#[default]
	Solid = 4,
	/// ## [Valve Developer Community Documentation](https://developer.valvesoftware.com/wiki/Render_modes_(GoldSrc))
	///
	/// Renders the sprite, brush, or MDL using additive translucency. `renderamt` can be used to reduce opacity.
	Additive = 5,
}
