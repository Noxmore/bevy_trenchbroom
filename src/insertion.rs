//! Handles the process of inserting a loaded level into the world.

// Currently, insertion uses too many file system calls for my liking.
// I've tried to only do cheep fs calls, or cache the results of said calls whenever i can, but in the future i would like all this to be asynchronous.

use std::{
    hash::{DefaultHasher, Hasher},
    time::Instant,
};

use bevy::{ecs::system::Command, render::render_resource::Face};

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

/// Marker component that specifies that the [Handle<Map>] of the entity it's on has been loaded spawned. Remove to respawn the map.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SpawnedMap;

/// When put in an entity with a `Handle<Map>`, this component will effect how the map spawns.
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct MapSpawningSettings {
    /// The unique identifier of this specific map entity. This is mainly used for networked games, so if you don't need it, [Uuid::nil] will work just fine.
    ///
    /// If [MapSpawningSettings] isn't a part of the map entity, it will also default to [Uuid::nil].
    pub uuid: Uuid,
}

pub fn spawn_maps(world: &mut World) {
    world.resource_scope(|world, maps: Mut<Assets<Map>>| {
        // Borrow checker thingy
        let mut insertion_params = None;

        'map_loop: for (entity, map_handle, settings) in world.query_filtered::<(Entity, &Handle<Map>, Option<&MapSpawningSettings>), Without<SpawnedMap>>().iter(world) {
            let Some(map) = maps.get(map_handle) else { continue };

            // If some material properties hasn't finished loading, don't insert yet
            for (_, mat_properties_handle) in &map.material_properties_map {
                if !world.resource::<Assets<MaterialProperties>>().contains(mat_properties_handle) {
                    continue 'map_loop;
                }
            }

            let uuid = settings
                .map(|settings| settings.uuid)
                .unwrap_or_else(Uuid::nil);

            insertion_params = Some((entity, map_handle.clone(), uuid));
        }

        if let Some((entity, map_handle, uuid)) = insertion_params {
            // Lets make sure we don't spawn a map every frame
            world.entity_mut(entity).insert(SpawnedMap);

            maps.get(&map_handle).unwrap().insert(world, entity, uuid);
        }
    });
}

impl Map {
    /// Inserts this map into the Bevy world through the specified entity.
    ///
    /// Note: `uuid` is the map entity specific id used to make sure every entity's id is unique. This is mainly for networking, if you don't care, [Uuid::nil] works just fine.
    pub fn insert(
        &self,
        world: &mut World,
        entity: Entity,
        uuid: Uuid,
    ) {
        let start = Instant::now();
        // Just in case we are reloading the level
        DespawnChildrenRecursive { entity }.apply(world);

        let mut hasher = DefaultHasher::new();
        hasher.write_u128(uuid.as_u128());
        let mut new_entities = Vec::new();

        world.resource_scope(|world, tb_config: Mut<TrenchBroomConfig>| {
            for map_entity in &self.entities {
                hasher.write_usize(map_entity.ent_index);
                hasher.write_usize(map_entity.brushes.len());
                let high_bits = hasher.finish();
                hasher.write_usize(map_entity.properties.len());
    
                let bevy_ent = world.spawn_empty().id();
    
                if let Err(err) = map_entity.insert(
                    world,
                    bevy_ent,
                    EntityInsertionView {
                        map: self,
                        entity,
                        map_entity,
                        tb_config: &tb_config,
                        uuid: Uuid::from_u64_pair(high_bits, hasher.finish()),
                    },
                ) {
                    error!(
                        "[{}] Problem occurred while spawning map entity {}: {err}",
                        self.name, map_entity.ent_index
                    );
                }
                new_entities.push(bevy_ent);
            }
        });


        world.entity_mut(entity).push_children(&new_entities);

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
        world: &mut World,
        entity: Entity,
        view: EntityInsertionView,
    ) -> Result<(), MapEntityInsertionError> {
        self.insert_class(view.tb_config.get_definition(self.classname()?)?, world, entity, view)?;

        if let Some(global_inserter) = view.tb_config.global_inserter {
            global_inserter(world, entity, view)?;
        }

        Ok(())
    }

