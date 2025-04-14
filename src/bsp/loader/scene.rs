use super::*;
use crate::*;
use bsp::*;
use models::InternalModel;

pub fn initialize_scene(ctx: &mut BspLoadCtx, models: &mut [InternalModel]) -> anyhow::Result<World> {
	let config = &ctx.loader.tb_server.config;

	let mut world = World::new();

	// Spawn entities into scene
	for (map_entity_idx, map_entity) in ctx.entities.iter().enumerate() {
		let Some(classname) = map_entity.properties.get("classname") else { continue };
		let Some(class) = config.get_class(classname) else {
			if !config.suppress_invalid_entity_definitions {
				error!("No class found for classname `{classname}` on entity {map_entity_idx}");
			}

			continue;
		};

		let mut entity = world.spawn_empty();
		let entity_id = entity.id();

		class
			.apply_spawn_fn_recursive(config, map_entity, &mut entity)
			.map_err(|err| anyhow!("spawning entity {map_entity_idx} ({classname}): {err}"))?;

		if let QuakeClassType::Solid(geometry_provider) = class.info.ty {
			let geometry_provider = geometry_provider();

			if let Some(model_idx) = get_model_idx(map_entity, class) {
				let model = models.get_mut(model_idx).ok_or_else(|| anyhow!("invalid model index {model_idx}"))?;

				// Assign model entity
				if model.entity.is_some() {
					error!("Map entity {map_entity_idx} ({}) points to model {model_idx}, but it has already been used by a different entity. Make an issue because i thought this wasn't possible!", class.info.name);
				}
				model.entity = Some(entity_id);

				let mut meshes = Vec::with_capacity(model.meshes.len());

				for model_mesh in &mut model.meshes {
					if config.auto_remove_textures.contains(&model_mesh.texture.name) {
						continue;
					}

					let mesh_entity = world.spawn(Name::new(model_mesh.texture.name.clone())).id();

					meshes.push(GeometryProviderMeshView {
						entity: mesh_entity,
						mesh: &mut model_mesh.mesh,
						texture: &mut model_mesh.texture,
					});

					model_mesh.entity = Some(mesh_entity);
				}

				let mut view = GeometryProviderView {
					world: &mut world,
					entity: entity_id,
					tb_server: &ctx.loader.tb_server,
					map_entity,
					map_entity_idx,
					meshes,
				};

				for provider in geometry_provider.providers {
					provider(&mut view);
				}

				(config.global_geometry_provider)(&mut view);

				// We add the children at the end to prevent the console flooding with warnings about broken Transform and Visibility hierarchies.
				for mesh_view in view.meshes {
					world.entity_mut(entity_id).add_child(mesh_view.entity);
				}
			}
		}

		let mut entity = world.entity_mut(entity_id);

		(config.global_spawner)(config, map_entity, &mut entity)
			.map_err(|err| anyhow!("spawning entity {map_entity_idx} ({classname}) with global spawner: {err}"))?;
	}

	Ok(world)
}
