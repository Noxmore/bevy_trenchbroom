use std::collections::hash_map::Entry;

use bevy::asset::{AssetLoader, AsyncReadExt};
use brush::{generate_mesh_from_brush_polygons, BrushSurfacePolygon, ConvexHull};
use class::QuakeClassType;
use config::TextureLoadView;
use geometry::{BrushList, Brushes, GeometryProviderMeshView, MapGeometryTexture};

use super::*;

pub struct QuakeMapLoader {
	pub tb_server: TrenchBroomServer,
}
impl FromWorld for QuakeMapLoader {
	fn from_world(world: &mut World) -> Self {
		Self {
			tb_server: world.resource::<TrenchBroomServer>().clone(),
		}
	}
}
impl AssetLoader for QuakeMapLoader {
	type Asset = QuakeMap;
	type Settings = ();
	type Error = anyhow::Error;

	fn load(
		&self,
		reader: &mut dyn bevy::asset::io::Reader,
		_settings: &Self::Settings,
		load_context: &mut bevy::asset::LoadContext,
	) -> impl bevy::utils::ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
		Box::pin(async {
			let mut input = String::new();
			reader.read_to_string(&mut input).await?;

			let quake_util_map = quake_util::qmap::parse(&mut io::Cursor::new(input))?;
			let mut entities = QuakeMapEntities::from_quake_util(quake_util_map, &self.tb_server.config);

			let mut mesh_handles = Vec::new();
			let mut brush_lists = HashMap::new();

			let mut world = World::new();

			// Handle origin brushes
			for map_entity in entities.iter_mut() {
				let origin_point = map_entity
					.brushes
					.iter()
					.find(|brush| {
						brush
							.surfaces
							.iter()
							.all(|surface| self.tb_server.config.origin_textures.contains(&surface.texture))
					})
					.map(|brush| self.tb_server.config.from_bevy_space_f64(brush.center()).as_vec3());

				if let Some(origin_point) = origin_point {
					map_entity.properties.insert("origin".s(), origin_point.fgd_to_string());
				}
			}

			for (map_entity_idx, map_entity) in entities.iter().enumerate() {
				let Some(classname) = map_entity.properties.get("classname") else { continue };
				let Some(class) = self.tb_server.config.get_class(classname) else {
					if !self.tb_server.config.suppress_invalid_entity_definitions {
						error!("No class found for classname `{classname}` on entity {map_entity_idx}");
					}

					continue;
				};

				let mut entity = world.spawn_empty();
				let entity_id = entity.id();

				class
					.apply_spawn_fn_recursive(&self.tb_server.config, map_entity, &mut entity)
					.map_err(|err| anyhow!("spawning entity {map_entity_idx} ({classname}): {err}"))?;

				if let QuakeClassType::Solid(geometry_provider) = class.info.ty {
					let geometry_provider = geometry_provider();

					let mut grouped_polygons: HashMap<&str, Vec<BrushSurfacePolygon>> = default();
					let mut texture_size_cache: HashMap<&str, UVec2> = default();
					let mut material_cache: HashMap<&str, Handle<GenericMaterial>> = default();

					for brush in &map_entity.brushes {
						for polygon in brush.polygonize() {
							grouped_polygons.entry(&polygon.surface.texture).or_default().push(polygon);
						}
					}

					let mut meshes = Vec::with_capacity(grouped_polygons.len());

					for (texture, polygons) in grouped_polygons {
						if self.tb_server.config.auto_remove_textures.contains(texture) {
							continue;
						}

						let texture_size = *match texture_size_cache.entry(texture) {
							Entry::Occupied(x) => x.into_mut(),
							Entry::Vacant(x) => x.insert(
								load_context
									.loader()
									.immediate()
									.load::<Image>(
										self.tb_server
											.config
											.material_root
											.join(format!("{}.{}", &polygons[0].surface.texture, self.tb_server.config.texture_extension)),
									)
									.await
									.map(|image: bevy::asset::LoadedAsset<Image>| image.take().size())
									.unwrap_or(UVec2::splat(1)),
							),
						};

						let material = match material_cache.entry(texture) {
							Entry::Occupied(x) => x.into_mut(),
							Entry::Vacant(x) => x.insert(
								(self.tb_server.config.load_loose_texture)(TextureLoadView {
									name: texture,
									tb_config: &self.tb_server.config,
									load_context,
									entities: &entities,
									alpha_mode: None,
									embedded_textures: None,
								})
								.await,
							),
						}
						.clone();

						let mut mesh = generate_mesh_from_brush_polygons(&polygons, &self.tb_server.config, texture_size);

						if let Ok(origin_point) = map_entity.get::<Vec3>("origin") {
							mesh = mesh.translated_by(self.tb_server.config.to_bevy_space(-origin_point));
						}

						let mesh_entity = world.spawn(Name::new(texture.s())).id();
						world.entity_mut(entity_id).add_child(mesh_entity);

						meshes.push((
							mesh_entity,
							mesh,
							MapGeometryTexture {
								name: texture.s(),
								material,
								#[cfg(feature = "bevy_pbr")]
								lightmap: None,
								flags: BspTexFlags::Normal,
							},
						));
					}

					let mesh_views = meshes
						.iter_mut()
						.map(|(entity, mesh, texture)| GeometryProviderMeshView {
							entity: *entity,
							mesh,
							texture,
						})
						.collect_vec();

					let mut view = GeometryProviderView {
						world: &mut world,
						entity: entity_id,
						tb_server: &self.tb_server,
						map_entity,
						map_entity_idx,
						meshes: mesh_views,
					};

					for provider in geometry_provider.providers {
						provider(&mut view);
					}

					(self.tb_server.config.global_geometry_provider)(&mut view);

					for (entity, mesh, _) in meshes {
						let handle = load_context.add_labeled_asset(format!("Mesh{}", mesh_handles.len()), mesh);

						world.entity_mut(entity).insert(Mesh3d(handle.clone()));

						mesh_handles.push(handle);
					}

					let brush_list_handle = load_context.add_labeled_asset(format!("Brushes{map_entity_idx}"), BrushList(map_entity.brushes.clone()));
					brush_lists.insert(map_entity_idx, brush_list_handle.clone());

					world.entity_mut(entity_id).insert(Brushes::Shared(brush_list_handle));
				}

				let mut entity = world.entity_mut(entity_id);

				(self.tb_server.config.global_spawner)(&self.tb_server.config, map_entity, &mut entity)
					.map_err(|err| anyhow!("spawning entity {map_entity_idx} ({classname}) with global spawner: {err}"))?;
			}

			Ok(QuakeMap {
				scene: load_context.add_labeled_asset("Scene".s(), Scene::new(world)),
				meshes: mesh_handles,
				brush_lists,
				entities,
			})
		})
	}

	fn extensions(&self) -> &[&str] {
		&["map"]
	}
}

#[cfg(feature = "bevy_pbr")]
#[test]
fn map_loading() {
	let mut app = App::new();

	// Can't find a better solution than this mess :(
	#[rustfmt::skip]
	app
		.add_plugins((AssetPlugin::default(), TaskPoolPlugin::default(), MaterializePlugin::new(TomlMaterialDeserializer)))
		.insert_resource(TrenchBroomServer::new(default()))
		.init_asset::<Image>()
		.init_asset::<StandardMaterial>()
		.init_asset::<Mesh>()
		.init_asset::<Scene>()
		.init_asset::<QuakeMap>()
		.init_asset_loader::<QuakeMapLoader>()
	;

	let map_handle = app.world().resource::<AssetServer>().load::<QuakeMap>("maps/example.map");

	for _ in 0..1000 {
		match app.world().resource::<AssetServer>().load_state(&map_handle) {
			bevy::asset::LoadState::Loaded => return,
			bevy::asset::LoadState::Failed(err) => panic!("{err}"),
			_ => std::thread::sleep(std::time::Duration::from_millis(5)),
		}

		app.update();
	}
	panic!("Map took longer than 5 seconds to load.");
}
