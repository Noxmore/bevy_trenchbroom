use super::*;
#[cfg(feature = "client")]
use crate::bsp::lighting::AnimatedLightingHandle;
use crate::{
	class::{
		QuakeClassMeshView, QuakeClassSpawnView, generate_class_map,
		scene_systems::{LoadContextRes, PostSpawn, SceneLoadContext},
		spawn_quake_entity_into_scene,
	},
	geometry::MapGeometry,
	util::MapFileType,
	*,
};
use bsp::*;
use models::InternalModel;

pub fn initialize_scene(ctx: &mut BspLoadCtx, models: &mut [InternalModel]) -> anyhow::Result<World> {
	let config = &ctx.loader.tb_server.config;
	let type_registry = ctx.type_registry.read();
	let class_map = generate_class_map(&type_registry);

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
				let mut name = Cow::Borrowed("<TEXTURE MISSING>");

				if let Some(texture_name) = &model_mesh.texture.name {
					if config.auto_remove_textures.contains(texture_name) {
						continue;
					}

					name = Cow::Owned(texture_name.clone());
				}

				let mesh_entity = world.spawn((Name::new(name), Transform::default())).id();

				meshes.push(QuakeClassMeshView {
					entity: mesh_entity,
					mesh: &mut model_mesh.mesh,
					texture: &mut model_mesh.texture,
				});

				model_mesh.entity = Some(mesh_entity);
			}
		}

		let mut view = QuakeClassSpawnView {
			file_type: MapFileType::Bsp,
			tb_config: config,
			src_entity: map_entity,
			src_entity_idx: map_entity_idx,
			type_registry: &type_registry,
			class_map: &class_map,
			class,
			world: &mut world,
			entity,
			load_context: ctx.load_context,
			meshes: &mut meshes,
		};

		// TODO: We probably don't want to hardcode this
		#[cfg(feature = "client")]
		for mesh_view in view.meshes.iter() {
			let Some(animated_lighting_handle) = &mesh_view.texture.lightmap else { continue };

			view.world
				.entity_mut(mesh_view.entity)
				.insert(AnimatedLightingHandle(animated_lighting_handle.clone()));
		}

		spawn_quake_entity_into_scene(&mut view).map_err(|err| anyhow!("spawning entity {map_entity_idx} ({classname}): {err}"))?;

		// We add the children at the end to prevent the console flooding with warnings about broken Transform and Visibility hierarchies.
		for mesh_view in view.meshes.iter() {
			view.world.entity_mut(mesh_view.entity).insert((ChildOf(entity), MapGeometry));
		}
	}

	// world.insert_non_send_resource(unsafe { LoadContextRes::new(ctx.load_context) });

	/* let scene_schedules = ctx.loader.scene_schedules.schedules.read();
	if let Some(schedule) = scene_schedules.get(PostSpawn) {
		let systems = schedule.graph().topsort_graph(schedule.graph().hierarchy().graph(), ReportCycles::Hierarchy).unwrap();
		for node_id in systems {
			match node_id {
				NodeId::System(system_key) => {
					let system = schedule.graph().systems.get(system_key).unwrap();
					world.register_boxed_system::<SQuakeSpawnView, ()>(system.system);
				}
				NodeId::Set(set_key) => {

				}
			}
			// schedule.graph().system_sets.
			// world.register_system(system)
			// world.run_system_with(id, input);
		}
	} */

	Ok(world)
}