    fn insert_class(
        &self,
        definition: &EntityDefinition,
        world: &mut World,
        entity: Entity,
        view: EntityInsertionView,
    ) -> Result<(), MapEntityInsertionError> {
        for base in &definition.base {
            self.insert_class(view.tb_config.get_definition(base)?, world, entity, view)?;
        }

        if let Some(inserter) = definition.inserter {
            inserter(world, entity, view)?;
        }

        Ok(())
    }
}

/// A function that inserts a map entity into a Bevy entity.
pub type EntityInserter = fn(
    commands: &mut World,
    entity: Entity,
    view: EntityInsertionView,
) -> Result<(), MapEntityInsertionError>;

/// Gives you access to important things when inserting an entity.
#[derive(Clone, Copy)]
pub struct EntityInsertionView<'w> {
    pub map: &'w Map,
    /// The entity with the `Handle<Map>` spawning this map entity.
    pub entity: Entity,
    pub map_entity: &'w MapEntity,
    pub tb_config: &'w TrenchBroomConfig,
    /// A unique identifier for the entity being inserted.
    pub uuid: Uuid,
}

impl<'w> EntityInsertionView<'w> {
    /// Gets a property from this entity accounting for entity class hierarchy.
    /// If the property is not defined, it attempts to get its default.
    pub fn get<T: TrenchBroomValue>(&self, key: &str) -> Result<T, MapEntityInsertionError> {
        let Some(value_str) = self.map_entity.properties.get(key).or(self
            .tb_config
            .get_entity_property_default(self.map_entity.classname()?, key))
        else {
            return Err(MapEntityInsertionError::RequiredPropertyNotFound {
                property: key.into(),
            });
        };
        T::tb_parse(value_str.trim_matches('"')).map_err(|err| {
            MapEntityInsertionError::PropertyParseError {
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
            translation: self
                .get::<Vec3>("origin")
                .unwrap_or(Vec3::ZERO)
                .trenchbroom_to_bevy_space(),
            rotation,
            scale: match self.get::<f32>("scale") {
                Ok(scale) => Vec3::splat(scale),
                Err(_) => match self.get::<Vec3>("scale") {
                    Ok(scale) => scale.trenchbroom_to_bevy_space(),
                    Err(_) => Vec3::ONE,
                },
            },
        }
    }

    /// Spawns all the brushes in this entity, and parents them to said entity, returning the list of entities that have been spawned.
    pub fn spawn_brushes(
        &self,
        world: &mut World,
        entity: Entity,
        settings: BrushSpawnSettings,
    ) -> Vec<Entity> {
        let mut entities = Vec::new();
        let mut faces = Vec::new();

        for brush in &self.map_entity.brushes {
            faces.push(brush.polygonize());
        }

        let brush_insertion_view = BrushInsertionView {
            entity_insertion_view: self,
            brushes: &faces,
        };
        for inserter in &settings.brush_inserters {
            entities.append(&mut inserter(world, entity, &brush_insertion_view));
        }

        // Since bevy can only have 1 material per mesh, surfaces with the same material are grouped here,
        // each group will have its own mesh to reduce the number of entities.
        let mut grouped_surfaces: HashMap<&str, Vec<&BrushSurfacePolygon>> = default();
        for face in faces.iter().flatten() {
            grouped_surfaces
                .entry(&face.surface.texture)
                .or_insert_with(Vec::new)
                .push(face);
        }

        // We need to pass the texture's material properties to the view
        world.resource_scope(|world, mat_properties_assets: Mut<Assets<MaterialProperties>>| {
            // Construct the meshes
            for (texture, faces) in grouped_surfaces {
                // I'd rather not clone here, oh well!
                let mat_properties = self.map.material_properties_map.get(texture)
                    .map(|handle| mat_properties_assets.get(handle).expect(&format!("Could not find material properties for {texture} but path exists, did you remove the asset?")))
                    .cloned().unwrap_or_default();

                let mut mesh = generate_mesh_from_brush_polygons(faces.as_slice(), self.tb_config);
                // TODO this makes pbr maps work, but messes up the lighting for me, why????
                if let Err(err) = mesh.generate_tangents() {
                    error!("Couldn't generate tangents for brush in map entity {} with texture {texture}: {err}", self.map_entity.ent_index);
                }

                let brush_mesh_insertion_view = BrushMeshInsertionView {
                    entity_insertion_view: self,
                    mesh: &mut mesh,
                    texture,
                    mat_properties,
                };

                let mut ent = world.spawn(Name::new(texture.to_string()));

                for inserter in &settings.mesh_inserters {
                    inserter(&mut ent, &brush_mesh_insertion_view);
                }

                entities.push(ent.id());
            }
        });

        world.entity_mut(entity).push_children(&entities);

        entities
    }
}

pub struct BrushMeshInsertionView<'w, 'l> {
    entity_insertion_view: &'l EntityInsertionView<'w>,
    pub mesh: &'l mut Mesh,
    pub texture: &'l str,
    pub mat_properties: MaterialProperties,
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
    mesh_inserters: Vec<Box<dyn Fn(&mut EntityWorldMut, &BrushMeshInsertionView)>>,
    brush_inserters: Vec<Box<dyn Fn(&mut World, Entity, &BrushInsertionView) -> Vec<Entity>>>,
}

impl BrushSpawnSettings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calls the specified function on each mesh produced by the entity.
    pub fn mesh_inserter(
        mut self,
        inserter: impl Fn(&mut EntityWorldMut, &BrushMeshInsertionView) + 'static,
    ) -> Self {
        self.mesh_inserters.push(Box::new(inserter));
        self
    }

    /// Calls a function with an inserting entity, after said entity polygonizes it's brushes.
    pub fn brush_inserter(
        mut self,
        inserter: impl Fn(&mut World, Entity, &BrushInsertionView) -> Vec<Entity> + 'static,
    ) -> Self {
        self.brush_inserters.push(Box::new(inserter));
        self
    }

    /// Spawns child entities with meshes per each material used, loading said materials in the process.
    /// Will do nothing is your config is specified to be a server.
    pub fn pbr_mesh(self) -> Self {
        self.mesh_inserter(|ent, view| {
            if view.tb_config.is_server {
                return;
            }

            let asset_server = ent.world().resource::<AssetServer>();

            if !view.mat_properties.get(MaterialProperties::RENDER) {
                return;
            }

            let material = BRUSH_TEXTURE_TO_MATERIALS.lock().unwrap().entry(view.texture.into()).or_insert_with(|| {
                macro_rules! load_texture {
                    ($name:ident = $map:literal) => {
                        let __texture_path = format!(
                            concat!("{}/{}", $map, ".{}"),
                            view.tb_config.texture_root.display(), view.texture, view.tb_config.texture_extension
                        );
                        let $name: Option<Handle<Image>> =
                            if view.tb_config.assets_path.join(&__texture_path).exists() {
                                Some(asset_server.load(__texture_path))
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

                asset_server.add(StandardMaterial {
                    base_color_texture,
                    normal_map_texture,
                    metallic_roughness_texture,
                    emissive_texture,
                    depth_map: depth_texture,
                    perceptual_roughness: view.mat_properties.get(MaterialProperties::ROUGHNESS),
                    metallic: view.mat_properties.get(MaterialProperties::METALLIC),
                    alpha_mode: view.mat_properties.get(MaterialProperties::ALPHA_MODE).into(),
                    emissive: view.mat_properties.get(MaterialProperties::EMISSIVE),
                    cull_mode: if view.mat_properties.get(MaterialProperties::DOUBLE_SIDED) {
                        None
                    } else {
                        Some(Face::Back)
                    },
                    ..default()
                })
            }).clone();

            ent.insert(PbrBundle {
                mesh: asset_server.add(view.mesh.clone()),
                material,
                ..default()
            });
        })
    }

    #[cfg(feature = "rapier")]
    /// Inserts trimesh colliders on each mesh this entity's brushes produce. This means that brushes will be hollow. Not recommended to use on physics objects.
    pub fn trimesh_collider(self) -> Self {
        self.mesh_inserter(|ent, view| {
            if !view.mat_properties.get(MaterialProperties::COLLIDE) { return }

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
        self.brush_inserter(|world, entity, view| {
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

            world
                .entity_mut(entity)
                .insert(Collider::compound(colliders));
            Vec::new()
        })
    }
}
