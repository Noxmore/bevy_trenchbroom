//! Handles spawning brushes from a [MapEntity] into the Bevy world.

use bevy::render::render_resource::Face;

use crate::*;

impl<'w> EntitySpawnView<'w> {
    /// Spawns all the brushes in this entity, and parents them to said entity, returning the list of entities that have been spawned.
    pub fn spawn_brushes(
        &self,
        world: &mut World,
        entity: Entity,
        settings: BrushSpawnSettings,
    ) -> Vec<Entity> {
        let mut entities = Vec::new();
        // Each element of this vector is a vector the polygonized surfaces of a brush
        let mut faces = Vec::new();

        for brush in &self.map_entity.brushes {
            faces.push(brush.polygonize());
        }

        let brush_spawn_view = BrushSpawnView {
            entity_spawn_view: self,
            brushes: &faces,
        };
        for spawner in &settings.brush_spawners {
            entities.append(&mut spawner(world, entity, &brush_spawn_view));
        }

        // Since bevy can only have 1 material per mesh, surfaces with the same material are grouped here, each group will have its own mesh.
        let mut grouped_surfaces: HashMap<&str, Vec<&BrushSurfacePolygon>> = default();
        for face in faces.iter().flatten() {
            grouped_surfaces
                .entry(&face.surface.texture)
                .or_insert_with(Vec::new)
                .push(face);
        }

        // We need to pass the texture's material properties to the view
        world.resource_scope(|world, mut mat_properties_assets: Mut<Assets<MaterialProperties>>| {
            // Used for material properties where it's file doesn't exist
            let default_material_properties = MaterialProperties::default();

            // Construct the meshes for each surface group
            for (texture, faces) in grouped_surfaces {
                let mat_properties_path = self.tb_config.texture_root.join(texture).with_extension(MATERIAL_PROPERTIES_EXTENSION);
                let full_mat_properties_path = self.tb_config.assets_path.join(&mat_properties_path);

                // If we don't check if the material properties file exists, the asset server will scream that it doesn't
                // This is a lot of filesystem calls, but i'm unsure of a better way to do this
                // We could make a cache mapping each path to whether it exists or not, but that's really only a band-aid fix
                // It would be better to get all this off the critical path in the first place, or at least pre-load all this when loading maps
                let mat_properties = if full_mat_properties_path.exists() {
                    // TODO does this work or does it load every time
                    let mat_properties_handle = world.resource::<AssetServer>().load::<MaterialProperties>(mat_properties_path.clone());

                    // This is an expanded version of mat_properties_assets.get_or_insert_with so that i can access control flow in the error state
                    if !mat_properties_assets.contains(&mat_properties_handle) {
                        match || -> anyhow::Result<MaterialProperties> {
                            Ok(MaterialPropertiesLoader.load_sync(&fs::read_to_string(&full_mat_properties_path)?)?)
                        }() {
                            Ok(mat_properties) => mat_properties_assets.insert(&mat_properties_handle, mat_properties),
                            Err(err) => {
                                error!("Error reading MaterialProperties from {} when spawning brush {entity:?} (index: {:?}): {err}", full_mat_properties_path.display(), self.map_entity.ent_index);
                                continue
                            }
                        }
                    }
                    mat_properties_assets.get(&mat_properties_handle).unwrap()
                } else {
                    &default_material_properties
                };

                let mut mesh = generate_mesh_from_brush_polygons(faces.as_slice(), self.tb_config);
                // TODO this makes pbr maps work, but messes up the lighting for me, why????
                if let Err(err) = mesh.generate_tangents() {
                    error!("Couldn't generate tangents for brush in MapEntity {entity:?} (index {:?}) with texture {texture}: {err}", self.map_entity.ent_index);
                }

                let brush_mesh_insertion_view = BrushMeshSpawnView {
                    entity_spawn_view: self,
                    mesh: &mut mesh,
                    texture,
                    mat_properties,
                };

                let mut ent = world.spawn(Name::new(texture.to_string()));

                for spawner in &settings.mesh_spawners {
                    spawner(&mut ent, &brush_mesh_insertion_view);
                }

                entities.push(ent.id());
            }
        });

        let mut ent = world.entity_mut(entity);
        ent.push_children(&entities);
        // To keep the visibility hierarchy for the possible child meshes when spawning these brushes
        if !ent.contains::<Visibility>() {
            ent.insert(Visibility::default());
        }
        if !ent.contains::<InheritedVisibility>() {
            ent.insert(InheritedVisibility::default());
        }
        if !ent.contains::<ViewVisibility>() {
            ent.insert(ViewVisibility::default());
        }

        entities
    }
}

pub struct BrushMeshSpawnView<'w, 'l> {
    entity_spawn_view: &'l EntitySpawnView<'w>,
    pub mesh: &'l mut Mesh,
    pub texture: &'l str,
    pub mat_properties: &'w MaterialProperties,
}
impl<'w, 'l> std::ops::Deref for BrushMeshSpawnView<'w, 'l> {
    type Target = EntitySpawnView<'w>;
    fn deref(&self) -> &Self::Target {
        self.entity_spawn_view
    }
}
pub struct BrushSpawnView<'w, 'l> {
    entity_spawn_view: &'l EntitySpawnView<'w>,
    /// A Vec of computed brushes, each brush is a Vec of [BrushSurfacePolygon]s.
    pub brushes: &'l Vec<Vec<BrushSurfacePolygon>>,
}
impl<'w, 'l> std::ops::Deref for BrushSpawnView<'w, 'l> {
    type Target = EntitySpawnView<'w>;
    fn deref(&self) -> &Self::Target {
        self.entity_spawn_view
    }
}

