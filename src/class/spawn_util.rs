use bevy::asset::AssetPath;

use super::*;
use crate::*;

/// Spawns the model stored in this class' `model` property, using the optional asset label specified that it will use to get the scene from the loaded asset.
///
/// This is the internal function that you should use when creating your own model loading hooks.
/// For general use, you should use functions like [`spawn_class_gltf`] for better ergonomics.
///
/// NOTE: This currently only works for simple paths (e.g. `#[model("path/to/model")]`), more advanced uses of the `model` property won't work.
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

/// Spawns the model stored in this class' `model` property as a gltf, and runs [`trenchbroom_gltf_rotation_fix`].
///
/// This function exists in such a way that you can directly use it as a spawn hook for your class, or call it from within an existing spawn hook.
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
/// #[spawn_hook(spawn_class_gltf::<Self>)]
/// pub struct Mushroom;
/// ```
pub fn spawn_class_gltf<T: QuakeClass>(view: &mut QuakeClassSpawnView) -> anyhow::Result<()> {
	trenchbroom_gltf_rotation_fix(view.entity);
	spawn_class_model_internal::<T>(view, Some("Scene0"));
	Ok(())
}
