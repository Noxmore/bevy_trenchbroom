#[cfg(feature = "client")]
mod irradiance_volume;
#[cfg(feature = "client")]
pub use irradiance_volume::IrradianceVolumeMultipliers;
#[cfg(feature = "client")]
mod lightmap;
mod models;
mod scene;
mod textures;

use bevy::{
	asset::{AssetLoader, LoadContext},
	tasks::ConditionalSendFuture,
};
use bsp::*;
#[cfg(feature = "client")]
use irradiance_volume::load_irradiance_volume;
#[cfg(feature = "client")]
use lightmap::BspLightmap;
use models::{compute_models, finalize_models};
use qmap::QuakeMapEntities;
use scene::initialize_scene;
use textures::EmbeddedTextures;

use crate::{class::scene_systems::SceneSchedules, *};

pub(crate) struct BspLoadCtx<'a, 'lc: 'a> {
	pub loader: &'a BspLoader,
	pub load_context: &'a mut LoadContext<'lc>,
	pub asset_server: &'a AssetServer,
	pub type_registry: &'a AppTypeRegistry,
	pub data: &'a BspData,
	pub entities: &'a QuakeMapEntities,
}

#[derive(TypePath)]
pub struct BspLoader {
	pub tb_server: TrenchBroomServer,
	pub asset_server: AssetServer,
	pub type_registry: AppTypeRegistry,
	pub scene_schedules: SceneSchedules,
}
impl FromWorld for BspLoader {
	fn from_world(world: &mut World) -> Self {
		Self {
			tb_server: world.resource::<TrenchBroomServer>().clone(),
			asset_server: world.resource::<AssetServer>().clone(),
			type_registry: world.resource::<AppTypeRegistry>().clone(),
			scene_schedules: world.resource::<SceneSchedules>().clone(),
		}
	}
}

impl AssetLoader for BspLoader {
	type Asset = Bsp;
	type Error = anyhow::Error;
	type Settings = ();

	fn load(
		&self,
		reader: &mut dyn bevy::asset::io::Reader,
		_settings: &Self::Settings,
		load_context: &mut LoadContext,
	) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
		Box::pin(async move {
			let mut bytes = Vec::new();
			reader.read_to_end(&mut bytes).await?;

			let lit = load_context.read_asset_bytes(load_context.path().path().with_extension("lit")).await.ok();

			let data = BspData::parse(BspParseInput {
				bsp: &bytes,
				lit: lit.as_deref(),
				settings: self.tb_server.config.bsp_parse_settings.clone(),
			})?;

			let fixed_entities_lump = qbsp::util::quake_string_to_utf8(&data.entities, "\\<b>", "\\</b>");

			let quake_util_map =
				quake_util::qmap::parse(&mut io::Cursor::new(fixed_entities_lump)).map_err(|err| anyhow!("Parsing entities: {err}"))?;
			let entities = QuakeMapEntities::from_quake_util(quake_util_map, &self.tb_server.config);

			let mut ctx = BspLoadCtx {
				loader: self,
				load_context,
				asset_server: &self.asset_server,
				type_registry: &self.type_registry,
				data: &data,
				entities: &entities,
			};

			let embedded_textures = EmbeddedTextures::setup(&mut ctx).await?;

			#[cfg(feature = "client")]
			let lightmap = BspLightmap::compute(&mut ctx)?;
			#[cfg(not(feature = "client"))]
			let lightmap = None;

			let mut models = compute_models(&mut ctx, &lightmap, &embedded_textures).await;

			let embedded_textures = embedded_textures.finalize(&mut ctx);

			let mut world = initialize_scene(&mut ctx, &mut models)?;

			let bsp_models = finalize_models(&mut ctx, models, &mut world)?;

			#[cfg(feature = "client")]
			let irradiance_volume = load_irradiance_volume(&mut ctx, &mut world)?;

			Ok(Bsp {
				scene: load_context.add_labeled_asset("Scene".s(), Scene::new(world)),
				embedded_textures,
				#[cfg(feature = "client")]
				lightmap: lightmap.map(|lm| lm.animated_lighting),
				#[cfg(feature = "client")]
				irradiance_volume,
				models: bsp_models,

				data,
				entities,
			})
		})
	}

	fn extensions(&self) -> &[&str] {
		&["bsp"]
	}
}

#[cfg(test)]
mod tests {
	#[allow(unused)]
	use super::*;

	#[cfg(feature = "client")]
	#[test]
	fn bsp_loading() {
		let mut app = App::new();

		// Can't find a better solution than this mess :(
		#[rustfmt::skip]
		app
			.add_plugins((
				AssetPlugin::default(),
				TaskPoolPlugin::default(),
				bevy::time::TimePlugin,
				MaterializePlugin::new(TomlMaterialDeserializer),
				ImagePlugin::default(),
			))
			.init_asset::<Image>()
			.init_asset::<StandardMaterial>()
			.add_plugins(
				CorePlugin(
					TrenchBroomConfig::default()
						.suppress_invalid_entity_definitions(true)
				)
			)
			.init_asset::<AnimatedLighting>()
			.init_asset::<Mesh>()
			.init_asset::<BrushHullsAsset>()
			.init_asset::<BrushesAsset>()
			.init_asset::<Scene>()
			.init_asset::<Bsp>()
			.init_asset_loader::<BspLoader>()
		;

		smol::block_on(async {
			app.world()
				.resource::<AssetServer>()
				.load_untyped_async("maps/example.bsp")
				.await
				.unwrap();
		});
	}
}