/// A collection of spawners to call on each brush/mesh produced when spawning a brush.
#[derive(Default)]
pub struct BrushSpawnSettings {
    mesh_spawners: Vec<Box<dyn Fn(&mut EntityWorldMut, &BrushMeshSpawnView)>>,
    brush_spawners: Vec<Box<dyn Fn(&mut World, Entity, &BrushSpawnView) -> Vec<Entity>>>,
}

impl BrushSpawnSettings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calls the specified function on each mesh produced by the entity.
    pub fn mesh_spawner(
        mut self,
        spawner: impl Fn(&mut EntityWorldMut, &BrushMeshSpawnView) + 'static,
    ) -> Self {
        self.mesh_spawners.push(Box::new(spawner));
        self
    }

    /// Calls a function with an inserting entity, after said entity polygonizes it's brushes.
    pub fn brush_spawner(
        mut self,
        spawner: impl Fn(&mut World, Entity, &BrushSpawnView) -> Vec<Entity> + 'static,
    ) -> Self {
        self.brush_spawners.push(Box::new(spawner));
        self
    }

    /// Spawns child entities with meshes per each material used, loading said materials in the process.
    /// Will do nothing is your config is specified to be a server.
    pub fn pbr_mesh(self) -> Self {
        self.mesh_spawner(|ent, view| {
            if view.tb_config.is_server {
                return;
            }

            let asset_server = ent.world().resource::<AssetServer>();

            if !view.mat_properties.get(MaterialProperties::RENDER) {
                return;
            }

            let material = BRUSH_TEXTURE_TO_MATERIALS_CACHE
                .lock()
                .unwrap()
                .entry(view.texture.into())
                .or_insert_with(|| {
                    macro_rules! load_texture {
                        ($name:ident = $map:literal) => {
                            let __texture_path = format!(
                                concat!("{}/{}", $map, ".{}"),
                                view.tb_config.texture_root.display(),
                                view.texture,
                                view.tb_config.texture_extension
                            );
                            let $name: Option<Handle<Image>> =
                                // TODO This is a lot of file system calls on the critical path, how can we offload this?
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
                        perceptual_roughness: view
                            .mat_properties
                            .get(MaterialProperties::ROUGHNESS),
                        metallic: view.mat_properties.get(MaterialProperties::METALLIC),
                        alpha_mode: view
                            .mat_properties
                            .get(MaterialProperties::ALPHA_MODE)
                            .into(),
                        emissive: view.mat_properties.get(MaterialProperties::EMISSIVE),
                        cull_mode: if view.mat_properties.get(MaterialProperties::DOUBLE_SIDED) {
                            None
                        } else {
                            Some(Face::Back)
                        },
                        ..default()
                    })
                })
                .clone();

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
        self.mesh_spawner(|ent, view| {
            if !view.mat_properties.get(MaterialProperties::COLLIDE) {
                return;
            }

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

    #[cfg(feature = "xpbd")]
    /// Inserts trimesh colliders on each mesh this entity's brushes produce. This means that brushes will be hollow. Not recommended to use on physics objects.
    pub fn trimesh_collider(self) -> Self {
        self.mesh_spawner(|ent, view| {
            if !view.mat_properties.get(MaterialProperties::COLLIDE) {
                return;
            }

            use bevy_xpbd_3d::prelude::*;
            if let Some(collider) = Collider::trimesh_from_mesh(&view.mesh) {
                ent.insert(collider);
            }
        })
    }

    #[cfg(feature = "rapier")]
    /// Inserts a compound collider of every brush in this entity into said entity. This means that even faces with [MaterialKind::Empty] will still have collision, and brushes will be fully solid.
    pub fn convex_collider(self) -> Self {
        self.brush_spawner(|world, entity, view| {
            use bevy_rapier3d::prelude::*;

            let mut colliders = Vec::new();

            for faces in view.brushes.iter() {
                let mesh = generate_mesh_from_brush_polygons(
                    &faces.iter().collect::<Vec<_>>(),
                    view.tb_config,
                );
                let Some(collider) = Collider::from_bevy_mesh(&mesh, &ComputedColliderShape::ConvexHull) else {
                    error!("MapEntity {entity:?} has an invalid (non-convex) brush, and a collider could not be computed for it!");
                    continue;
                };
                colliders.push((
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    collider,
                ));
            }

            world
                .entity_mut(entity)
                .insert(Collider::compound(colliders));
            Vec::new()
        })
    }

    #[cfg(feature = "xpbd")]
    /// Inserts a compound collider of every brush in this entity into said entity. This means that even faces with [MaterialKind::Empty] will still have collision, and brushes will be fully solid.
    pub fn convex_collider(self) -> Self {
        self.brush_spawner(|world, entity, view| {
            use bevy_xpbd_3d::prelude::*;
            let mut colliders = Vec::new();
            for faces in view.brushes.iter() {
                let mesh = generate_mesh_from_brush_polygons(
                    &faces.iter().collect::<Vec<_>>(),
                    view.tb_config,
                );
                if let Some(collider) = Collider::convex_hull_from_mesh(&mesh) {
                    colliders.push((Vec3::ZERO, Quat::IDENTITY, collider))
                }
            }
            world
                .entity_mut(entity)
                .insert(Collider::compound(colliders));
            Vec::new()
        })
    }
}
