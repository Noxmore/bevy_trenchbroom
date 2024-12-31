use bevy::{asset::LoadContext, pbr::Lightmap, render::{mesh::VertexAttributeValues, render_resource::Face}};
use bsp::BspEmbeddedTexture;
use physics::{TrimeshCollision, ConvexCollision};
use qmap::QuakeMapEntity;

use crate::*;

/// A good starting threshold in radians for interpolating similar normals, creating smoother curved surfaces.
pub const DEFAULT_NORMAL_SMOOTH_THRESHOLD: f32 = std::f32::consts::FRAC_PI_4;

/// Contains the brushes that a solid entity is made of.
/// 
/// Can either be [Owned](Brushes::Owned), meaning the brushes are stored directly in the component itself (useful for dynamically editing brushes),
/// or [Shared](Brushes::Shared), which reads from an asset instead for completely static geometry, usually from .
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
#[require(Transform)]
pub enum Brushes {
    Owned(BrushList),
    Shared(Handle<BrushList>),
}
impl Brushes {
    pub fn get<'l, 'w: 'l>(&'l self, brush_lists: &'w Assets<BrushList>) -> Option<&'l BrushList> {
        match self {
            Self::Owned(list) => Some(list),
            Self::Shared(handle) => brush_lists.get(handle),
        }
    }
}

#[derive(Asset, Reflect, Debug, Clone)]
pub struct BrushList(pub Vec<Brush>);
impl std::ops::Deref for BrushList {
    type Target = [Brush];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


#[derive(Reflect, Debug, Clone, PartialEq, Eq)]
pub struct MapGeometryTexture<'w> {
    pub name: String,
    pub embedded: Option<&'w BspEmbeddedTexture>,
    pub lightmap: Option<&'w Handle<AnimatedLighting>>,
    /// If the texture should be full-bright
    pub special: bool,
}

pub struct GeometryProviderMeshView<'w> {
    pub entity: Entity,
    pub mesh: Mesh,
    pub texture: MapGeometryTexture<'w>,
    pub mat_properties: &'w MaterialProperties,
}

pub struct GeometryProviderView<'w, 'l> {
    pub world: &'w mut World,
    pub entity: Entity,
    pub tb_server: &'w TrenchBroomServer,
    /// The main world's asset server, this is here for things you can't do with `load_context`'s abstraction. So use `load_context` for asset-related things unless you *have* to use this.
    pub asset_server: &'w AssetServer,
    pub map_entity: &'w QuakeMapEntity,
    pub map_entity_idx: usize,
    pub meshes: Vec<GeometryProviderMeshView<'w>>,
    pub load_context: &'l mut LoadContext<'l>,
}
impl GeometryProviderView<'_, '_> {
    /// Shorthand for adding a labled
    pub fn add_material<M: Material>(&mut self, mesh_idx: usize, material: M) -> Handle<M> {
        self.load_context.add_labeled_asset(format!("Entity{}Material{mesh_idx}", self.map_entity_idx), material)
    }
}

#[derive(Default)]
pub struct GeometryProvider {
    providers: Vec<Box<dyn Fn(&mut GeometryProviderView)>>,
}

impl GeometryProvider {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a function to the settings' spawner stack.
    pub fn push(
        mut self,
        provider: impl Fn(&mut GeometryProviderView) + 'static,
    ) -> Self {
        self.providers.push(Box::new(provider));
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
        self.push(move |view| {
            if normal_smooth_threshold <= 0. {
                return; // The user doesn't want to smooth after all!
            }

            #[derive(Clone, Copy, PartialEq, Eq, Hash)]
            struct Vec3Ord([FloatOrd; 3]);

            // It's either a map or a doubly-connected edge list, the prior seems to work well enough.
            let mut vertex_map: HashMap<Vec3Ord, Vec<&mut [f32; 3]>> = default();


            let ent_index = view.map_entity_idx; // Borrow checker
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
        self.push(|view| {
            if view.tb_server.config.is_server {
                return;
            }

            // Borrow checker
            let mut texture_applications = Vec::with_capacity(view.meshes.len());

            for (mesh_idx, mesh_view) in view.meshes.iter().enumerate() {
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
                    lightmap_exposure: view.tb_server.config.default_lightmap_exposure,
                    ..default()
                };

                let mut material = match &mesh_view.texture.embedded {
                    Some(embedded) => {
                        // TODO Should PBR be supported here? We probably shouldn't read from loose files, maybe from other embedded textures?
                        StandardMaterial {
                            base_color_texture: Some(embedded.image.clone()),
                            alpha_mode: embedded.alpha_mode,
                            ..default_material
                        }
                    },
                    None => {
                        macro_rules! load_texture {
                            ($map:literal) => {{
                                let texture_path = format!(
                                    concat!("{}/{}", $map, ".{}"),
                                    view.tb_server.config.texture_root.display(),
                                    mesh_view.texture.name,
                                    view.tb_server.config.texture_extension
                                );
                                // TODO this good?
                                if view.asset_server.get_source(texture_path.clone()).is_ok() {
                                    Some(view.load_context.load(texture_path))
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

                let mesh_handle = view.load_context.add_labeled_asset(format!("Entity{}Mesh{mesh_idx}", view.map_entity_idx), mesh_view.mesh.clone());
                view.world.entity_mut(mesh_view.entity).insert(Mesh3d(mesh_handle));
                texture_applications.push((material, mesh_idx));
            }
            
            for (material, mesh_idx) in texture_applications {
                (view.tb_server.config.material_application_hook)(material, view, mesh_idx);
            }
        })
    }

    /// Inserts lightmaps if available.
    pub fn with_lightmaps(self) -> Self {
        self.push(|view| {
            for mesh_view in &view.meshes {
                if mesh_view.texture.special { continue }
                let Some(animated_lighting_handle) = &mesh_view.texture.lightmap else { continue };
                let Some(animated_lighting) = view.world.resource::<Assets<AnimatedLighting>>().get(*animated_lighting_handle) else {
                    error!("Animated lighting for entity {} (index {:?}) doesn't exist!", view.entity, view.map_entity_idx);
                    continue;
                };
                let lightmap_handle = animated_lighting.output.clone();
                
                view.world.entity_mut(mesh_view.entity)
                    .insert(Lightmap { image: lightmap_handle.clone(), uv_rect: Rect::new(0., 0., 1., 1.) });
            }
        })
    }

    /// Inserts trimesh colliders on each mesh of this entity. This means that brushes will be hollow. Not recommended to use on physics objects.
    pub fn trimesh_collider(self) -> Self {
        self.push(|view| {
            for mesh_view in &view.meshes {
                if !mesh_view.mat_properties.get(MaterialProperties::COLLIDE) {
                    continue;
                }

                view.world.entity_mut(mesh_view.entity).insert(TrimeshCollision);
            }
        })
    }

    // TODO convex colliders with BSPs

    #[cfg(feature = "rapier")]
    /// Inserts a compound collider of every brush in this entity into said entity. This means that even faces with [MaterialKind::Empty] will still have collision, and brushes will be fully solid.
    pub fn convex_collider(self) -> Self {
        self.push(|view| {
            view.world.entity_mut(view.entity).insert(ConvexCollision);
        })
    }
}