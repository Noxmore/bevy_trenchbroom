use bevy::{image::ImageSampler, prelude::*};
use bevy_materialize::prelude::*;

use crate::{geometry::MapGeometry, TrenchBroomServer};

use super::ConfigPlugin;

impl ConfigPlugin {
	/// Sets the samplers of any images within the materials within entities with [`MapGeometry`] components to [`TrenchBroomConfig::texture_sampler`](super::TrenchBroomConfig::texture_sampler).
	/// 
	/// This is pretty hacky, but i can't think of a better solution. Oh well!
	pub fn set_image_samplers(
		mut commands: Commands,
		map_geometry_query: Query<&GenericMaterial3d, With<MapGeometry>>,
		generic_materials: Res<Assets<GenericMaterial>>,
		mut image_events: MessageReader<AssetEvent<Image>>,
		asset_server: Res<AssetServer>,
		tb_server: Res<TrenchBroomServer>,
	) {
		if matches!(tb_server.config.texture_sampler, ImageSampler::Default) {
			return;
		}
		
		if image_events.read().all(|event| {
			!matches!(
				event,
				AssetEvent::Added { .. } | AssetEvent::LoadedWithDependencies { .. }
			)
		}) {
			return;
		}
	
		for generic_material_3d in &map_geometry_query {
			if asset_server.load_state(&generic_material_3d.0).is_loading() {
				warn!(
					"Generic material {:?} in map geometry is still loading. It probably wasn't a dependency of the map, this might cause problems!",
					generic_material_3d.0
				);
			}
			let Some(generic_material) = generic_materials.get(&generic_material_3d.0) else {
				continue;
			};
	
			let handle = generic_material.handle.clone();
	
			commands.queue(move |world: &mut World| {
				let id = handle.id();
				// We use asset_scope_mut instead of asset_scope, not because we need mutable access to the material—we don't—but because
				// mutably accessing the material causes the images to update their samplers, when sometimes they randomly didn't.
				// TODO: This is a Bevy bug, and i have no idea why this fixes it.
				handle.asset_scope_mut(
					world,
					Box::new(move |world, material| {
						let Some(material) = material else {
							warn!("Material {id} within GenericMaterial for map geometry isn't loaded.");
							return;
						};
	
						let dyn_struct = material.reflect_ref().as_struct().unwrap(); // ErasedMaterial requires Struct
	
						let sampler = world.resource::<TrenchBroomServer>().config.texture_sampler.clone();
	
						for field in dyn_struct.iter_fields() {
							let Some(image_handle) = field.try_downcast_ref::<Handle<Image>>().map(Handle::id).or_else(|| {
								field
									.try_downcast_ref::<Option<Handle<Image>>>()
									.and_then(|option| option.as_ref().map(Handle::id))
							}) else {
								continue;
							};
	
							// First we check before mutating it to avoid a feedback loop, doing this every frame
							let Some(image) = world.resource::<Assets<Image>>().get(image_handle) else {
								continue;
							};
	
							if image.sampler == sampler {
								continue;
							}
	
							let mut image_assets = world.resource_mut::<Assets<Image>>();
							let Some(image) = image_assets.get_mut(image_handle) else {
								continue;
							};
	
							image.sampler = sampler.clone();
						}
					}),
				);
			});
		}
	}
}
