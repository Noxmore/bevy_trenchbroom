//! Handles the process of inserting a loaded level into the world.

// Currently, insertion uses too many file system calls for my liking.
// I've tried to only do cheep fs calls, or cache the results of said calls whenever i can, but in the future i would like all this to be asynchronous.

use std::{
    hash::{DefaultHasher, Hasher},
    time::Instant,
};

use bevy::{ecs::system::EntityCommands, render::render_resource::Face};

use crate::*;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum MapEntityInsertionError {
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

/// Gets sent whenever a map gets spawned in the world.
#[derive(Event, Debug, Clone)]
pub struct MapSpawnedEvent {
    pub entity: Entity,
}

/// When put in an entity with a `Handle<Map>`, this component will effect how the map spawns.
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct MapSpawningSettings {
    /// The unique identifier of this specific map entity. This is mainly used for networked games, so if you don't need it, [Uuid::nil] will work just fine.
    ///
    /// If [MapSpawningSettings] isn't a part of the map entity, it will also default to [Uuid::nil].
    pub uuid: Uuid,
}

pub fn spawn_maps(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut asset_events: EventReader<AssetEvent<Map>>,
    maps: Res<Assets<Map>>,
    tb_config: Res<TrenchBroomConfig>,
    added_query: Query<(Entity, &Handle<Map>, Option<&MapSpawningSettings>), Added<Handle<Map>>>,
    has_query: Query<(Entity, &Handle<Map>, Option<&MapSpawningSettings>)>,
    mut spawned_map_events: EventWriter<MapSpawnedEvent>,
) {
    // Stores the entities that have already been loaded to avoid spawning the map twice if it loads too fast
    let mut loaded_entities = Vec::new();

    // Spawn maps in pre-existing entities when it finishes loading
    for event in asset_events.read() {
        let AssetEvent::LoadedWithDependencies { id } = event else {
            continue;
        };

        for (entity, map_id, settings) in &has_query {
            if map_id.id() != *id {
                continue;
            }
            let Some(map) = maps.get(map_id) else {
                continue;
            };
            let uuid = settings
                .map(|settings| settings.uuid)
                .unwrap_or_else(Uuid::nil);

            map.insert(&mut commands, entity, &asset_server, &tb_config, uuid);
            spawned_map_events.send(MapSpawnedEvent { entity });
            loaded_entities.push(entity);
        }
    }

    // Spawn maps in newly-added entities
    for (entity, map_id, settings) in &added_query {
        let Some(map) = maps.get(map_id) else {
            continue;
        };
        if loaded_entities.contains(&entity) {
            continue;
        }
        let uuid = settings
            .map(|settings| settings.uuid)
            .unwrap_or_else(Uuid::nil);

        map.insert(&mut commands, entity, &asset_server, &tb_config, uuid);
        spawned_map_events.send(MapSpawnedEvent { entity });
    }
}

impl Map {
    /// Inserts this map into the Bevy world through the specified entity.
    ///
    /// Note: `uuid` is the map entity specific id used to make sure every entity's id is unique. This is mainly for networking, if you don't care, [Uuid::nil] works just fine.
    pub fn insert(
        &self,
        commands: &mut Commands,
        entity: Entity,
        asset_server: &AssetServer,
        tb_config: &TrenchBroomConfig,
        uuid: Uuid,
    ) {
        let start = Instant::now();
        // Just in case we are reloading the level
        commands.entity(entity).despawn_descendants();

        let mut hasher = DefaultHasher::new();
        hasher.write_u128(uuid.as_u128());
        let mut new_entities = Vec::new();

        for ent in &self.entities {
            hasher.write_usize(ent.ent_index);
            hasher.write_usize(ent.brushes.len());
            let high_bits = hasher.finish();
            hasher.write_usize(ent.properties.len());

            let bevy_ent = commands.spawn_empty().id();

            if let Err(err) = ent.insert(
                commands,
                bevy_ent,
                EntityInsertionView {
                    map_entity: entity,
                    properties: MapEntityPropertiesView {
                        entity: ent,
                        tb_config,
                    },
                    asset_server,
                    tb_config,
                    uuid: Uuid::from_u64_pair(high_bits, hasher.finish()),
                },
            ) {
                error!(
                    "[{}] Problem occurred while spawning map entity {}: {err}",
                    self.name, ent.ent_index
                );
            }
            new_entities.push(bevy_ent);
        }

        commands.entity(entity).push_children(&new_entities);

        info!(
            "Inserted map [{}] in {:.3}s",
            self.name,
            start.elapsed().as_secs_f32()
        );
    }
}

impl MapEntity {
    pub fn insert(
        &self,
        commands: &mut Commands,
        entity: Entity,
        view: EntityInsertionView,
    ) -> Result<(), MapEntityInsertionError> {
        self.insert_class(view.tb_config.get_definition(self.classname()?)?, commands, entity, view)?;

        if let Some(global_inserter) = view.tb_config.global_inserter {
            global_inserter(commands, entity, view)?;
        }

        Ok(())
    }

    fn insert_class(
        &self,
        definition: &EntityDefinition,
        commands: &mut Commands,
        entity: Entity,
        view: EntityInsertionView,
    ) -> Result<(), MapEntityInsertionError> {
        for base in &definition.base {
            self.insert_class(view.tb_config.get_definition(base)?, commands, entity, view)?;
        }

        if let Some(inserter) = definition.inserter {
            inserter(commands, entity, view)?;
        }

        Ok(())
    }
}

/// A function that inserts a map entity into a Bevy entity.
pub type EntityInserter = fn(
    commands: &mut Commands,
    entity: Entity,
    view: EntityInsertionView,
) -> Result<(), MapEntityInsertionError>;

/// Gives you access to important things when inserting an entity.
#[derive(Clone, Copy)]
pub struct EntityInsertionView<'w> {
    /// The entity with the `Handle<Map>` spawning this map entity.
    pub map_entity: Entity,
    pub properties: MapEntityPropertiesView<'w>,
    pub asset_server: &'w AssetServer,
    pub tb_config: &'w TrenchBroomConfig,
    /// A unique identifier for the entity being inserted.
    pub uuid: Uuid,
}

