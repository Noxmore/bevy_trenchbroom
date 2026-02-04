use super::*;
use crate::{
	brush::{Brush, BrushSurface, BrushUV},
	util::TextureSizeCache,
	*,
};
use bevy_mesh::{Indices, PrimitiveTopology};
use bsp::*;
use qbsp::data::{
	BspLeaf, BspNodeRef, ModelBrushes,
	texture::{EmbeddedTextureName, TextureName},
};

#[derive(Default)]
pub struct InternalModel {
	pub meshes: Vec<InternalModelMesh>,
	/// Entity to apply [`Brushes`] to. Should probably only be one of these.
	pub entity: Option<Entity>,
}

// We need to run spawners before adding model assets because they have mutable access to meshes
pub struct InternalModelMesh {
	pub texture: MapGeometryTexture,
	pub mesh: Mesh,
	/// Entity to apply [`Mesh3d`] to. Should probably only be one of these.
	pub entity: Option<Entity>,
}

#[cfg(feature = "client")]
type Lightmap = BspLightmap;
#[cfg(not(feature = "client"))]
type Lightmap = LightmapUvMap;

pub async fn compute_models<'a, 'lc: 'a>(
	ctx: &mut BspLoadCtx<'a, 'lc>,
	lightmap: &Option<Lightmap>,
	embedded_textures: &EmbeddedTextures,
) -> Vec<InternalModel> {
	let config = &ctx.loader.tb_server.config;
	#[cfg(feature = "client")]
	let lightmap_uvs = lightmap.as_ref().map(|lm| &lm.uv_map);
	#[cfg(not(feature = "client"))]
	let lightmap_uvs = lightmap.as_ref();

	let mut models = Vec::with_capacity(ctx.data.models.len());

	let mut texture_size_cache: TextureSizeCache<TextureName> = default();

	for model_idx in 0..ctx.data.models.len() {
		let model_output = ctx.data.mesh_model(model_idx, lightmap_uvs);
		let mut model = InternalModel::default();
		model.meshes.reserve(model_output.meshes.len());

		for mut exported_mesh in model_output.meshes {
			// Check if we have to scale the UVs ourselves.
			if let Some(texture_name) = exported_mesh.texture
				&& !exported_mesh.prescaled_uvs
			{
				let texture_size = texture_size_cache
					.entry(texture_name, ctx.load_context, &ctx.loader.tb_server.config)
					.await;

				for uv in &mut exported_mesh.uvs {
					*uv /= texture_size.as_vec2();
				}
			}

			for position in &mut exported_mesh.positions {
				*position = config.to_bevy_space(*position);
			}

			for normal in &mut exported_mesh.normals {
				*normal = normal.trenchbroom_to_bevy();
			}

			let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, config.brush_mesh_asset_usages);

			mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, exported_mesh.positions);
			mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, exported_mesh.normals);
			mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, exported_mesh.uvs);
			if let Some(lightmap_uvs) = &exported_mesh.lightmap_uvs {
				mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, lightmap_uvs.iter().map(qbsp::glam::Vec2::to_array).collect_vec());
			}
			mesh.insert_indices(Indices::U32(exported_mesh.indices.into_flattened()));

			// Servers don't care about things like normal maps.
			#[cfg(feature = "client")]
			if let Some(mut tangents) = exported_mesh.tangents {
				for tangent in &mut tangents {
					*tangent = tangent.xyz().trenchbroom_to_bevy().extend(tangent.w);
				}

				mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
			} else if let Err(err) = mesh.generate_tangents() {
				error!(
					"Failed to generate tangents for model {model_idx}, mesh with texture {:?}: {err}",
					exported_mesh.texture
				);
			}

			let material = if let Some(texture_name) = exported_mesh.texture {
				let embedded_texture = texture_name
					.truncate() // Truncate down the Quake 1 texture length if we can
					.and_then(|texture_name: EmbeddedTextureName| embedded_textures.textures.get(&texture_name));

				match embedded_texture {
					Some(embedded_texture) => embedded_texture.material.clone(),
					None => {
						(config.load_loose_texture)(TextureLoadView {
							name: texture_name.as_str(),
							tb_server: &ctx.loader.tb_server,
							load_context: ctx.load_context,
							asset_server: ctx.asset_server,
							entities: ctx.entities,
							#[cfg(feature = "client")]
							alpha_mode: None,
							embedded_textures: Some(&embedded_textures.images),
						})
						.await
					}
				}
			} else {
				ctx.loader.tb_server.missing_material.read().clone()
			};

			model.meshes.push(InternalModelMesh {
				texture: MapGeometryTexture {
					material,
					#[cfg(feature = "client")]
					lightmap: lightmap.as_ref().map(|lm| lm.animated_lighting.clone()),
					name: exported_mesh.texture.as_ref().map(ToString::to_string),
					flags: exported_mesh.tex_flags,
				},
				mesh,
				entity: None,
			});
		}

		models.push(model)
	}

	models
}

