use bevy::{
	asset::{AssetLoader, AsyncReadExt},
	platform::collections::hash_map::Entry,
	tasks::ConditionalSendFuture,
};
use brush::{BrushSurfacePolygon, ConvexHull, generate_mesh_from_brush_polygons};
use config::TextureLoadView;
use geometry::{BrushList, Brushes, MapGeometryTexture};

use crate::{
	class::{QuakeClassMeshView, QuakeClassSpawnView, generate_class_map},
	geometry::MapGeometry,
};

use super::*;

pub struct QuakeMapLoader {
	pub asset_server: AssetServer,
	pub tb_server: TrenchBroomServer,
	pub type_registry: AppTypeRegistry,
}
impl FromWorld for QuakeMapLoader {
	fn from_world(world: &mut World) -> Self {
		Self {
			asset_server: world.resource::<AssetServer>().clone(),
			tb_server: world.resource::<TrenchBroomServer>().clone(),
			type_registry: world.resource::<AppTypeRegistry>().clone(),
		}
	}
}
impl QuakeMapLoader {
	/// The type registry cannot be read in an async context because the read lock doesn't implement Send. (Still not really sure why)
	/// And so, we have to use a non-async function to do this.
	pub fn generate_class_map(&self) -> HashMap<&'static str, &'static class::ErasedQuakeClass> {
		generate_class_map(&self.type_registry.read())
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
	) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
		Box::pin(async move {
			let mut input = String::new();
			reader.read_to_string(&mut input).await?;

			let quake_util_map = quake_util::qmap::parse(&mut io::Cursor::new(input))?;
			let mut entities = QuakeMapEntities::from_quake_util(quake_util_map, &self.tb_server.config);

			let mut mesh_handles = Vec::new();
			let mut brush_lists = HashMap::default();

			let mut world = World::new();

			// Handle origin brushes
			for map_entity in entities.iter_mut() {
				let origin_point = map_entity
					.brushes
					.iter()
					.enumerate()
					.find(|(_, brush)| {
						brush
							.surfaces
							.iter()
							.all(|surface| self.tb_server.config.origin_textures.contains(&surface.texture))
					})
					.map(|(brush_idx, brush)| (brush_idx, self.tb_server.config.from_bevy_space_f64(brush.center()).as_vec3()));

				if let Some((origin_brush_idx, origin_point)) = origin_point {
					map_entity.properties.insert("origin".s(), origin_point.fgd_to_string_unquoted());
					map_entity.brushes.remove(origin_brush_idx);
				}
			}

			let class_map = self.generate_class_map();

			for (map_entity_idx, map_entity) in entities.iter().enumerate() {
				let Some(classname) = map_entity.properties.get("classname") else { continue };
				let Some(class) = class_map.get(classname.as_str()).copied() else {
					if !self.tb_server.config.suppress_invalid_entity_definitions {
						error!("No class found for classname `{classname}` on entity {map_entity_idx}");
					}

					continue;
				};

				let entity = world.spawn_empty().id();

				let mut meshes = Vec::new();

				if class.info.ty.is_solid() {
					let mut grouped_polygons: HashMap<&str, Vec<BrushSurfacePolygon>> = default();
					let mut texture_size_cache: HashMap<&str, UVec2> = default();
					let mut material_cache: HashMap<&str, Handle<GenericMaterial>> = default();

					for brush in &map_entity.brushes {
						for polygon in brush.polygonize() {
							grouped_polygons.entry(&polygon.surface.texture).or_default().push(polygon);
						}
					}

					meshes.reserve(grouped_polygons.len());

					for (texture, polygons) in grouped_polygons {
						if self.tb_server.config.auto_remove_textures.contains(texture) {
							continue;
						}

						let texture_size = *match texture_size_cache.entry(texture) {
							Entry::Occupied(x) => x.into_mut(),
							Entry::Vacant(x) => x.insert('size_searcher: {
								for ext in &self.tb_server.config.texture_extensions {
									if let Ok(image) = load_context
										.loader()
										.immediate()
										.load::<Image>(
											self.tb_server
												.config
												.material_root
												.join(format!("{}.{}", &polygons[0].surface.texture, ext)),
										)
										.await
									{
										break 'size_searcher image.take().size();
									}
								}

								error!(
									"Failed to get size for texture {texture:?} looking for the following extensions: {:?}",
									self.tb_server.config.texture_extensions
								);
								UVec2::splat(1)
							}),
						};

						let material = match material_cache.entry(texture) {
							Entry::Occupied(x) => x.into_mut(),
							Entry::Vacant(x) => x.insert(
								(self.tb_server.config.load_loose_texture)(TextureLoadView {
									name: texture,
									tb_config: &self.tb_server.config,
									load_context,
									asset_server: &self.asset_server,
									entities: &entities,
									#[cfg(feature = "client")]
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

						let mesh_entity = world.spawn((Name::new(texture.s()), Transform::default())).id();

						meshes.push((
							mesh_entity,
							mesh,
							MapGeometryTexture {
								name: texture.s(),
								material,
								#[cfg(all(feature = "client", feature = "bsp"))]
								lightmap: None,
								#[cfg(feature = "bsp")]
								flags: BspTexFlags::Normal,
							},
						));
					}
				}

				let mut mesh_views = meshes
					.iter_mut()
					.map(|(entity, mesh, texture)| QuakeClassMeshView {
						entity: *entity,
						mesh,
						texture,
					})
					.collect_vec();

				let mut view = QuakeClassSpawnView {
					config: &self.tb_server.config,
					src_entity: map_entity,
					src_entity_idx: map_entity_idx,
					type_registry: &self.type_registry.read(),
					class_map: &class_map,
					class,
					world: &mut world,
					entity,
					load_context,
					meshes: &mut mesh_views,
				};

				class
					.apply_spawn_fn_recursive(&mut view)
					.map_err(|err| anyhow!("spawning entity {map_entity_idx} ({classname}): {err}"))?;

				(self.tb_server.config.global_spawner)(&mut view)
					.map_err(|err| anyhow!("spawning entity {map_entity_idx} ({classname}) with global spawner: {err}"))?;

				for (mesh_entity, mesh, _) in meshes {
					let handle = load_context.add_labeled_asset(format!("Mesh{}", mesh_handles.len()), mesh);

					// We add the children at the end to prevent the console flooding with warnings about broken Transform and Visibility hierarchies.
					world
						.entity_mut(mesh_entity)
						.insert((Mesh3d(handle.clone()), ChildOf(entity), MapGeometry));

					mesh_handles.push(handle);
				}

				let brush_list_handle = load_context.add_labeled_asset(format!("Brushes{map_entity_idx}"), BrushList(map_entity.brushes.clone()));
				brush_lists.insert(map_entity_idx, brush_list_handle.clone());

				world.entity_mut(entity).insert(Brushes::Shared(brush_list_handle));
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

#[cfg(test)]
mod tests {
	#[allow(unused)]
	use super::*;

	#[cfg(feature = "client")]
	#[test]
	fn map_loading() {
		let mut app = App::new();

		// Can't find a better solution than this mess :(
		#[rustfmt::skip]
		app
			.add_plugins((AssetPlugin::default(), TaskPoolPlugin::default(), bevy::time::TimePlugin))
			.insert_resource(TrenchBroomServer::new(
				TrenchBroomConfig::default()
					.suppress_invalid_entity_definitions(true)
			))
			.init_asset::<Image>()
			.init_asset::<StandardMaterial>()
			.init_asset::<Mesh>()
			.init_asset::<Scene>()
			.init_asset::<QuakeMap>()
			.init_asset_loader::<QuakeMapLoader>()
		;

		smol::block_on(async {
			app.world()
				.resource::<AssetServer>()
				.load_untyped_async("maps/example.map")
				.await
				.unwrap();
		});
	}
}
