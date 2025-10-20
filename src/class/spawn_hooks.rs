use super::*;
use crate::config::SpawnFnOnce;
use crate::*;
use bevy::ecs::lifecycle::HookContext;
use bevy::{asset::AssetPath, ecs::world::DeferredWorld};
use bevy_mesh::VertexAttributeValues;

/// A good starting threshold in radians for interpolating similar normals, creating smoother curved surfaces.
pub const DEFAULT_NORMAL_SMOOTH_THRESHOLD: f32 = std::f32::consts::FRAC_PI_4;

/// Functions that occur during the spawning of an [`QuakeClass`] entity into a scene world.
#[derive(Default)]
pub struct SpawnHooks {
	pub hooks: Vec<Box<SpawnFnOnce>>,
}
impl SpawnHooks {
	pub fn new() -> Self {
		Self::default()
	}

	/// Consumes this, and applies all hooks contained within.
	pub fn apply(self, view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
		for hook in self.hooks {
			hook(view)?;
		}
		Ok(())
	}

	/// Add a function to the hook stack.
	pub fn push(mut self, provider: impl FnOnce(&mut QuakeClassSpawnView) -> anyhow::Result<()> + Send + Sync + 'static) -> Self {
		self.hooks.push(Box::new(provider));
		self
	}

	/// Inserts a bundle onto the entity.
	///
	/// This is a convenience function, you should generally use required components instead of this if at all possible.
	pub fn with(self, bundle: impl Bundle) -> Self {
		self.push(move |view| {
			view.world.entity_mut(view.entity).insert(bundle);
			Ok(())
		})
	}
	/// Inserts a bundle onto all mesh entities.
	pub fn meshes_with(self, bundle: impl Bundle + Clone) -> Self {
		self.push(move |view| {
			for mesh_view in view.meshes.iter() {
				view.world.entity_mut(mesh_view.entity).insert(bundle.clone());
			}
			Ok(())
		})
	}

	/// Any intersecting vertices where the angle between their normals in radians is less than [`DEFAULT_NORMAL_SMOOTH_THRESHOLD`] will have their normals interpolated, making curved surfaces look smooth.
	///
	/// Shorthand for `self.smooth_by_angle(DEFAULT_NORMAL_SMOOTH_THRESHOLD)` to reduce syntactic noise.
	pub fn smooth_by_default_angle(self) -> Self {
		self.smooth_by_angle(DEFAULT_NORMAL_SMOOTH_THRESHOLD)
	}

