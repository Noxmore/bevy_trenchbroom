//! Handles the process of spawning a loaded [Map] and [MapEntities](MapEntity) into the world.

// Currently, spawning uses too many file system calls for my liking.
// I've tried to only do cheep fs calls, or cache the results of said calls whenever i can, but in the future i would like all this to be asynchronous.

use bevy::ecs::world::Command;

use crate::*;

pub mod geometry;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum MapEntitySpawnError {
    #[error("requires property `{property}` to be created")]
    RequiredPropertyNotFound { property: String },
    #[error("requires property `{property}` to be a valid `{required_type}`. Error: {error}")]
    PropertyParseError {
        property: String,
        required_type: &'static str,
        error: String,
    },
    #[error("definition for \"{classname}\" not found")]
    DefinitionNotFound { classname: String },
    #[error("Entity class {classname} has a base of {base_name}, but that class does not exist")]
    InvalidBase {
        classname: String,
        base_name: String,
    },
}

/// Marker component that specifies that the [Handle<Map>] of the entity it's on has been loaded spawned. Remove to respawn the map.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SpawnedMap;

pub fn spawn_maps(world: &mut World) {
    // Spawn maps
    world.resource_scope(|world, maps: Mut<Assets<Map>>| {
        for (entity, map_handle) in world
            .query_filtered::<(Entity, &Handle<Map>), Without<SpawnedMap>>()
            .iter(world)
            .map(|(e, h)| (e, h.clone()))
            .collect_vec()
        {
            let Some(map) = maps.get(&map_handle) else {
                continue;
            };

            // Lets make sure we don't spawn a map every frame
            world.entity_mut(entity).insert(SpawnedMap);

            map.spawn(world, entity);
        }
    });

    let server = world.resource::<TrenchBroomServer>().clone();
    for (entity, map_entity) in world
        .query_filtered::<(Entity, &MapEntity), Without<SpawnedMapEntity>>()
        .iter(world)
        // I'd really rather not clone this, but the borrow checker has forced my hand
        .map(|(e, h)| (e, h.clone()))
        .collect_vec()
    {
        DespawnChildrenRecursive { entity }.apply(world);

        world.entity_mut(entity).insert(SpawnedMapEntity);

        if let Err(err) = MapEntity::spawn(
            world,
            entity,
            EntitySpawnView {
                map_entity: &map_entity,
                server: &server,
            },
        ) {
            error!(
                "Problem occurred while spawning MapEntity {entity} (index {:?}): {err}",
                map_entity.ent_index
            );
        }
    }
}

pub fn reload_maps(
    mut commands: Commands,
    mut asset_events: EventReader<AssetEvent<Map>>,
    spawned_map_query: Query<(Entity, &Handle<Map>), With<SpawnedMap>>,
) {
    for event in asset_events.read() {
        let AssetEvent::Modified { id } = event else {
            continue;
        };

        for (entity, map_handle) in &spawned_map_query {
            if &map_handle.id() == id {
                commands.entity(entity).remove::<SpawnedMap>();
            }
        }
    }
}

impl Map {
    /// Spawns this map into the Bevy world through the specified entity. The map will not be fully spawned until [spawn_maps] has ran.
    pub fn spawn(&self, world: &mut World, entity: Entity) {
        // Just in case we are reloading the level
        DespawnChildrenRecursive { entity }.apply(world);

        // Add skeleton entities as children of the Map entity, if this is being called from spawn_maps, they'll be spawned later this update
        let skeleton_entities = self
            .entities
            .iter()
            .cloned()
            .map(|map_entity| world.spawn(map_entity).id())
            .collect_vec();

        world.entity_mut(entity).push_children(&skeleton_entities);
    }
}

impl MapEntity {
    /// Spawns this MapEntity into the Bevy world.
    pub fn spawn(
        world: &mut World,
        entity: Entity,
        view: EntitySpawnView,
    ) -> Result<(), MapEntitySpawnError> {
        Self::spawn_class(
            view.server.config
                .get_definition(view.map_entity.classname()?)?,
            world,
            entity,
            view,
        )?;

        if let Some(global_spawner) = view.server.config.global_spawner {
            global_spawner(world, entity, view)?;
        }

        Ok(())
    }

    fn spawn_class(
        definition: &EntityDefinition,
        world: &mut World,
        entity: Entity,
        view: EntitySpawnView,
    ) -> Result<(), MapEntitySpawnError> {
        for base in &definition.base {
            Self::spawn_class(view.server.config.get_definition(base)?, world, entity, view)?;
        }

        if let Some(spawner) = definition.spawner {
            spawner(world, entity, view)?;
        }

        Ok(())
    }
}

/// A function that spawns a MapEntity into a Bevy entity.
pub type EntitySpawner = fn(
    commands: &mut World,
    entity: Entity,
    view: EntitySpawnView,
) -> Result<(), MapEntitySpawnError>;

/// Gives you access to important things when spawning a [MapEntity].
#[derive(Clone, Copy)]
pub struct EntitySpawnView<'w> {
    pub map_entity: &'w MapEntity,
    pub server: &'w TrenchBroomServer,
}

impl<'w> EntitySpawnView<'w> {
    /// Gets a property from this entity accounting for entity class hierarchy.
    /// If the property is not defined, it attempts to get its default.
    pub fn get<T: TrenchBroomValue>(&self, key: &str) -> Result<T, MapEntitySpawnError> {
        let Some(value_str) = self.map_entity.properties.get(key).or(self
            .server.config
            .get_entity_property_default(self.map_entity.classname()?, key))
        else {
            return Err(MapEntitySpawnError::RequiredPropertyNotFound {
                property: key.into(),
            });
        };
        T::tb_parse(value_str.trim_matches('"')).map_err(|err| {
            MapEntitySpawnError::PropertyParseError {
                property: key.into(),
                required_type: std::any::type_name::<T>(),
                error: err.to_string(),
            }
        })
    }

    /// Extracts a transform from this entity using the properties `angles`, `origin`, and `scale`.
    /// If you are not using those for your transform, you probably shouldn't use this function.
    pub fn get_transform(&self) -> Transform {
        let rotation = match self.get::<Vec3>("angles") {
            Ok(rot) => Quat::from_euler(
                // Honestly, i don't know why this works, i got here through hours of trial and error
                EulerRot::default(),
                (rot.y - 90.).to_radians(),
                -rot.x.to_radians(),
                -rot.z.to_radians(),
            ),
            Err(_) => Quat::default(),
        };

        Transform {
            translation: self.server.config.to_bevy_space(self
                .get::<Vec3>("origin")
                .unwrap_or(Vec3::ZERO)),
            rotation,
            scale: match self.get::<f32>("scale") {
                Ok(scale) => Vec3::splat(scale),
                Err(_) => match self.get::<Vec3>("scale") {
                    Ok(scale) => self.server.config.to_bevy_space(scale),
                    Err(_) => Vec3::ONE,
                },
            },
        }
    }
}
