#[cfg(feature = "client")]
mod irradiance_volume;
#[cfg(feature = "client")]
pub use irradiance_volume::IrradianceVolumeMultipliers;
#[cfg(feature = "client")]
mod lightmap;
mod models;
mod scene;
mod textures;

use bevy::asset::{AssetLoader, LoadContext};
use bsp::*;
#[cfg(feature = "client")]
use irradiance_volume::load_irradiance_volume;
#[cfg(feature = "client")]
use lightmap::BspLightmap;
use models::{compute_models, finalize_models};
use qmap::QuakeMapEntities;
use scene::initialize_scene;
use textures::EmbeddedTextures;

use crate::*;

pub(crate) struct BspLoadCtx<'a, 'lc: 'a> {
	pub loader: &'a BspLoader,
	pub load_context: &'a mut LoadContext<'lc>,
	pub asset_server: &'a AssetServer,
	pub data: &'a BspData,
	pub entities: &'a QuakeMapEntities,
}

pub struct BspLoader {
	pub tb_server: TrenchBroomServer,
	pub asset_server: AssetServer,
}
impl FromWorld for BspLoader {
	fn from_world(world: &mut World) -> Self {
		Self {
			tb_server: world.resource::<TrenchBroomServer>().clone(),
			asset_server: world.resource::<AssetServer>().clone(),
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
	) -> impl bevy::utils::ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
		Box::pin(async move {
			let mut bytes = Vec::new();
			reader.read_to_end(&mut bytes).await?;

			let lit = load_context.read_asset_bytes(load_context.path().with_extension("lit")).await.ok();

			let data = BspData::parse(BspParseInput {
				bsp: &bytes,
				lit: lit.as_deref(),
				settings: self.tb_server.config.bsp_parse_settings.clone(),
			})?;

			let quake_util_map =
				quake_util::qmap::parse(&mut io::Cursor::new(data.entities.as_bytes())).map_err(|err| anyhow!("Parsing entities: {err}"))?;
			let entities = QuakeMapEntities::from_quake_util(quake_util_map, &self.tb_server.config);

			let mut ctx = BspLoadCtx {
				loader: self,
				load_context,
				asset_server: &self.asset_server,
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

#[cfg(feature = "client")]
#[test]
fn bsp_loading() {
	let mut app = App::new();

	// Can't find a better solution than this mess :(
	#[rustfmt::skip]
	app
		.add_plugins((AssetPlugin::default(), TaskPoolPlugin::default(), bevy::time::TimePlugin, MaterializePlugin::new(TomlMaterialDeserializer)))
		.insert_resource(TrenchBroomServer::new(default()))
		.init_asset::<Image>()
		.init_asset::<StandardMaterial>()
		.init_asset::<AnimatedLighting>()
		.init_asset::<Mesh>()
		.init_asset::<BspBrushesAsset>()
		.init_asset::<Scene>()
		.init_asset::<Bsp>()
		.init_asset_loader::<BspLoader>()
	;

	let bsp_handle = app.world().resource::<AssetServer>().load::<Bsp>("maps/example.bsp");

	for _ in 0..1000 {
		match app.world().resource::<AssetServer>().load_state(&bsp_handle) {
			bevy::asset::LoadState::Loaded => return,
			bevy::asset::LoadState::Failed(err) => panic!("{err}"),
			_ => std::thread::sleep(std::time::Duration::from_millis(5)),
		}

		app.update();
	}
	panic!("Bsp took longer than 5 seconds to load.");
}