impl<'w> EntityInsertionView<'w> {
    /// Spawns all the brushes in this entity, and parents them to said entity, returning the list of entities that have been spawned.
    pub fn spawn_brushes(
        &self,
        commands: &mut Commands,
        entity: Entity,
        settings: BrushSpawnSettings,
    ) -> Vec<Entity> {
        let mut entities = Vec::new();
        let mut faces = Vec::new();

        for brush in &self.properties.entity.brushes {
            faces.push(brush.polygonize());
        }

        let brush_insertion_view = BrushInsertionView {
            entity_insertion_view: self,
            brushes: &faces,
        };
        for inserter in &settings.brush_inserters {
            entities.append(&mut inserter(commands, entity, &brush_insertion_view));
        }

        // Since bevy can only have 1 material per mesh, surfaces with the same material are grouped here,
        // each group will have its own mesh to reduce draw calls.
        let mut grouped_surfaces: HashMap<&str, Vec<&BrushSurfacePolygon>> = default();
        for face in faces.iter().flatten() {
            grouped_surfaces
                .entry(&face.surface.material)
                .or_insert_with(Vec::new)
                .push(face);
        }

        // Construct the meshes
        for (texture, faces) in grouped_surfaces {
            let mat_properties = MaterialProperties::load(
                self.tb_config
                    .assets_path
                    .join(format!("materials/{texture}.ron")),
            );
            if !mat_properties.kind.should_render() && !mat_properties.kind.should_collide() {
                continue;
            }

            let mut mesh = generate_mesh_from_brush_polygons(faces.as_slice(), self.tb_config);
            // TODO this makes pbr maps work, but messes up the lighting for me, why????
            if let Err(err) = mesh.generate_tangents() {
                error!("Couldn't generate tangents for brush in map entity {} with texture {texture}: {err}", self.properties.entity.ent_index);
            }

            let brush_mesh_insertion_view = BrushMeshInsertionView {
                entity_insertion_view: self,
                mesh: &mut mesh,
                texture,
                mat_properties: &mat_properties,
            };

            let mut ent = commands.spawn(Name::new(texture.to_string()));

            for inserter in &settings.mesh_inserters {
                inserter(&mut ent, &brush_mesh_insertion_view);
            }

            ent.insert(mat_properties);

            entities.push(ent.id());
        }

        commands.entity(entity).push_children(&entities);

        entities
    }
}

pub struct BrushMeshInsertionView<'w, 'l> {
    entity_insertion_view: &'l EntityInsertionView<'w>,
    pub mesh: &'l mut Mesh,
    pub texture: &'l str,
    pub mat_properties: &'l MaterialProperties,
}
impl<'w, 'l> std::ops::Deref for BrushMeshInsertionView<'w, 'l> {
    type Target = EntityInsertionView<'w>;
    fn deref(&self) -> &Self::Target {
        self.entity_insertion_view
    }
}
pub struct BrushInsertionView<'w, 'l> {
    entity_insertion_view: &'l EntityInsertionView<'w>,
    /// A Vec of computed brushes, each brush is a Vec of [BrushSurfacePolygon]s.
    pub brushes: &'l Vec<Vec<BrushSurfacePolygon>>,
}
impl<'w, 'l> std::ops::Deref for BrushInsertionView<'w, 'l> {
    type Target = EntityInsertionView<'w>;
    fn deref(&self) -> &Self::Target {
        self.entity_insertion_view
    }
}

