//! Contains Brush definitions, math, and mesh generation.

use crate::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use util::{AlmostEqual, ConvertZeroToOne};

/// Represents an infinitely large plane in 3d space, used for defining convex hulls like [Brush]es.
#[derive(Reflect, Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BrushPlane {
	pub normal: DVec3,
	pub distance: f64,
}

impl BrushPlane {
	/// Converts a triangle into a [BrushPlane]. The direction of the plane is based on the order of the vertices of the triangle.
	pub fn from_triangle(tri: [DVec3; 3]) -> Self {
		let normal = (tri[2] - tri[0]).cross(tri[1] - tri[0]).normalize();
		Self {
			normal,
			distance: -normal.dot(tri[0]),
		}
	}

	/// Calculates what side of the plane a point is on.
	///
	/// `>0` = Front Side. `<0` = Back Side. `0` = On Plane
	pub fn point_side(&self, point: DVec3) -> f64 {
		self.normal.dot(point) + self.distance
	}

	// TODO This function was made for brush uvs with quake projections, but i don't think it works right now. This isn't a high priority to fix
	/// Projects `point` onto this plane, and returns the 2d position of it.
	pub fn project(&self, point: DVec3) -> DVec2 {
		let x_normal = self.normal.cross(DVec3::Y);
		// If the x normal is 0, then the normal vector is pointing straight up, and we can use `DVec3::X` instead
		let x_normal = if x_normal == DVec3::ZERO { DVec3::X } else { x_normal };

		let y_normal = x_normal.cross(self.normal);

		dvec2(x_normal.dot(point), y_normal.dot(point))
	}

	/// Attempts to calculate the intersection point between 3 planes, returns `None` if there is no intersection, or the planes are parallel.
	pub fn calculate_intersection_point(planes: [&BrushPlane; 3]) -> Option<DVec3> {
		let [p1, p2, p3] = planes;
		let m1 = dvec3(p1.normal.x, p2.normal.x, p3.normal.x);
		let m2 = dvec3(p1.normal.y, p2.normal.y, p3.normal.y);
		let m3 = dvec3(p1.normal.z, p2.normal.z, p3.normal.z);
		let d = -dvec3(p1.distance, p2.distance, p3.distance);

		let u = m2.cross(m3);
		let v = m1.cross(d);

		let denom = m1.dot(u);

		// Check for parallel planes or if planes do not intersect
		if denom.abs() < f64::EPSILON {
			return None;
		}

		Some(dvec3(d.dot(u), m3.dot(v), -m2.dot(v)) / denom)
	}
}
impl std::ops::Neg for BrushPlane {
	type Output = Self;
	fn neg(self) -> Self::Output {
		Self {
			normal: -self.normal,
			distance: -self.distance,
		}
	}
}

/// Brush face UV coordinates.
#[derive(Reflect, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BrushUV {
	pub offset: Vec2,
	pub rotation: f32,
	pub scale: Vec2,

	/// Describes the X and Y texture-space axes, if the map is using the `Valve220` format.
	pub axes: Option<[DVec3; 2]>,
}

/// A surface of a brush, includes the plane the surface is along, the material of the surface, and the UV coordinates that the material follows.
#[derive(Reflect, Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BrushSurface {
	pub plane: BrushPlane,
	pub texture: String,
	pub uv: BrushUV,
}
impl BrushSurface {
	/// Returns this BrushSurface with it's plane facing the opposite direction.
	pub fn inverted(self) -> Self {
		Self { plane: -self.plane, ..self }
	}
}

/// A convex hull with material data attached.
#[derive(Reflect, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Brush {
	pub surfaces: Vec<BrushSurface>,
}

impl Brush {
	/// Converts a brush from [quake_util] to a bevy_trenchbroom brush.
	pub(crate) fn from_quake_util(brush: &quake_util::qmap::Brush, config: &TrenchBroomConfig) -> Self {
		Self {
			surfaces: brush
				.iter()
				.map(|surface| BrushSurface {
					plane: BrushPlane::from_triangle(surface.half_space.map(|half_space| config.to_bevy_space_f64(DVec3::from(half_space)))),
					texture: surface.texture.to_string_lossy().to_string(),
					uv: BrushUV {
						offset: DVec2::from(surface.alignment.offset).as_vec2(),
						rotation: surface.alignment.rotation as f32,
						scale: DVec2::from(surface.alignment.scale).as_vec2(),
						axes: surface
							.alignment
							.axes
							.map(|axes| axes.map(|axes_vec| config.to_bevy_space_f64(DVec3::from(axes_vec)))),
					},
				})
				.collect(),
		}
	}

	/// Returns `true` if `point` is not on the outside of the brush, else `false`.
	pub fn contains_point(&self, point: DVec3) -> bool {
		// I don't use exactly 0 here just in case the floating-point precision is a bit off
		self.surfaces.iter().all(|surface| surface.plane.point_side(point) < 0.000001)
	}

