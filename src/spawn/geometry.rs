//! Handles spawning brushes from a [MapEntity] into the Bevy world.

use bevy::{pbr::Lightmap, render::{mesh::VertexAttributeValues, render_resource::Face}};

use crate::*;

impl<'w> EntitySpawnView<'w> {
    /// Spawns all the brushes in this entity, and parents them to said entity.
    pub fn spawn_brushes(&self, world: &mut World, entity: Entity, settings: BrushSpawnSettings) {
        // We need to pass the texture's material properties to the view
        world.resource_scope(|world, mut mat_properties_assets: Mut<Assets<MaterialProperties>>| {
            // let mut mat_properties_assets = std::cell::UnsafeCell::new(mat_properties_assets);

            // Used for material properties where it's file doesn't exist
            let default_material_properties = MaterialProperties::default();

            
            // Create, or retrieve the meshes from the entity
            let meshes: Vec<(MapEntityGeometryTexture, Mesh)> = match &self.map_entity.geometry {
                MapEntityGeometry::Bsp(bsp_meshes) => {
                    bsp_meshes.iter().map(|(texture, mesh)| {
                        let mut mesh = mesh.clone();
                        mesh.asset_usage = self.server.config.brush_mesh_asset_usages;
                        (texture.clone(), mesh)
                    }).collect()
                }
                
                MapEntityGeometry::Map(brushes) => {
                    // Each element of this vector represents a brush, each brush is a vector the polygonized surfaces of said brush
                    let mut faces = Vec::new();

                    for brush in brushes {
                        faces.push(brush.polygonize());
                    }
                    
                    // Since bevy can only have 1 material per mesh, surfaces with the same material are grouped here, each group will have its own mesh.
                    let mut grouped_surfaces: HashMap<&str, Vec<&BrushSurfacePolygon>> = default();
                    for face in faces.iter().flatten() {
                        grouped_surfaces
                            .entry(&face.surface.texture)
                            .or_insert_with(Vec::new)
                            .push(face);
                    }

                    grouped_surfaces.into_iter().map(|(texture, polygons)| {
                        (MapEntityGeometryTexture { name: texture.to_string(), embedded: None, lightmap: None, special: false }, generate_mesh_from_brush_polygons(polygons.as_slice(), &self.server.config))
                    }).collect()
                }
            };


            // Stores BrushSpawnViews, just with a Handle to MaterialProperties instead of a reference, for the borrow checker
            // I know, it's an ugly solution, but it gets the job done
            let mut brush_mesh_views = Vec::new();

            for (texture, mut mesh) in meshes {
                let mat_properties_path = self.server.config.texture_root.join(&texture.name).with_extension(MATERIAL_PROPERTIES_EXTENSION);
                let full_mat_properties_path = self.server.config.assets_path.join(&mat_properties_path);

                // If we don't check if the material properties file exists, the asset server will scream that it doesn't
                // This is a lot of filesystem calls, but i'm unsure of a better way to do this
                // We could make a cache mapping each path to whether it exists or not, but that's really only a band-aid fix
                // It would be better to get all this off the critical path in the first place, or at least pre-load all this when loading maps
                let mat_properties_handle = if full_mat_properties_path.exists() {
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
                    // mat_properties_assets.get(&mat_properties_handle).unwrap()
                    Some(mat_properties_handle)
                } else {
                    None
                };

                if let Err(err) = mesh.generate_tangents() {
                    error!("Couldn't generate tangents for brush in MapEntity {entity:?} (index {:?}) with texture {}: {err}", self.map_entity.ent_index, texture.name);
                }

                let mesh_entity = world.spawn(Name::new(texture.name.clone())).id();

                // Because of the borrow checker, we have to push the handle, not just a reference to the material properties. Tuples it is!
                brush_mesh_views.push((mesh_entity, mesh, texture, mat_properties_handle));

                world.entity_mut(entity).add_child(mesh_entity);
            }

            let mut view = BrushSpawnView {
                entity_spawn_view: self,
                meshes: brush_mesh_views.into_iter().map(|(entity, mesh, texture, mat_properties_handle)| {
                    BrushMeshView {
                        entity,
                        mesh,
                        texture,
                        mat_properties: mat_properties_handle.map(|handle| mat_properties_assets.get(&handle).unwrap()).unwrap_or(&default_material_properties)
                    }
                }).collect(),
            };

            for spawner in settings.spawners {
                spawner(world, entity, &mut view);
            }
            for spawner in &self.server.config.global_brush_spawners {
                spawner(world, entity, &mut view);
            }
        });

        // To keep the visibility hierarchy for the possible child meshes when spawning these brushes
        let mut ent = world.entity_mut(entity);
        if !ent.contains::<Visibility>() {
            ent.insert(Visibility::default());
        }
        if !ent.contains::<InheritedVisibility>() {
            ent.insert(InheritedVisibility::default());
        }
        if !ent.contains::<ViewVisibility>() {
            ent.insert(ViewVisibility::default());
        }
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
    pub meshes: Vec<BrushMeshView<'w>>,
}
impl<'w, 'l> std::ops::Deref for BrushSpawnView<'w, 'l> {
    type Target = EntitySpawnView<'w>;
    fn deref(&self) -> &Self::Target {
        self.entity_spawn_view
    }
}

pub struct BrushMeshView<'w> {
    pub entity: Entity,
    pub mesh: Mesh,
    pub texture: MapEntityGeometryTexture,
    pub mat_properties: &'w MaterialProperties,
}

