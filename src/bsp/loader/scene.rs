use std::hash::{Hash as _, Hasher as _};

use super::*;
#[cfg(feature = "client")]
use crate::bsp::lighting::AnimatedLightingHandle;
use crate::{
	bsp::loader::textures::MaterialProperties,
	class::{QuakeClassMeshView, QuakeClassSpawnView, generate_class_map, spawn_quake_entity_into_scene},
	geometry::MapGeometry,
	util::MapFileType,
	*,
};
use bsp::*;
use models::InternalModel;

pub async fn initialize_scene(
	ctx: &mut BspLoadCtx<'_, '_>,
	models: &mut [InternalModel],
	embedded_textures: &mut EmbeddedTextures<'_>,
) -> anyhow::Result<World> {
	let config = &ctx.loader.tb_server.config;
	let class_map = generate_class_map(&ctx.type_registry.read());

	let mut world = World::new();

	// Spawn entities into scene
	for (map_entity_idx, map_entity) in ctx.entities.iter().enumerate() {
		let Some(classname) = map_entity.properties.get("classname") else { continue };
		let Some(class) = class_map.get(classname.as_str()).copied() else {
			if !config.suppress_invalid_entity_definitions {
				error!("No class found for classname `{classname}` on entity {map_entity_idx}");
			}

			continue;
		};

		let entity = world.spawn_empty().id();

		let mut meshes = Vec::new();

		if class.info.ty.is_solid()
			&& let Some(model_idx) = get_model_idx(map_entity, class)
		{
			let model = models.get_mut(model_idx).ok_or_else(|| anyhow!("invalid model index {model_idx}"))?;

			// Assign model entity
			if model.entity.is_some() {
				error!(
					"Map entity {map_entity_idx} ({}) points to model {model_idx}, but it has already been used by a different entity. Make an issue because i thought this wasn't possible!",
					class.info.name
				);
			}
			model.entity = Some(entity);

			meshes.reserve(model.meshes.len());

			for model_mesh in &mut model.meshes {
				if config.auto_remove_textures.contains(&model_mesh.texture.name) {
					continue;
				}

				let mesh_entity = world.spawn((Name::new(model_mesh.texture.name.clone()), Transform::default())).id();

				meshes.push(QuakeClassMeshView {
					entity: mesh_entity,
					mesh: &mut model_mesh.mesh,
					texture: &mut model_mesh.texture,
				});

				model_mesh.entity = Some(mesh_entity);
			}
		}

		// TODO: We probably don't want to hardcode this
		#[cfg(feature = "client")]
		for mesh_view in meshes.iter() {
			let Some(animated_lighting_handle) = &mesh_view.texture.lightmap else { continue };

			world
				.entity_mut(mesh_view.entity)
				.insert(AnimatedLightingHandle(animated_lighting_handle.clone()));
		}

		// We add the children at the end to prevent the console flooding with warnings about broken Transform and Visibility hierarchies.
		for mesh in &mut meshes {
			#[derive(Reflect, Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
			#[reflect(Hash)]
			#[reflect(PartialEq)]
			struct RenderModeTexture {
				alpha: u8,
				unlit: bool,
			}

			impl MaterialProperties for RenderModeTexture {
				fn write_material(&self, material: &mut StandardMaterial) -> anyhow::Result<()> {
					if self.alpha < u8::MAX {
						material.alpha_mode = AlphaMode::Premultiplied;
						material.base_color.set_alpha(self.alpha as f32 / 255.);
					}
					material.unlit = self.unlit;

					Ok(())
				}
			}

			#[derive(Reflect, Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
			#[reflect(Hash)]
			#[reflect(PartialEq)]
			struct RenderModeAdditive {
				alpha: u8,
			}

			impl MaterialProperties for RenderModeAdditive {
				fn write_material(&self, material: &mut StandardMaterial) -> anyhow::Result<()> {
					material.alpha_mode = AlphaMode::Add;
					material.base_color.set_alpha(self.alpha as f32 / 255.);
					material.unlit = true;

					Ok(())
				}
			}

			let name = mesh.texture.name.clone();

			let maybe_mat = match map_entity.properties.get("rendermode").map(|s| &**s) {
				Some(n @ "2") | Some(n @ "4") => {
					let mut alpha: u8 = map_entity.properties.get("renderamt").and_then(|s| s.parse().ok()).unwrap_or(255);

					if n == "4" && alpha != 0 {
						alpha = u8::MAX;
					}

					embedded_textures
						.material(&mut *ctx, &name, RenderModeTexture { alpha, unlit: n == "2" })
						.await
				}
				Some("5") => {
					let alpha: u8 = map_entity.properties.get("renderamt").and_then(|s| s.parse().ok()).unwrap_or(255);

					embedded_textures.material(&mut *ctx, &name, RenderModeAdditive { alpha }).await
				}
				_ => embedded_textures.material(&mut *ctx, &name, NoMaterialProperties).await,
			};

			let material = match maybe_mat {
				Some(embedded_texture) => embedded_texture.clone(),
				None => (config.load_loose_texture)(TextureLoadView {
					name: &name,
					tb_config: config,
					load_context: ctx.load_context,
					asset_server: ctx.asset_server,
					entities: ctx.entities,
					#[cfg(feature = "client")]
					alpha_mode: None,
					embedded_textures: Some(&embedded_textures.images),
				})
				.await
				.clone(),
			};

			mesh.texture.material = Some(material);

			world.entity_mut(mesh.entity).insert((ChildOf(entity), MapGeometry));
		}

		spawn_quake_entity_into_scene(&mut QuakeClassSpawnView {
			file_type: MapFileType::Bsp,
			tb_config: config,
			src_entity: map_entity,
			src_entity_idx: map_entity_idx,
			type_registry: &ctx.type_registry.read(),
			class_map: &class_map,
			class,
			world: &mut world,
			entity,
			load_context: ctx.load_context,
			meshes: &mut meshes,
		})
		.map_err(|err| anyhow!("spawning entity {map_entity_idx} ({classname}): {err}"))?;
	}

	Ok(world)
}