	/// Returns `true` if `plane` is intersecting the brush, or directly on one of the existing planes, else `false`.
	pub fn contains_plane(&self, plane: &BrushPlane) -> bool {
		!self
			.calculate_vertices()
			.into_iter()
			.map(|(vertex, _)| match plane.point_side(vertex) {
				0.0.. => true,
				_ => false,
			})
			.all_equal()
			|| self.surfaces.iter().any(|surface| plane == &surface.plane)
	}

	/// Cuts the brush along the specified surface, adding said surface to the brush, and removing all other surfaces in front of it.
	///
	/// NOTE: If `along` is outside of the brush, it can make this brush invalid, if you are using untrusted data, check with [Brush::contains_plane].
	pub fn cut(&mut self, along: BrushSurface) {
		// TODO this should probably support cutting along a Vec<BrushSurface>
		let vertices = self.calculate_vertices();

		// We're going to take the current vector of surfaces, and add back the ones we want to keep
		let mut old_surfaces = mem::take(&mut self.surfaces).into_iter().map(Option::from).collect_vec();

		for (vertex, surfaces) in vertices {
			if along.plane.point_side(vertex) < -f64::EPSILON {
				for surface_index in surfaces {
					// If the surface has already been added, we don't need to add it again
					if old_surfaces[surface_index].is_none() {
						continue;
					}
					self.surfaces.push(mem::take(&mut old_surfaces[surface_index]).unwrap());
				}
			}
		}

		self.surfaces.push(along);
	}

	/// Calculates the intersections of the surfaces making up the brush, filtering out intersections that exist outside the brush.
	///
	/// Returns a vector of polygonal faces where each face includes vertices, indices, and a copy of the surface the face was calculated from.
	///
	/// If you want a map of intersections to the surfaces causing them, see [calculate_vertices\()](Self::calculate_vertices)
	///
	/// NOTE: Duplicate intersections can occur on more complex shapes, (shapes where 4+ faces intersect at once) this is not a bug.
	pub fn polygonize(&self) -> impl Iterator<Item = BrushSurfacePolygon> {
		let mut vertex_map: HashMap<usize, Vec<DVec3>> = default();

		for ((s1_i, s1), (s2_i, s2), (s3_i, s3)) in self.surfaces.iter().enumerate().tuple_combinations() {
			if let Some(intersection) = BrushPlane::calculate_intersection_point([&s1.plane, &s2.plane, &s3.plane]) {
				// If the intersection does not exist within the bounds the hull, discard it
				if !self.contains_point(intersection) {
					continue;
				}

				// Add the intersection to the map
				for surface_index in [s1_i, s2_i, s3_i] {
					vertex_map.entry(surface_index).or_insert_with(Vec::new).push(intersection);
				}
			}
		}

		vertex_map
			.into_iter()
			.map(|(surface_index, vertices)| BrushSurfacePolygon::new(&self.surfaces[surface_index], vertices))
	}

	/// Calculates the intersections of the surfaces making up the brush,
	/// returns a map that maps the intersection position to the indexes of the surfaces causing it,
	/// and filters out the intersections that exist outside of the brush.
	///
	/// If you want a map of the surfaces to the intersections they cause, see [polygonize\()](Self::polygonize).
	///
	/// NOTE: Duplicate intersections can occur on more complex shapes, (shapes where 4+ faces intersect at once) this is not a bug.
	pub fn calculate_vertices(&self) -> Vec<(DVec3, [usize; 3])> {
		let mut vertex_map = Vec::new();

		for ((s1_i, s1), (s2_i, s2), (s3_i, s3)) in self.surfaces.iter().enumerate().tuple_combinations() {
			if let Some(intersection) = BrushPlane::calculate_intersection_point([&s1.plane, &s2.plane, &s3.plane]) {
				// If the intersection does not exist within the bounds the hull, discard it
				if !self.contains_point(intersection) {
					continue;
				}

				vertex_map.push((intersection, [s1_i, s2_i, s3_i]));
				// vertex_map.push(BrushVertex {
				//     position: intersection,
				//     surfaces: [s1, s2, s3],
				// });
			}
		}

		vertex_map
	}
}

/// A polygonal face calculated from a [BrushSurface], mainly used for rendering.
#[derive(Debug, Clone)]
pub struct BrushSurfacePolygon<'w> {
	pub surface: &'w BrushSurface,
	vertices: Vec<DVec3>,
	indices: Vec<u32>,
}

impl<'w> BrushSurfacePolygon<'w> {
	/// The margin that vertices can be off by, but are still treated as one.
	pub const VERTEX_PRECISION_MARGIN: f64 = 0.0001;

