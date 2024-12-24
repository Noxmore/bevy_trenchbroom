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

/// The component for spawning map entities.
/// Any entity with this component and without [SpawnedMapEntity] will have the containing map entity spawned into the Bevy world.
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct MapEntityRef {
    pub map_entity: Arc<MapEntity>,
    pub map_handle: Option<Handle<Map>>,
}

/// Marker component for a [MapEntity] that has been spawned, to respawn a [MapEntity], remove this component.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SpawnedMapEntity;

impl TrenchBroomPlugin {
    /// Spawns maps and map entities
    pub fn spawn_maps(world: &mut World) {
        // Spawn maps
        world.resource_scope(|world, maps: Mut<Assets<Map>>| {
            let stuff = world
                .query_filtered::<(Entity, &MapHandle), Without<SpawnedMap>>()
                .iter(world)
                .map(|(e, h)| (e, h.clone()))
                .collect_vec();
            for (entity, map_handle) in stuff {
                let Some(map) = maps.get(&map_handle.0) else {
                    continue;
                };

                // Lets make sure we don't spawn a map every frame
                world.entity_mut(entity).insert(SpawnedMap);

                for (irradiance_volume, transform) in &map.irradiance_volumes {
                    world
                        .spawn(Visibility::default())
                        .insert(irradiance_volume.clone())
                        .insert(LightProbe)
                        .insert(*transform);
                }

                map.spawn(world, entity);
            }
        });

        // Spawn map entities
        let server = world.resource::<TrenchBroomServer>().clone();
        for (entity, map_entity_ref) in world
            .query_filtered::<(Entity, &MapEntityRef), Without<SpawnedMapEntity>>()
            .iter(world)
            // I'd really rather not clone this, but the borrow checker has forced my hand
            .map(|(e, h)| (e, h.clone()))
            .collect_vec()
        {
            DespawnChildrenRecursive {
                entity,
                warn: false,
            }
            .apply(world);

            world.entity_mut(entity).insert(SpawnedMapEntity);

            if let Err(err) = MapEntity::spawn(
                world,
                entity,
                EntitySpawnView {
                    map_entity_ref: &map_entity_ref,
                    server: &server,
                },
            ) {
                if matches!(
                    err,
                    MapEntitySpawnError::DefinitionNotFound { classname: _ }
                ) && server.config.ignore_invalid_entity_definitions
                {
                    continue;
                }

                error!(
                    "Problem occurred while spawning MapEntity {entity} (index {:?}): {err}",
                    map_entity_ref.map_entity.ent_index
                );
            }
        }
    }

    /// Map hot-reloading
    pub fn reload_maps(
        mut commands: Commands,
        mut asset_events: EventReader<AssetEvent<Map>>,
        spawned_map_query: Query<(Entity, &MapHandle), With<SpawnedMap>>,
    ) {
        for event in asset_events.read() {
            let AssetEvent::Modified { id } = event else {
                continue;
            };

            for (entity, map_handle) in &spawned_map_query {
                if &map_handle.0.id() == id {
                    commands.entity(entity).remove::<SpawnedMap>();
                }
            }
        }
    }
}

impl Map {
    /// Spawns this map into the Bevy world through the specified entity. The map will not be fully spawned until [spawn_maps] has ran.
    pub fn spawn(&self, world: &mut World, entity: Entity) {
        // Just in case we are reloading the level
        DespawnChildrenRecursive {
            entity,
            warn: false,
        }
        .apply(world);

        let map_handle = world.entity(entity).get::<MapHandle>().cloned();

        // Add skeleton entities as children of the Map entity, if this is being called from spawn_maps, they'll be spawned later this update
        let skeleton_entities = self
            .entities
            .iter()
            .cloned()
            .map(|map_entity| {
                world
                    .spawn(MapEntityRef {
                        map_entity,
                        map_handle: map_handle.as_ref().map(|h| h.0.clone()),
                    })
                    .id()
            })
            .collect_vec();

        world.entity_mut(entity).add_children(&skeleton_entities);
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
            view.server
                .config
                .get_definition(view.map_entity.classname()?)?,
            world,
            entity,
            view,
        )?;

        for global_spawner in &view.server.config.global_spawners {
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
            Self::spawn_class(
                view.server.config.get_definition(base)?,
                world,
                entity,
                view,
            )?;
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
    map_entity_ref: &'w MapEntityRef,
    pub server: &'w TrenchBroomServer,
}
impl std::ops::Deref for EntitySpawnView<'_> {
    type Target = MapEntityRef;
    fn deref(&self) -> &Self::Target {
        self.map_entity_ref
    }
}

impl<'w> EntitySpawnView<'w> {
    /// Gets a property from this entity accounting for entity class hierarchy.
    /// If the property is not defined, it attempts to get its default.
    pub fn get<T: FgdType>(&self, key: &str) -> Result<T, MapEntitySpawnError> {
        let Some(value_str) = self.map_entity.properties.get(key).or(self
            .server
            .config
            .get_entity_property_default(self.map_entity.classname()?, key))
        else {
            return Err(MapEntitySpawnError::RequiredPropertyNotFound {
                property: key.into(),
            });
        };
        T::fgd_parse(value_str.trim_matches('"')).map_err(|err| {
            MapEntitySpawnError::PropertyParseError {
                property: key.into(),
                required_type: std::any::type_name::<T>(),
                error: err.to_string(),
            }
        })
    }

    /// Extracts a transform from this entity using the properties `angles`, `mangle`, or `angle` for rotation, `origin` for translation, and `scale` for scale.
    /// If you are not using those for your transform, you probably shouldn't use this function.
    pub fn get_transform(&self) -> Transform {
        let rotation = self
            .get::<Vec3>("angles")
            .map(angles_to_quat)
            .or_else(|_| {
                self.get::<Vec3>("mangle")
                    // According to TrenchBroom docs https://trenchbroom.github.io/manual/latest/#editing-objects
                    // “mangle” is interpreted as “yaw pitch roll” if the entity classnames begins with “light”, otherwise it’s a synonym for “angles”
                    .map(
                        if self.map_entity.classname().map(|s| s.starts_with("light")) == Ok(true) {
                            mangle_to_quat
                        } else {
                            angles_to_quat
                        },
                    )
            })
            .or_else(|_| self.get::<f32>("angle").map(angle_to_quat))
            .unwrap_or_default();

        Transform {
            translation: self
                .server
                .config
                .to_bevy_space(self.get::<Vec3>("origin").unwrap_or(Vec3::ZERO)),
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
