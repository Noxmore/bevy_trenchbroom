#define_import_path bevy_trenchbroom::composite_lightmaps

#import bevy_render::globals::Globals

// Must match const value in `bsp_lighting.rs`
const MAX_ANIMATORS: u32 = 255;

struct Animator {
	sequence: array<vec3f, 64>,
	sequence_len: u32,
	speed: f32,
	interpolate: u32,
}

@group(0) @binding(0) var input_texture_0: texture_2d<f32>;
@group(0) @binding(1) var input_texture_1: texture_2d<f32>;
@group(0) @binding(2) var input_texture_2: texture_2d<f32>;
@group(0) @binding(3) var input_texture_3: texture_2d<f32>;
@group(0) @binding(4) var input_texture_mapping: texture_2d<u32>;
@group(0) @binding(5) var input_sampler: sampler;
@group(0) @binding(6) var<uniform> animators: array<Animator, MAX_ANIMATORS>;

@group(1) @binding(0) var<uniform> globals: Globals;

struct VertexOutput {
	@builtin(position) position: vec4f,
	@location(0) uv: vec2f,
}

@vertex
fn vertex(
	@builtin(vertex_index) vert_idx: u32,
) -> VertexOutput {
	// Create a quad
	var output: VertexOutput;
	output.uv = vec2f(f32(vert_idx % 2), f32(vert_idx / 2));
	output.position = vec4f(output.uv * 2 - 1, 0, 1);
	output.uv.y = 1 - output.uv.y; // TODO this is hacky

	return output;
}

fn sample_atlas(input: texture_2d<f32>, animator_idx: u32, uv: vec2f) -> vec4f {
	var mul = animators[animator_idx].sequence[u32(globals.time * animators[animator_idx].speed) % animators[animator_idx].sequence_len];
	if animators[animator_idx].interpolate != 0 {
		mul = mix(mul, animators[animator_idx].sequence[(u32(globals.time * animators[animator_idx].speed) + 1) % animators[animator_idx].sequence_len], (globals.time * animators[animator_idx].speed) % 1);
	}

	return textureSample(input, input_sampler, uv) * vec4f(mul, 1);
}

@fragment
fn fragment(
	input: VertexOutput,
) -> @location(0) vec4f {
	var color = vec4f(0, 0, 0, 1);

	let mapping: vec4u = textureLoad(input_texture_mapping, vec2u(input.uv * vec2f(textureDimensions(input_texture_mapping))), 0);

	color += sample_atlas(input_texture_0, mapping.x, input.uv);
	color += sample_atlas(input_texture_1, mapping.y, input.uv);
	color += sample_atlas(input_texture_2, mapping.z, input.uv);
	color += sample_atlas(input_texture_3, mapping.w, input.uv);

	return color;
}