/// A collection of inserters to call on each brush/mesh produced when spawning a brush.
#[derive(Default)]
pub struct BrushSpawnSettings {
    mesh_inserters: Vec<Box<dyn Fn(&mut EntityCommands, &BrushMeshInsertionView)>>,
    brush_inserters: Vec<Box<dyn Fn(&mut Commands, Entity, &BrushInsertionView) -> Vec<Entity>>>,
}

impl BrushSpawnSettings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calls the specified function on each mesh produced by the entity.
    pub fn mesh_inserter(
        mut self,
        inserter: impl Fn(&mut EntityCommands, &BrushMeshInsertionView) + 'static,
    ) -> Self {
        self.mesh_inserters.push(Box::new(inserter));
        self
    }

    /// Calls a function with an inserting entity, after said entity polygonizes it's brushes.
    pub fn brush_inserter(
        mut self,
        inserter: impl Fn(&mut Commands, Entity, &BrushInsertionView) -> Vec<Entity> + 'static,
    ) -> Self {
        self.brush_inserters.push(Box::new(inserter));
        self
    }

    /// Spawns child entities with meshes for each material used, loading said materials in the process.
    /// Will do nothing is your config is specified to be a server.
    pub fn draw_mesh(self) -> Self {
        self.mesh_inserter(|ent, view| {
            if !view.mat_properties.kind.should_render() || view.tb_config.is_server {
                return;
            }

            macro_rules! load_texture {
                ($name:ident = $map:literal) => {
                    let __texture_path = format!(
                        concat!("materials/{}", $map, ".{}"),
                        view.texture, view.tb_config.texture_extension
                    );
                    let $name: Option<Handle<Image>> =
                        if view.tb_config.assets_path.join(&__texture_path).exists() {
                            Some(view.asset_server.load(__texture_path))
                        } else {
                            None
                        };
                };
            }

            load_texture!(base_color_texture = "");
            load_texture!(normal_map_texture = "_normal");
            load_texture!(metallic_roughness_texture = "_mr");
            load_texture!(emissive_texture = "_emissive");
            load_texture!(depth_texture = "_depth");

            ent.insert(PbrBundle {
                mesh: view.asset_server.add(view.mesh.clone()),
                material: view.asset_server.add(StandardMaterial {
                    base_color_texture,
                    normal_map_texture,
                    metallic_roughness_texture,
                    emissive_texture,
                    depth_map: depth_texture,
                    perceptual_roughness: view.mat_properties.roughness,
                    metallic: view.mat_properties.metallic,
                    alpha_mode: view.mat_properties.alpha_mode.into(),
                    cull_mode: if view.mat_properties.double_sided {
                        None
                    } else {
                        Some(Face::Back)
                    },
                    ..default()
                }),
                ..default()
            });
        })
    }

    #[cfg(feature = "rapier")]
    /// Inserts trimesh colliders on each mesh this entity's brushes produce. This means that brushes will be hollow. Not recommended to use on physics objects.
    pub fn trimesh_collider(self) -> Self {
        self.mesh_inserter(|ent, view| {
            use bevy_rapier3d::prelude::*;
            ent.insert(
                bevy_rapier3d::geometry::Collider::from_bevy_mesh(
                    &view.mesh,
                    &ComputedColliderShape::TriMesh,
                )
                .unwrap(),
            );
        })
    }

    #[cfg(feature = "rapier")]
    /// Inserts a compound collider of every brush in this entity into said entity. This means that even faces with [MaterialKind::Empty] will still have collision, and brushes will be fully solid.
    pub fn convex_collider(self) -> Self {
        self.brush_inserter(|commands, entity, view| {
            use bevy_rapier3d::prelude::*;

            let mut colliders = Vec::new();

            for faces in view.brushes.iter() {
                let mesh = generate_mesh_from_brush_polygons(
                    &faces.iter().collect::<Vec<_>>(),
                    view.tb_config,
                );
                colliders.push((
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    Collider::from_bevy_mesh(&mesh, &ComputedColliderShape::ConvexHull).unwrap(),
                ));
            }

            commands
                .entity(entity)
                .insert(Collider::compound(colliders));
            Vec::new()
        })
    }
}
