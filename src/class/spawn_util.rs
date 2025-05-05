use super::*;
use crate::*;
use bevy::ecs::component::HookContext;
use bevy::{asset::AssetPath, ecs::world::DeferredWorld};

/// Spawns the model stored in this class' `model` property, using the optional asset label specified that it will use to get the scene from the loaded asset.
///
/// This is the internal function that you should use when creating your own model loading hooks.
/// For general use, you should use functions like [`spawn_class_gltf`] for better ergonomics.
///
/// TODO: This currently only works for simple paths (e.g. `#[model("path/to/model")]`), more advanced uses of the `model` property won't work.
#[track_caller]
pub fn spawn_class_model_internal<T: QuakeClass>(view: &mut QuakeClassSpawnView, label: Option<&'static str>) {
	let mut model_path = AssetPath::from(
		T::CLASS_INFO
			.model_path()
			.expect("`spawn_class_model` called but `model` property missing/invalid!"),
	);

	if let Some(label) = label {
		model_path = model_path.with_label(label);
	}

	let model_handle = view.load_context.load(model_path);

	view.entity.insert(SceneRoot(model_handle));
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
/// #[derive(PointClass, Component, Reflect)]
/// #[reflect(Component)]
/// #[model("models/mushroom.glb")]
/// #[size(-4 -4 0, 4 4 16)]
/// #[spawn_hook(spawn_class_gltf::<Self>)]
/// pub struct Mushroom;
/// ```
#[track_caller]
pub fn spawn_class_gltf<T: QuakeClass>(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
	spawn_class_model_internal::<T>(view, Some("Scene0"));
	Ok(())
}

/// Spawn hook that simply loads the path specified in the model, adding it to the map's asset dependencies.
///
/// TODO: This currently only works for simple paths (e.g. `#[model("path/to/model")]`), more advanced uses of the `model` property won't work.
///
/// # Examples
/// ```
/// # use bevy::prelude::*;
/// # use bevy_trenchbroom::prelude::*;
/// #[derive(PointClass, Component, Reflect)]
/// #[reflect(Component)]
/// #[model("models/torch.glb")]
/// #[spawn_hook(preload_model::<Self>)]
/// pub struct Torch;
/// ```
#[track_caller]
pub fn preload_model<T: QuakeClass>(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
	let handle = view.load_context.loader().with_unknown_type().load(
		T::CLASS_INFO
			.model_path()
			.expect("`preload_model` called but `model` property missing/invalid!"),
	);
	view.preload_asset(handle.untyped());
	Ok(())
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

#[test]
#[cfg(feature = "client")]
fn preloading() {
	use bevy::render::view::VisibilityClass;

	#[derive(PointClass, Component, Reflect)]
	#[reflect(QuakeClass, Component)]
	#[model("models/mushroom.glb")]
	#[spawn_hook(preload_model::<Self>)]
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
			bevy::log::LogPlugin::default(),
			bevy::scene::ScenePlugin,
			TransformPlugin,
			bevy::render::mesh::MeshPlugin,
			MaterializePlugin::new(TomlMaterialDeserializer),
			bevy::gltf::GltfPlugin::default(),
			ImagePlugin::default(),
		))
		.insert_resource(TrenchBroomServer::new(
			TrenchBroomConfig::default().suppress_invalid_entity_definitions(true),
		))
		.register_type::<Mushroom>()
		.register_type::<bevy::pbr::LightProbe>()
		.register_type::<Visibility>()
		.register_type::<InheritedVisibility>()
		.register_type::<ViewVisibility>()
		.register_type::<crate::bsp::lighting::AnimatedLightingHandle>()
		.register_type::<PreloadedAssets>()
		.register_type::<MeshMaterial3d<StandardMaterial>>()
		.register_type::<Aabb>()
		.register_type::<VisibilityClass>()
		.init_asset::<Image>()
		.init_asset::<StandardMaterial>()
		.init_asset::<crate::bsp::lighting::AnimatedLighting>()
		.init_asset::<crate::bsp::BspBrushesAsset>()
		.init_asset::<crate::bsp::Bsp>()
		.init_asset_loader::<crate::bsp::loader::BspLoader>()
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
		let handle = smol::block_on(async { asset_server.load_untyped_async("maps/example.bsp#Scene").await.expect("Asset error") });

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
	fn exit(mut exit: EventWriter<AppExit>, mut ticks: Local<u32>) {
		*ticks += 1;

		if *ticks > 3 {
			exit.write_default();
		}
	}

	fn validate_mesh(trigger: Trigger<OnAdd, Mesh3d>, mesh_query: Query<&Mesh3d>, asset_server: Res<AssetServer>) {
		let handle = &mesh_query.get(trigger.target()).unwrap().0;
		validate_asset(handle, &asset_server, "Mesh");
	}

	fn validate_material(
		trigger: Trigger<OnAdd, MeshMaterial3d<StandardMaterial>>,
		material_query: Query<&MeshMaterial3d<StandardMaterial>>,
		asset_server: Res<AssetServer>,
	) {
		let handle = &material_query.get(trigger.target()).unwrap().0;
		validate_asset(handle, &asset_server, "Material");
	}

	fn validate_scene(trigger: Trigger<OnAdd, SceneRoot>, scene_query: Query<&SceneRoot>, asset_server: Res<AssetServer>) {
		let handle = &scene_query.get(trigger.target()).unwrap().0;
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