	/// Any intersecting vertices where the angle between their normals in radians is less than `normal_smooth_threshold` will have their normals interpolated, making curved surfaces look smooth.
	/// [`DEFAULT_NORMAL_SMOOTH_THRESHOLD`] is a good starting value for this, shorthanded by [`Self::smooth_by_default_angle`].
	///
	/// if `normal_smooth_threshold` is <= 0, nothing will happen.
	pub fn smooth_by_angle(self, normal_smooth_threshold: f32) -> Self {
		self.push(move |view| {
			if normal_smooth_threshold <= 0. {
				return Ok(()); // The user doesn't want to smooth after all!
			}

			#[derive(Clone, Copy, PartialEq, Eq, Hash)]
			struct Vec3Ord([FloatOrd; 3]);

			// It's either a map or a doubly-connected edge list, the prior seems to work well enough.
			let mut vertex_map: HashMap<Vec3Ord, Vec<&mut [f32; 3]>> = default();

			let ent_index = view.src_entity_idx; // Borrow checker
			// We go through all the meshes and add all their normals into vertex_map
			for mesh_view in view.meshes.iter_mut() {
				// SAFETY: Getting ATTRIBUTE_POSITION and ATTRIBUTE_NORMAL gives us 2 different attributes, but the borrow checker doesn't know that!
				let mesh2 = unsafe { &mut *std::ptr::from_mut(&mut mesh_view.mesh) };

				let Some(positions) = mesh_view
					.mesh
					.attribute(Mesh::ATTRIBUTE_POSITION)
					.and_then(VertexAttributeValues::as_float3)
				else {
					anyhow::bail!(
						"[entity {} (map entity {:?})] Tried to smooth by angle, but the ATTRIBUTE_POSITION doesn't exist on mesh!",
						mesh_view.entity,
						ent_index
					);
				};
				let positions_len = positions.len();

				let Some(normals) = mesh2.attribute_mut(Mesh::ATTRIBUTE_NORMAL).and_then(|values| match values {
					VertexAttributeValues::Float32x3(v) => Some(v),
					_ => None,
				}) else {
					anyhow::bail!(
						"[entity {} (map entity {:?})] Tried to smooth by angle, but the ATTRIBUTE_NORMAL doesn't exist on mesh!",
						mesh_view.entity,
						ent_index
					);
				};
				let normals_len = normals.len();

				if normals_len != positions_len {
					anyhow::bail!(
						"[entity {} (map entity {:?})] Tried to smooth by angle, but ATTRIBUTE_NORMAL len doesn't match ATTRIBUTE_POSITION len! ({} and {})",
						mesh_view.entity,
						ent_index,
						normals_len,
						positions_len
					);
				}

				for (i, normal) in normals.iter_mut().enumerate() {
					// Let's make this lower precision, just in case
					let position = Vec3Ord(positions[i].map(|v| FloatOrd((v * 10000.).round() / 10000.)));

					vertex_map.entry(position).or_default().push(normal);
				}
			}

			for (_position, mut normals) in vertex_map {
				use disjoint_sets::*;

				if normals.len() <= 1 {
					// There are no duplicates
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
				let mut groups: HashMap<usize, Vec<usize>> = HashMap::default();
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

			Ok(())
		})
	}

	/// Inserts trimesh colliders on each mesh of this entity. This means that brushes will be hollow. Not recommended to use on physics objects.
	#[cfg(feature = "physics-integration")]
	pub fn trimesh_collider(self) -> Self {
		self.meshes_with(crate::physics::TrimeshCollision)
	}

	/// Inserts a compound collider of every brush in this entity into said entity. Brushes will be fully solid.
	///
	/// NOTE: If you're using BSPs, use the `-wrbrushesonly` command-line argument for `qbsp`, otherwise no brushes will be inserted into the BSP, and no collision will be built!
	#[cfg(feature = "physics-integration")]
	pub fn convex_collider(self) -> Self {
		self.with(crate::physics::ConvexCollision)
	}

	#[cfg(all(feature = "bsp", feature = "client"))]
	pub fn without_lightmaps(self) -> Self {
		use crate::bsp::lighting::AnimatedLightingHandle;

		self.push(move |view| {
			for mesh_view in view.meshes.iter() {
				view.world.entity_mut(mesh_view.entity).remove::<AnimatedLightingHandle>();
			}
			Ok(())
		})
	}
	#[cfg(all(feature = "bsp", not(feature = "client")))]
	pub fn without_lightmaps(self) -> Self {
		self
	}

	/// Spawns the model stored in this class' `model` property, using the optional asset label specified that it will use to get the scene from the loaded asset.
	///
	/// This is the internal function that you should use when creating your own model loading hooks.
	/// For general use, you should use functions like [`spawn_class_gltf(...)`](Self::spawn_class_gltf) for better ergonomics.
	///
	/// TODO: This currently only works for simple paths (e.g. `#[model("path/to/model")]`), more advanced uses of the `model` property won't work.
	pub fn spawn_class_model_internal<T: QuakeClass>(self, label: Option<&'static str>) -> Self {
		self.push(move |view| {
			let Some(model_path) = T::CLASS_INFO.model_path() else {
				anyhow::bail!("`spawn_class_model` called but `model` property missing/invalid!");
			};

			let mut model_path = AssetPath::from(model_path);

			if let Some(label) = label {
				model_path = model_path.with_label(label);
			}

			let model_handle = view.load_context.load(model_path);

			view.world.entity_mut(view.entity).insert(SceneRoot(model_handle));
			Ok(())
		})
	}

	/// Spawns the model stored in this class' `model` property as a gltf.
	///
	/// This function exists in such a way that you can directly use it as a spawn hook for your class, or call it from within an existing spawn hook.
	///
	/// TODO: This currently only works for simple paths (e.g. `#[model("path/to/model")]`), more advanced uses of the `model` property won't work.
	///
	/// # Examples
	/// ```
	/// # use bevy::prelude::*;
	/// # use bevy_trenchbroom::prelude::*;
	/// #[point_class(
	///     model("models/mushroom.glb"),
	///     size(-4 -4 0, 4 4 16),
	///     hooks(SpawnHooks::new().spawn_class_gltf::<Self>()),
	/// )]
	/// pub struct Mushroom;
	/// ```
	pub fn spawn_class_gltf<T: QuakeClass>(self) -> Self {
		self.spawn_class_model_internal::<T>(Some("Scene0"))
	}

	/// Spawn hook that simply loads the path specified in the model, adding it to the map's asset dependencies.
	///
	/// TODO: This currently only works for simple paths (e.g. `#[model("path/to/model")]`), more advanced uses of the `model` property won't work.
	///
	/// # Examples
	/// ```
	/// # use bevy::prelude::*;
	/// # use bevy_trenchbroom::prelude::*;
	/// #[point_class(
	///     model("models/torch.glb"),
	///     hooks(SpawnHooks::new().preload_model::<Self>()),
	/// )]
	/// pub struct Torch;
	/// ```
	pub fn preload_model<T: QuakeClass>(self) -> Self {
		self.push(|view| {
			let Some(model_path) = T::CLASS_INFO.model_path() else {
				anyhow::bail!("`spawn_class_model` called but `model` property missing/invalid!");
			};

			let handle = view.load_context.loader().with_unknown_type().load(model_path);
			view.preload_asset(handle.untyped());
			Ok(())
		})
	}
}

// TODO: Can't reflect until https://github.com/bevyengine/bevy/pull/18827 lands
/// Hacky component that stores a preloaded asset in the scene for just long enough for it not to be detected unused and removed.
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component)]
#[component(storage = "SparseSet", on_insert = Self::on_insert)]
pub struct PreloadedAssets(#[reflect(ignore)] pub Vec<UntypedHandle>);
impl PreloadedAssets {
	pub fn on_insert(mut world: DeferredWorld, ctx: HookContext) {
		// We have to check `is_scene_world` because the vector starts empty.
		if world.is_scene_world() || world.entity(ctx.entity).get::<Self>().map(|model| model.0.is_empty()) != Some(true) {
			return;
		}

		world.commands().entity(ctx.entity).remove_by_id(ctx.component_id);
	}
}

#[cfg(test)]
mod tests {
	#[allow(unused)]
	use super::*;

	#[test]
	#[ignore]
	#[cfg(feature = "client")]
	fn preloading() {
		use crate::{
			geometry::BrushList,
			qmap::{QuakeMap, loader::QuakeMapLoader},
		};
		use bevy::{gltf::GltfPlugin, log::LogPlugin, mesh::MeshPlugin, scene::ScenePlugin};

		#[point_class(
			model("models/mushroom.glb"),
			hooks(SpawnHooks::new().preload_model::<Self>()),
		)]
		#[component(on_add = Self::on_add)]
		pub struct Mushroom;
		impl Mushroom {
			pub fn on_add(mut world: DeferredWorld, ctx: HookContext) {
				let Some(asset_server) = world.get_resource::<AssetServer>() else { return };
				// Loads the scene after adding to the main world.
				let handle = asset_server.load(GltfAssetLabel::Scene(0).from_asset(Self::CLASS_INFO.model_path().unwrap()));

				world.commands().entity(ctx.entity).insert(SceneRoot(handle));
			}
		}

		// This monstrosity is so we only have the things we absolutely need for this test.
		App::new()
			.add_plugins((
				MinimalPlugins,
				AssetPlugin::default(),
				LogPlugin::default(),
				ScenePlugin,
				TransformPlugin,
				MeshPlugin,
				MaterializePlugin::new(TomlMaterialDeserializer),
				GltfPlugin::default(),
				ImagePlugin::default(),
			))
			.insert_resource(TrenchBroomServer::new(
				TrenchBroomConfig::default().suppress_invalid_entity_definitions(true),
			))
			.init_asset::<Image>()
			.init_asset::<BrushList>()
			.init_asset::<StandardMaterial>()
			.init_asset::<QuakeMap>()
			.init_asset_loader::<QuakeMapLoader>()
			.add_systems(Startup, setup)
			.add_systems(Update, spawn_scene)
			.add_systems(Last, exit)
			.add_observer(validate_mesh)
			.add_observer(validate_material)
			.add_observer(validate_scene)
			.run();

		#[derive(Resource)]
		struct MapScene(Handle<Scene>);

		fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
			let handle = smol::block_on(async { asset_server.load_untyped_async("maps/example.map#Scene").await.expect("Asset error") });

			// This is set back so that the Scene can be put in Assets<Scene>, otherwise validate_scene will fail with it.
			commands.insert_resource(MapScene(handle.try_typed::<Scene>().unwrap()));
		}

		fn spawn_scene(mut commands: Commands, scene: Res<MapScene>, mut init: Local<bool>) {
			if !*init {
				commands.spawn(SceneRoot(scene.0.clone()));
				*init = true;
			}
		}

		/// Live for a few ticks to let everything sort out
		fn exit(mut exit: MessageWriter<AppExit>, mut ticks: Local<u32>) {
			*ticks += 1;

			if *ticks > 3 {
				exit.write_default();
			}
		}

		fn validate_mesh(trigger: On<Add, Mesh3d>, mesh_query: Query<&Mesh3d>, asset_server: Res<AssetServer>) {
			let handle = &mesh_query.get(trigger.event_target()).unwrap().0;
			validate_asset(handle, &asset_server, "Mesh");
		}

		fn validate_material(
			trigger: On<Add, MeshMaterial3d<StandardMaterial>>,
			material_query: Query<&MeshMaterial3d<StandardMaterial>>,
			asset_server: Res<AssetServer>,
		) {
			let handle = &material_query.get(trigger.event_target()).unwrap().0;
			validate_asset(handle, &asset_server, "Material");
		}

		fn validate_scene(trigger: On<Add, SceneRoot>, scene_query: Query<&SceneRoot>, asset_server: Res<AssetServer>) {
			let handle = &scene_query.get(trigger.event_target()).unwrap().0;
			validate_asset(handle, &asset_server, "Scene");
		}

		fn validate_asset<A: Asset>(handle: &Handle<A>, asset_server: &AssetServer, type_name: &str) {
			let Some(path) = handle.path() else {
				return;
			};
			if !asset_server.is_loaded_with_dependencies(handle) {
				panic!("{type_name} at path \"{path}\" was not preloaded",);
			}
		}
	}
}
