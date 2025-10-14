#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::pbr_fragment::pbr_input_from_vertex_output
#import bevy_pbr::mesh_view_bindings::globals

struct QuakeSkyMaterial {
	fg_scroll: vec2f,
	bg_scroll: vec2f,
	texture_scale: f32,
	sphere_scale: vec3f,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(1) var fg_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var fg_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(3) var bg_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var bg_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: QuakeSkyMaterial;

@fragment
fn fragment(
	mesh: VertexOutput,
	@builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
	// We're creating a PbrInput just to have the `V` field from it. Probably a waste of processing time, but it's fine for now.
	let pbr_input = pbr_input_from_vertex_output(mesh, is_front, false);
	var normal: vec3f = pbr_input.V;
	normal *= material.sphere_scale;
	normal = normalize(normal);
	let uv = vec2f(dot(normal, vec3f(0, 0, 1)), dot(normal, vec3f(1, 0, 0)));

	let fg = textureSample(fg_texture, fg_sampler, uv * material.texture_scale + globals.time * material.fg_scroll);
	let bg = textureSample(bg_texture, bg_sampler, uv * material.texture_scale + globals.time * material.bg_scroll);

	// If we don't do this with fg.a, the black edges around fg will be too noticeable, this helps a little.
	return mix(bg, fg, fg.a * fg.a);
}
