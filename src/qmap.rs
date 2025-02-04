use bevy::asset::{AssetLoader, AsyncReadExt};
use brush::{generate_mesh_from_brush_polygons, Brush, BrushSurfacePolygon};
use class::QuakeClassType;
use config::TextureLoadView;
use fgd::FgdType;
use geometry::{BrushList, Brushes, GeometryProviderMeshView, MapGeometryTexture};

use crate::*;

/// Quake map loaded from a .map file.
#[derive(Reflect, Asset, Debug, Clone)]
pub struct QuakeMap {
	pub scene: Handle<Scene>,
	pub meshes: Vec<Handle<Mesh>>,
	/// Maps from entity indexes to brush lists.
	pub brush_lists: HashMap<usize, Handle<BrushList>>,
	pub entities: QuakeMapEntities,
}

/// All the entities stored in a quake map, whether [QuakeMap] or [Bsp](bsp::Bsp).
#[derive(Reflect, Debug, Clone, Default, Deref, DerefMut)]
pub struct QuakeMapEntities(pub Vec<QuakeMapEntity>);
impl QuakeMapEntities {
	pub fn from_quake_util(qmap: quake_util::qmap::QuakeMap, config: &TrenchBroomConfig) -> Self {
		let mut entities = Self::default();
		entities.reserve(qmap.entities.len());

		for entity in qmap.entities {
			let properties = entity
				.edict
				.into_iter()
				.map(|(k, v)| (k.to_string_lossy().into(), v.to_string_lossy().into()))
				.collect::<HashMap<String, String>>();

			entities.push(QuakeMapEntity {
				properties,
				brushes: entity.brushes.iter().map(|brush| Brush::from_quake_util(brush, config)).collect(),
			});
		}

		entities
	}

	/// Gets the worldspawn of this map, this will return `Some` on any valid map.
	///
	/// worldspawn should be the first entity, so normally this will be an `O(1)` operation
	pub fn worldspawn(&self) -> Option<&QuakeMapEntity> {
		self.iter().find(|ent| ent.classname() == Ok("worldspawn"))
	}
}

/// A single entity from a quake map, containing the entities property map, and optionally, brushes.
#[derive(Reflect, Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuakeMapEntity {
	/// The properties defined in this entity instance.
	pub properties: HashMap<String, String>,
	pub brushes: Vec<Brush>,
}

impl QuakeMapEntity {
	/// Gets the classname of the entity, on any valid entity, this will return `Ok`. Otherwise it will return [QuakeEntityError::RequiredPropertyNotFound].
	pub fn classname(&self) -> Result<&str, QuakeEntityError> {
		self.properties
			.get("classname")
			.map(String::as_str)
			.ok_or_else(|| QuakeEntityError::RequiredPropertyNotFound {
				property: "classname".into(),
			})
	}

	/// Helper function to try to parse an [FgdType] property from this map entity.
	pub fn get<T: FgdType>(&self, key: &str) -> Result<T, QuakeEntityError> {
		let s = self
			.properties
			.get(key)
			.ok_or_else(|| QuakeEntityError::RequiredPropertyNotFound { property: key.s() })?;

		T::fgd_parse(s).map_err(|err| QuakeEntityError::PropertyParseError {
			property: key.s(),
			required_type: type_name::<T>(),
			error: format!("{err}"),
		})
	}
}

#[derive(Error, Debug, Clone, PartialEq)]
pub enum QuakeEntityError {
	#[error("required property `{property}` not found")]
	RequiredPropertyNotFound { property: String },
	#[error("requires property `{property}` to be a valid `{required_type}`. Error: {error}")]
	PropertyParseError {
		property: String,
		required_type: &'static str,
		error: String,
	},
	#[error("definition for \"{classname}\" not found")]
	DefinitionNotFound { classname: String },
	#[error("Entity class {classname} has a base of {base_name}, but that class does not exist")]
	InvalidBase { classname: String, base_name: String },
}

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
	// TODO this should be some asset version of QuakeMap
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
			let entities = QuakeMapEntities::from_quake_util(quake_util_map, &self.tb_server.config);

			let mut mesh_handles = Vec::new();
			let mut brush_lists = HashMap::new();

			let mut world = World::new();

			for (map_entity_idx, map_entity) in entities.iter().enumerate() {
				let Some(classname) = map_entity.properties.get("classname") else { continue };
				let Some(class) = self.tb_server.config.get_class(classname) else {
					if !self.tb_server.config.ignore_invalid_entity_definitions {
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

						let texture_size = *texture_size_cache.entry(texture).or_insert_with(|| {
							// Have to because this is not an async context, and it's simpler than expanding or_insert_with
							smol::block_on(async {
								load_context
									.loader()
									.immediate()
									.load::<Image>(
										self.tb_server
											.config
											.texture_root
											.join(format!("{}.{}", &polygons[0].surface.texture, self.tb_server.config.texture_extension)),
									)
									.await
							})
							.map(|image: bevy::asset::LoadedAsset<Image>| image.take().size())
							.unwrap_or(UVec2::splat(1))
						});

						let material = material_cache
							.entry(texture)
							.or_insert_with(|| {
								(self.tb_server.config.load_loose_texture)(TextureLoadView {
									name: texture,
									tb_config: &self.tb_server.config,
									load_context,
									entities: &entities,
									alpha_mode: None,
									embedded_textures: None,
								})
							})
							.clone();

						let mesh = generate_mesh_from_brush_polygons(&polygons, &self.tb_server.config, texture_size);

						let mesh_entity = world.spawn(Name::new(texture.s())).id();
						world.entity_mut(entity_id).add_child(mesh_entity);

						meshes.push((
							mesh_entity,
							mesh,
							MapGeometryTexture {
								name: texture.s(),
								material,
								lightmap: None,
								special: false,
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