	/// Creates a new surface polygon, sorts and deduplicates the vertices, and calculates indices.
	pub fn new(surface: &'w BrushSurface, mut vertices: Vec<DVec3>) -> Self {
		let mut indices = Vec::new();

		// Calculate 2d vertices

		if vertices.len() > 2 {
			// Make sure there aren't duplicates at the beginning, this is needed for proper sorting
			while vertices[0].almost_eq(vertices[1], Self::VERTEX_PRECISION_MARGIN) {
				vertices.remove(1);
			}

			// Sort vertices
			let vert_0 = vertices[0];
			let vert_1 = vertices[1];
			let starting_vector = vert_1 - vert_0;

			{
				// We can't sort the first vertex, so here we skip it
				let (_first, rest) = vertices.split_first_mut().unwrap();
				rest.sort_unstable_by_key(|vertex| {
					if *vertex == vert_1 {
						return float_ord::FloatOrd(0.0);
					}
					// Make sure duplicates of vert_0 are at the start of the list for deduplication
					if vertex.almost_eq(vert_0, Self::VERTEX_PRECISION_MARGIN) {
						return float_ord::FloatOrd(f64::NEG_INFINITY);
					}
					let vertex_vector = *vertex - vert_0;

					float_ord::FloatOrd(
						vertex_vector
							.cross(starting_vector)
							.dot(surface.plane.normal)
							.atan2(starting_vector.dot(vertex_vector)),
					)
				});
			}

			vertices.dedup_by(|a, b| a.almost_eq(*b, Self::VERTEX_PRECISION_MARGIN));

			// Calculate indices
			for i in 1..vertices.len() - 1 {
				indices.extend([0, i + 1, i].map(|x| x as u32));
			}
		}

		Self { surface, vertices, indices }
	}

	pub fn vertices(&self) -> &Vec<DVec3> {
		&self.vertices
	}
	pub fn indices(&self) -> &Vec<u32> {
		&self.indices
	}
}

/// Combines a bunch of [BrushSurfacePolygon]s into a full mesh.
///
/// It is assumed all faces have the same material with the specified `texture_size`.
pub fn generate_mesh_from_brush_polygons(polygons: &[BrushSurfacePolygon], config: &TrenchBroomConfig, texture_size: UVec2) -> Mesh {
	let texture_size = texture_size.as_vec2();

	let mut vertices: Vec<DVec3> = default();
	let mut normals: Vec<DVec3> = default();
	let mut uvs: Vec<Vec2> = default();
	let mut indices: Vec<u32> = default();

	// Combine the attributes of all the polygons
	for polygon in polygons {
		indices.extend(polygon.indices.iter().map(|x| vertices.len() as u32 + *x));
		vertices.extend(&polygon.vertices);
		normals.extend(repeat_n(polygon.surface.plane.normal, polygon.vertices.len()));
		uvs.extend(polygon.vertices.iter().map(|vertex| {
			let mut uv = match polygon.surface.uv.axes {
				Some([x_normal, y_normal]) => vec2(x_normal.dot(*vertex) as f32, y_normal.dot(*vertex) as f32),
				None => polygon.surface.plane.project(*vertex).as_vec2(),
			};

			// Correct the size into Bevy space
			// Honestly not sure how this works, but it does
			if polygon.surface.uv.axes.is_some() {
				uv *= config.scale * config.scale / texture_size;
			}

			uv /= polygon.surface.uv.scale.convert_zero_to_one();
			uv += polygon.surface.uv.offset / texture_size;

			// From my testing it seems rotation is built-in to the uv axes in Valve format, so we only need to do this if the axes are not defined
			// I have no idea if this works
			if polygon.surface.uv.axes.is_none() {
				uv = Vec2::from_angle(polygon.surface.uv.rotation.to_radians()).rotate(uv);
			}

			uv
		}));
	}

	assert_eq!(vertices.len(), normals.len());

	// Convert attributes to single precision for creating the mesh
	let [vertices, normals] = [vertices, normals].map(|x| x.into_iter().map(|x| x.as_vec3()).collect::<Vec<Vec3>>());

	let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, config.brush_mesh_asset_usages);
	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
	mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
	if !uvs.is_empty() {
		mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
	}
	mesh.insert_indices(Indices::U32(indices));

	mesh
}

#[test]
fn contains_point() {
	let mut brush = Brush::default();
	let normals = [DVec3::X, DVec3::NEG_X, DVec3::Y, DVec3::NEG_Y, DVec3::Z, DVec3::NEG_Z];

	for normal in normals {
		brush.surfaces.push(BrushSurface {
			plane: BrushPlane { normal, distance: -16. },
			texture: default(),
			uv: default(),
		});
	}

	assert!(brush.contains_point(DVec3::ZERO));
	for normal in normals {
		assert!(brush.contains_point(normal));
	}
	assert!(!brush.contains_point(dvec3(32., 0., 0.)));
	assert!(!brush.contains_point(dvec3(0., 32., 0.)));
	assert!(!brush.contains_point(dvec3(0., 0., 32.)));
}

#[test]
fn triangle_conversion() {
	let tri_1 = [dvec3(-16., 16., -32.), dvec3(-16., 16., 16.), dvec3(-16., -16., 16.)];
	let tri_2 = [dvec3(16., 16., 16.), dvec3(16., 16., -32.), dvec3(16., -16., -16.)];

	let plane_1 = BrushPlane::from_triangle(tri_1);
	let plane_2 = BrushPlane::from_triangle(tri_2);

	assert_eq!(plane_1.normal, dvec3(-1., 0., 0.));
	assert_eq!(plane_2.normal, dvec3(1., 0., 0.));
	assert_eq!(plane_1.distance, -16.);
	assert_eq!(plane_2.distance, -16.);
}