fn model_brushes_to_brush_hull(model_brushes: &ModelBrushes, config: &TrenchBroomConfig) -> Vec<BrushHull> {
	model_brushes
		.brushes
		.iter()
		.map(|model_brush| {
			let min = config.to_bevy_space(model_brush.bound.min).as_dvec3();
			let max = config.to_bevy_space(model_brush.bound.max).as_dvec3();
			// Something about the conversion makes the asserts below fail, this ensures they don't.
			let (min, max) = ((-min).min(-max), (-min).max(-max));
			debug_assert!(min.x < max.x);
			debug_assert!(min.y < max.y);
			debug_assert!(min.z < max.z);

			let mut brush = BrushHull::default();
			brush.planes.reserve(4 + model_brush.planes.len());

			#[rustfmt::skip]
			brush.planes.extend([
				BrushPlane { normal: DVec3::Y,     distance:  min.y },
				BrushPlane { normal: DVec3::NEG_Y, distance: -max.y },
				BrushPlane { normal: DVec3::X,     distance:  min.x },
				BrushPlane { normal: DVec3::NEG_X, distance: -max.x },
				BrushPlane { normal: DVec3::Z,     distance:  min.z },
				BrushPlane { normal: DVec3::NEG_Z, distance: -max.z },
			]);

			brush.planes.extend(model_brush.planes.iter().map(|plane| {
				// We need to invert it because brush math expects normals to point inwards
				BrushPlane {
					normal: plane.normal.as_dvec3().trenchbroom_to_bevy(),
					distance: -plane.dist as f64 / config.scale as f64,
				}
			}));

			brush
		})
		.collect()
}

fn for_each_leaf(data: &BspData, node_ref: BspNodeRef, f: &mut impl FnMut(&BspLeaf)) {
	match node_ref {
		BspNodeRef::Node(idx) => {
			let node = &data.nodes[idx as usize];
			for_each_leaf(data, *node.front, f);
			for_each_leaf(data, *node.back, f);
		}
		BspNodeRef::Leaf(idx) => {
			f(&data.leaves[idx as usize]);
		}
	}
}

pub fn finalize_models(ctx: &mut BspLoadCtx, internal_models: Vec<InternalModel>, world: &mut World) -> anyhow::Result<Vec<BspModel>> {
	let config = &ctx.loader.tb_server.config;

	let mut models = Vec::with_capacity(internal_models.len());

	for (model_idx, model) in internal_models.into_iter().enumerate() {
		models.push(BspModel {
			meshes: model
				.meshes
				.into_iter()
				.enumerate()
				.map(|(mesh_idx, model_mesh)| {
					let mesh_handle = ctx
						.load_context
						.add_labeled_asset(format!("Model{model_idx}Mesh{mesh_idx}"), model_mesh.mesh);

					if let Some(mesh_entity) = model_mesh.entity {
						world.entity_mut(mesh_entity).insert(Mesh3d(mesh_handle.clone()));
					}

					(model_mesh.texture.name, mesh_handle)
				})
				.collect(),

			brushes: match &ctx.data.bspx.brush_list {
				Some(brush_list) => brush_list
					.iter()
					.find(|model_brushes| model_brushes.model_idx as usize == model_idx)
					.map(|model_brushes| {
						let brushes_asset = ctx.load_context.add_labeled_asset(
							format!("Model{model_idx}Brushes"),
							BrushHullsAsset(model_brushes_to_brush_hull(model_brushes, config)),
						);

						if let Some(entity) = model.entity {
							world.entity_mut(entity).insert(Brushes::Bsp(brushes_asset.clone()));
						}

						GenericBrushListHandle::Hulls(brushes_asset)
					}),
				None => {
					let bsp_model = &ctx.data.models[model_idx];
					let mut brushes: Vec<Brush> = Vec::new();

					for_each_leaf(ctx.data, bsp_model.hulls.root, &mut |leaf| {
						let Some(leaf_brushes) = &leaf.leaf_brushes else { return };

						for leaf_brush_idx in leaf_brushes.idx.0..leaf_brushes.idx.0 + leaf_brushes.num.0 {
							let bsp_brush = &ctx.data.brushes[ctx.data.leaf_brushes[leaf_brush_idx as usize].0 as usize];
							let mut brush = Brush::default();

							for brush_side_idx in bsp_brush.first_side..bsp_brush.first_side + bsp_brush.num_sides {
								let brush_side = &ctx.data.brush_sides[brush_side_idx as usize];
								let plane = &ctx.data.planes[brush_side.plane_idx.0 as usize];
								let tex_info = &ctx.data.tex_info[brush_side.tex_info_idx.0 as usize];

								brush.surfaces.push(BrushSurface {
									plane: BrushPlane {
										normal: plane.normal.as_dvec3().trenchbroom_to_bevy(),
										distance: -plane.dist as f64 / config.scale as f64,
									},
									texture: ctx
										.data
										.get_texture_name(tex_info)
										.expect("We couldn't get the texture name here. If this isn't a corrupted BSP, report this!")
										.to_string(),
									uv: BrushUV {
										offset: vec2(tex_info.projection.u_offset, tex_info.projection.v_offset),
										rotation: 0.,
										scale: Vec2::ONE,
										axes: Some([tex_info.projection.u_axis.as_dvec3(), tex_info.projection.v_axis.as_dvec3()]),
									},
								});
							}

							brushes.push(brush);
						}
					});

					if brushes.is_empty() {
						None
					} else {
						let handle = ctx
							.load_context
							.add_labeled_asset(format!("Model{model_idx}Brushes"), BrushesAsset(brushes));

						if let Some(entity) = model.entity {
							world.entity_mut(entity).insert(Brushes::Shared(handle.clone()));
						}

						Some(GenericBrushListHandle::Brushes(handle))
					}
				}
			},
		});
	}

	Ok(models)
}
