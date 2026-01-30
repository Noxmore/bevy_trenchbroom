use bevy::{
	image::{ImageAddressMode, ImageSamplerDescriptor},
	prelude::*,
	render::{RenderApp, RenderStartup, renderer::RenderDevice, texture::DefaultImageSampler},
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
			.add_systems(RenderStartup, Self::repeat_default_sampler);
	}
}
impl RepeatDefaultSamplerPlugin {
	fn repeat_default_sampler(
		sampler_descriptor: Res<RepeatedDefaultSamplerDescriptor>,
		render_device: Res<RenderDevice>,
		mut default_sampler: ResMut<DefaultImageSampler>,
	) {
		**default_sampler = render_device.create_sampler(&sampler_descriptor.0.as_wgpu());
	}
}

#[derive(Resource)]
struct RepeatedDefaultSamplerDescriptor(ImageSamplerDescriptor);