/// A good starting threshold in radians for interpolating similar normals, creating smoother curved surfaces.
pub const DEFAULT_NORMAL_SMOOTH_THRESHOLD: f32 = std::f32::consts::FRAC_PI_4;

/// A collection of spawners to call on each brush/mesh produced when spawning a brush.
#[derive(Default)]
pub struct BrushSpawnSettings {
    spawners: Vec<Box<dyn Fn(&mut World, Entity, &mut BrushSpawnView)>>,
}

impl BrushSpawnSettings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a function to the settings' spawner stack.
    pub fn spawner(
        mut self,
        spawner: impl Fn(&mut World, Entity, &mut BrushSpawnView) + 'static,
    ) -> Self {
        self.spawners.push(Box::new(spawner));
        self
    }

    /// Any intersecting vertices where the angle between their normals in radians is less than [DEFAULT_NORMAL_SMOOTH_THRESHOLD] will have their normals interpolated, making curved surfaces look smooth.
    ///
    /// Shorthand for `self.smooth_by_angle(DEFAULT_NORMAL_SMOOTH_THRESHOLD)` to reduce syntactic noise.
    pub fn smooth_by_default_angle(self) -> Self {
        self.smooth_by_angle(DEFAULT_NORMAL_SMOOTH_THRESHOLD)
    }

    /// Any intersecting vertices where the angle between their normals in radians is less than `normal_smooth_threshold` will have their normals interpolated, making curved surfaces look smooth.
    /// [DEFAULT_NORMAL_SMOOTH_THRESHOLD] is a good starting value for this, shorthanded by [smooth_by_default_angle\()](Self::smooth_by_default_angle).
    ///
    /// if `normal_smooth_threshold` is <= 0, nothing will happen.
    pub fn smooth_by_angle(self, normal_smooth_threshold: f32) -> Self {
        self.spawner(move |_world, _entity, view| {
            if normal_smooth_threshold <= 0. {
                return; // The user doesn't want to smooth after all!
            }

            #[derive(Clone, Copy, PartialEq, Eq, Hash)]
            struct Vec3Ord([FloatOrd; 3]);

            // It's either a map or a doubly-connected edge list, the prior seems to work well enough.
            let mut vertex_map: HashMap<Vec3Ord, Vec<&mut [f32; 3]>> = default();


            let ent_index = view.map_entity.ent_index; // Borrow checker
            // We go through all the meshes and add all their normals into vertex_map
            for mesh_view in &mut view.meshes {
                if !mesh_view.mat_properties.get(MaterialProperties::RENDER) {
                    continue;
                }

                // SAFETY: Getting ATTRIBUTE_POSITION and ATTRIBUTE_NORMAL gives us 2 different attributes, but the borrow checker doesn't know that!
                let mesh2 = unsafe { &mut *std::ptr::from_mut(&mut mesh_view.mesh) };

                let Some(positions) = mesh_view.mesh.attribute(Mesh::ATTRIBUTE_POSITION).map(VertexAttributeValues::as_float3).flatten() else {
                    error!("[entity {} (map entity {:?})] Tried to smooth by angle, but the ATTRIBUTE_POSITION doesn't exist on mesh!", mesh_view.entity, ent_index);
                    return;
                };
                let positions_len = positions.len();

                let Some(normals) = mesh2.attribute_mut(Mesh::ATTRIBUTE_NORMAL).map(|values| match values {
                    VertexAttributeValues::Float32x3(v) => Some(v),
                    _ => None,
                }).flatten() else {
                    error!("[entity {} (map entity {:?})] Tried to smooth by angle, but the ATTRIBUTE_NORMAL doesn't exist on mesh!", mesh_view.entity, ent_index);
                    return;
                };
                let normals_len = normals.len();

                if normals_len != positions_len {
                    error!("[entity {} (map entity {:?})] Tried to smooth by angle, but ATTRIBUTE_NORMAL len doesn't match ATTRIBUTE_POSITION len! ({} and {})", mesh_view.entity, ent_index, normals_len, positions_len);
                    return;
                }

                for (i, normal) in normals.into_iter().enumerate() {
                    // Let's make this lower precision, just in case
                    let position = Vec3Ord(positions[i].map(|v| FloatOrd((v * 10000.).round() / 10000.)));

                    vertex_map.entry(position).or_default().push(normal);
                }
            }


            for (_position, mut normals) in vertex_map {
                use disjoint_sets::*;

                if normals.len() <= 1 { // There are no duplicates
                    continue;
                }

                // Group normals to be smoothed
                let mut uf = UnionFind::new(normals.len());

                for ((a_i, a), (b_i, b)) in normals.iter().map(|v| Vec3::from(**v)).enumerate().tuple_combinations() {
                    if a.angle_between(b) < normal_smooth_threshold {
                        uf.union(a_i, b_i);
                    }
                }

                // Put the groups into an easily iterable structure, then average the normals in each group
                let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
                for i in 0..normals.len() {
                    let root = uf.find(i);
                    groups.entry(root).or_default().push(i);
                }

                for (_, group) in groups {
                    let new_normal = group.iter().map(|idx| Vec3::from(*normals[*idx])).sum::<Vec3>() / normals.len() as f32;

                    for idx in group {
                        *normals[idx] = new_normal.to_array();
                    }
                }
            }
        })
    }

    /// Spawns child entities with meshes per each material used, loading said materials in the process.
    /// Will do nothing is your config is specified to be a server.
    pub fn pbr_mesh(self) -> Self {
        self.spawner(|world, _entity, view| {
            if view.server.config.is_server {
                return;
            }

            for mesh_view in &view.meshes {
                if !mesh_view.mat_properties.get(MaterialProperties::RENDER) {
                    continue;
                }

                let default_material = StandardMaterial {
                    perceptual_roughness: mesh_view
                        .mat_properties
                        .get(MaterialProperties::ROUGHNESS),
                    metallic: mesh_view.mat_properties.get(MaterialProperties::METALLIC),
                    alpha_mode: mesh_view
                        .mat_properties
                        .get(MaterialProperties::ALPHA_MODE)
                        .into(),
                    emissive: mesh_view.mat_properties.get(MaterialProperties::EMISSIVE),
                    cull_mode: if mesh_view
                        .mat_properties
                        .get(MaterialProperties::DOUBLE_SIDED)
                    {
                        None
                    } else {
                        Some(Face::Back)
                    },
                    lightmap_exposure: view.server.config.default_lightmap_exposure,
                    ..default()
                };

                let mut material = match &mesh_view.texture.embedded {
                    Some(embedded) => {
                        StandardMaterial {
                            base_color_texture: Some(embedded.image_handle.clone()),
                            alpha_mode: embedded.alpha_mode,
                            ..default_material
                        }
                    },
                    None => {
                        let asset_server = world.resource::<AssetServer>();
                        
                        // view.server.material_cache
                        //     .lock()
                        //     .entry(mesh_view.texture.name.clone())
                        //     .or_insert_with(|| {
                                
                        //     })
                        //     .clone()
                        // TODO cache is hard because of different material types, do we need it?
                        //      NOTE: it also makes having multiple maps loaded at once not really work
                        //      I could use a Handle<Image> as a key

                        macro_rules! load_texture {
                            ($map:literal) => {{
                                let texture_path = format!(
                                    concat!("{}/{}", $map, ".{}"),
                                    view.server.config.texture_root.display(),
                                    mesh_view.texture.name,
                                    view.server.config.texture_extension
                                );
                                // TODO This is a lot of file system calls on the critical path, how can we offload this?
                                if view.server.config.assets_path.join(&texture_path).exists() {
                                    Some(asset_server.load(texture_path))
                                } else {
                                    None
                                }
                            }};
                        }

                        let base_color_texture = load_texture!("");
                        let normal_map_texture = load_texture!("_normal");
                        let metallic_roughness_texture = load_texture!("_mr");
                        let emissive_texture = load_texture!("_emissive");
                        let depth_texture = load_texture!("_depth");

                        StandardMaterial {
                            base_color_texture,
                            normal_map_texture,
                            metallic_roughness_texture,
                            emissive_texture,
                            depth_map: depth_texture,
                            ..default_material
                        }
                    }
                };

                if mesh_view.texture.special {
                    material.emissive_texture = material.base_color_texture.clone();
                    material.emissive = LinearRgba::WHITE;
                }

                let mesh_handle = world.resource::<AssetServer>().add(mesh_view.mesh.clone());
                world.entity_mut(mesh_view.entity).insert(Mesh3d(mesh_handle));
                (view.server.config.material_application_hook)(material, mesh_view, world, view);
            }
        })
    }

    /// Inserts lightmaps if available.
    pub fn with_lightmaps(self) -> Self {
        self.spawner(|world, entity, view| {
            for mesh_view in &view.meshes {
                if mesh_view.texture.special { continue }
                let Some(animated_lighting_handle) = &mesh_view.texture.lightmap else { continue };
                let Some(animated_lighting) = world.resource::<Assets<AnimatedLighting>>().get(animated_lighting_handle) else {
                    error!("Animated lighting for entity {entity} (index {:?}) doesn't exist!", view.map_entity.ent_index);
                    continue;
                };
                let lightmap_handle = animated_lighting.output.clone();
                
                world.entity_mut(mesh_view.entity)
                    .insert(Lightmap { image: lightmap_handle.clone(), uv_rect: Rect::new(0., 0., 1., 1.) });
            }
        })
    }

    #[cfg(feature = "rapier")]
    /// Inserts trimesh colliders on each mesh this entity's brushes produce. This means that brushes will be hollow. Not recommended to use on physics objects.
    pub fn trimesh_collider(self) -> Self {
        self.spawner(|world, _entity, view| {
            for mesh_view in &view.meshes {
                if !mesh_view.mat_properties.get(MaterialProperties::COLLIDE) {
                    continue;
                }

                use bevy_rapier3d::prelude::*;
                world.entity_mut(mesh_view.entity).insert(
                    bevy_rapier3d::geometry::Collider::from_bevy_mesh(
                        &mesh_view.mesh,
                        &ComputedColliderShape::TriMesh,
                    )
                    .unwrap(),
                );
            }
        })
    }

    #[cfg(feature = "avian")]
    /// Inserts trimesh colliders on each mesh this entity's brushes produce. This means that brushes will be hollow. Not recommended to use on physics objects.
    pub fn trimesh_collider(self) -> Self {
        self.spawner(|world, entity, view| {
            for mesh_view in &view.meshes {
                if !mesh_view.mat_properties.get(MaterialProperties::COLLIDE) {
                    continue;
                }

                use avian3d::prelude::*;
                if let Some(collider) = Collider::trimesh_from_mesh(&mesh_view.mesh) {
                    world.entity_mut(entity).insert(collider);
                }
            }
        })
    }

    // TODO convex colliders with BSPs

    #[cfg(feature = "rapier")]
    /// Inserts a compound collider of every brush in this entity into said entity. This means that even faces with [MaterialKind::Empty] will still have collision, and brushes will be fully solid.
    pub fn convex_collider(self) -> Self {
        self.spawner(|world, entity, view| {
            use bevy_rapier3d::prelude::*;

            let mut colliders = Vec::new();

            for mesh_view in view.meshes.iter() {
                let Some(collider) = Collider::from_bevy_mesh(&mesh_view.mesh, &ComputedColliderShape::ConvexHull) else {
                    error!("MapEntity {entity} has an invalid (non-convex) brush, and a collider could not be computed for it!");
                    continue;
                };
                colliders.push((
                    Vec3::ZERO,
                    Quat::IDENTITY,
                    collider,
                ));
            }

            world.entity_mut(entity).insert(Collider::compound(colliders));
        })
    }

    #[cfg(feature = "avian")]
    /// Inserts a compound collider of every brush in this entity into said entity. This means that even faces with [MaterialKind::Empty] will still have collision, and brushes will be fully solid.
    pub fn convex_collider(self) -> Self {
        self.spawner(|world, entity, view| {
            use avian3d::prelude::*;
            
            let mut colliders = Vec::new();

            for mesh_view in view.meshes.iter() {
                if let Some(collider) = Collider::convex_hull_from_mesh(&mesh_view.mesh) {
                    colliders.push((Vec3::ZERO, Quat::IDENTITY, collider))
                }
            }
            world
                .entity_mut(entity)
                .insert(Collider::compound(colliders));
        })
    }
}
