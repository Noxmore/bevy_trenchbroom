use bevy::{
	image::{ImageAddressMode, ImageSamplerDescriptor},
	prelude::*,
	render::{RenderApp, RenderStartup, init_gpu_resource, render_resource::DefaultImageSamplerDescriptor, texture::DefaultImageSampler},
};

pub struct RepeatDefaultSamplerPlugin;
impl Plugin for RepeatDefaultSamplerPlugin {
	fn build(&self, app: &mut App) {
		let Some(mut sampler_descriptor) = app
			.get_added_plugins::<ImagePlugin>()
			.first()
			.map(|plugin| plugin.default_sampler.clone())
		else {
			return;
		};
		let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };

		sampler_descriptor.set_address_mode(ImageAddressMode::Repeat);

		render_app
			.insert_resource(RepeatedDefaultSamplerDescriptor(sampler_descriptor))
			// bevy 0.19 builds the GPU `DefaultImageSampler` from `DefaultImageSamplerDescriptor`
			// in `init_gpu_resource::<DefaultImageSampler>` (a `RenderStartup` system). We must
			// overwrite the descriptor *before* that runs, otherwise the sampler is created with
			// the original (clamping) address mode and texture repeating breaks (#161).
			.add_systems(
				RenderStartup,
				Self::repeat_default_sampler.before(init_gpu_resource::<DefaultImageSampler>),
			);
	}
}
impl RepeatDefaultSamplerPlugin {
	fn repeat_default_sampler(sampler_descriptor: Res<RepeatedDefaultSamplerDescriptor>, mut default_sampler: ResMut<DefaultImageSamplerDescriptor>) {
		default_sampler.0 = sampler_descriptor.0.clone();
	}
}

#[derive(Resource)]
struct RepeatedDefaultSamplerDescriptor(ImageSamplerDescriptor);
