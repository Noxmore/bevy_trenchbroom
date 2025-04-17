use bevy::{
	asset::AssetPath,
	ecs::{component::ComponentId, world::DeferredWorld},
};

use super::*;
use crate::*;

/// Spawns the model stored in this class' `model` property, using the optional asset label specified that it will use to get the scene from the loaded asset.
///
/// This is the internal function that you should use when creating your own model loading hooks.
/// For general use, you should use functions like [`spawn_class_gltf`] for better ergonomics.
///
/// NOTE: This currently only works for simple paths (e.g. `#[model("path/to/model")]`), more advanced uses of the `model` property won't work.
pub fn spawn_class_model_internal<T: QuakeClass>(mut world: DeferredWorld, entity: Entity, label: Option<&'static str>) {
	let Some(asset_server) = world.get_resource::<AssetServer>() else { return };
	let model_path = T::CLASS_INFO
		.model
		.expect("`spawn_class_model` called but `model` property not specified!");

	let mut model_path = AssetPath::from(model_path.trim_matches('"'));

	if let Some(label) = label {
		model_path = model_path.with_label(label);
	}

	let model_handle = asset_server.load(model_path);

	world.commands().entity(entity).insert(SceneRoot(model_handle));
}

/// Spawns the model stored in this class' `model` property as a gltf.
///
/// This function exists in such a way that you can directly use it as a component hook for your class, or call it from within an existing component hook.
///
/// NOTE: This currently only works for simple paths (e.g. `#[model("path/to/model")]`), more advanced uses of the `model` property won't work.
///
/// # Examples
/// ```
/// # use bevy::prelude::*;
/// # use bevy_trenchbroom::prelude::*;
/// #[derive(PointClass, Component, Reflect)]
/// #[reflect(Component)]
/// #[base(Transform)]
/// #[model("models/mushroom.glb")]
/// #[size(-4 -4 0, 4 4 16)]
/// #[component(on_add = spawn_class_gltf::<Mushroom>)]
/// pub struct Mushroom;
/// ```
pub fn spawn_class_gltf<T: QuakeClass>(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
	if world.is_scene_world() {
		return;
	}
	world.commands().entity(entity).insert(TrenchBroomGltfRotationFix);
	spawn_class_model_internal::<T>(world, entity, Some("Scene0"))
}
