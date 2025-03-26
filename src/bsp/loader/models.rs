use super::*;
use crate::*;
use bevy_mesh::{Indices, PrimitiveTopology};
use bsp::*;

#[derive(Default)]
pub struct InternalModel {
	pub meshes: Vec<InternalModelMesh>,
	/// Entity to apply [`Brushes`] to. Should probably only be one of these.
	pub entity: Option<Entity>,
}

// We need to run geometry providers before adding model assets because geometry providers have mutable access to meshes
pub struct InternalModelMesh {
	pub texture: MapGeometryTexture,
	pub mesh: Mesh,
	/// Entity to apply [`Mesh3d`] to. Should probably only be one of these.
	pub entity: Option<Entity>,
}

#[inline]
fn convert_vec3(config: &TrenchBroomConfig) -> impl Fn(qbsp::glam::Vec3) -> Vec3 + '_ {
	|x| config.to_bevy_space(Vec3::from_array(x.to_array()))
}

#[cfg(feature = "client")]
type Lightmap = BspLightmap;
#[cfg(not(feature = "client"))]
type Lightmap = LightmapUvMap;

pub async fn compute_models<'a, 'lc: 'a>(
	ctx: &mut BspLoadCtx<'a, 'lc>,
	lightmap: &Option<Lightmap>,
	embedded_textures: &EmbeddedTextures<'a>,
) -> Vec<InternalModel> {
	let config = &ctx.loader.tb_server.config;
	#[cfg(feature = "client")]
	let lightmap_uvs = lightmap.as_ref().map(|lm| &lm.uv_map);
	#[cfg(not(feature = "client"))]
	let lightmap_uvs = lightmap.as_ref();

	let mut models = Vec::with_capacity(ctx.data.models.len());

	for model_idx in 0..ctx.data.models.len() {
		let model_output = ctx.data.mesh_model(model_idx, lightmap_uvs);
		let mut model = InternalModel::default();
		model.meshes.reserve(model_output.meshes.len());

		for exported_mesh in model_output.meshes {
			let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, config.brush_mesh_asset_usages);

			mesh.insert_attribute(
				Mesh::ATTRIBUTE_POSITION,
				exported_mesh.positions.into_iter().map(convert_vec3(config)).collect_vec(),
			);
			mesh.insert_attribute(
				Mesh::ATTRIBUTE_NORMAL,
				exported_mesh.normals.into_iter().map(convert_vec3(config)).collect_vec(),
			);
			mesh.insert_attribute(
				Mesh::ATTRIBUTE_UV_0,
				exported_mesh.uvs.iter().map(qbsp::glam::Vec2::to_array).collect_vec(),
			);
			if let Some(lightmap_uvs) = &exported_mesh.lightmap_uvs {
				mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, lightmap_uvs.iter().map(qbsp::glam::Vec2::to_array).collect_vec());
			}
			mesh.insert_indices(Indices::U32(exported_mesh.indices.into_flattened()));

			let material = match embedded_textures.textures.get(&exported_mesh.texture) {
				Some(embedded_texture) => embedded_texture.material.clone(),
				None => {
					(config.load_loose_texture)(TextureLoadView {
						name: &exported_mesh.texture,
						tb_config: config,
						load_context: ctx.load_context,
						entities: ctx.entities,
						#[cfg(feature = "client")]
						alpha_mode: None,
						embedded_textures: Some(&embedded_textures.images),
					})
					.await
				}
			};

			model.meshes.push(InternalModelMesh {
				texture: MapGeometryTexture {
					material,
					#[cfg(feature = "client")]
					lightmap: lightmap.as_ref().map(|lm| lm.animated_lighting.clone()),
					name: exported_mesh.texture,
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

pub fn finalize_models(ctx: &mut BspLoadCtx, models: Vec<InternalModel>, world: &mut World) -> anyhow::Result<Vec<BspModel>> {
	let config = &ctx.loader.tb_server.config;

	let brush_list = match ctx.data.bspx.parse_brush_list(&ctx.data.parse_ctx) {
		Some(result) => result?,
		None => Vec::new(),
	};

	Ok(models
		.into_iter()
		.enumerate()
		.map(|(model_idx, model)| BspModel {
			meshes: model
				.meshes
				.into_iter()
				.enumerate()
				.map(|(mesh_idx, model_mesh)| {
					let mesh_handle = ctx
						.load_context
						.add_labeled_asset(format!("Model{model_idx}Mesh{mesh_idx}"), model_mesh.mesh).unwrap();

					if let Some(mesh_entity) = model_mesh.entity {
						world.entity_mut(mesh_entity).insert(Mesh3d(mesh_handle.clone()));
					}

					(model_mesh.texture.name, mesh_handle)
				})
				.collect(),

			brushes: brush_list
				.iter()
				.find(|model_brushes| model_brushes.model_idx as usize == model_idx)
				.map(|model_brushes| {
					let brushes_asset = ctx.load_context.add_labeled_asset(
						format!("Model{model_idx}Brushes"),
						BspBrushesAsset {
							brushes: model_brushes
								.brushes
								.iter()
								.map(|model_brush| {
									let min = config.to_bevy_space(model_brush.bound.min).as_dvec3();
									let max = config.to_bevy_space(model_brush.bound.max).as_dvec3();

									let mut brush = BspBrush::default();
									brush.planes.reserve(4 + model_brush.planes.len());

									#[rustfmt::skip]
									brush.planes.extend([
										BrushPlane { normal: DVec3::Y, distance: -max.y },
										BrushPlane { normal: DVec3::NEG_Y, distance: min.y },
										BrushPlane { normal: DVec3::X, distance: -max.x },
										BrushPlane { normal: DVec3::NEG_X, distance: min.x },
										BrushPlane { normal: DVec3::Z, distance: -min.z },
										BrushPlane { normal: DVec3::NEG_Z, distance: max.z },
									]);

									brush.planes.extend(model_brush.planes.iter().map(|plane| {
										// We need to invert it because brush math expects normals to point inwards
										BrushPlane {
											normal: plane.normal.as_dvec3().z_up_to_y_up(),
											distance: -plane.dist as f64 / config.scale as f64,
										}
									}));

									brush
								})
								.collect(),
						},
					).unwrap();

					if let Some(entity) = model.entity {
						world.entity_mut(entity).insert(Brushes::Bsp(brushes_asset.clone()));
					}

					brushes_asset
				}),
		})
		.collect())
}
