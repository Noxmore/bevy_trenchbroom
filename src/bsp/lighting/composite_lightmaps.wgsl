#define_import_path bevy_trenchbroom::composite_lightmaps

#import bevy_render::globals::Globals

// Must match const value in `bsp_lighting.rs`
const MAX_ANIMATORS: u32 = 255;

struct Animator {
	sequence: array<vec3f, 64>,
	sequence_len: u32,
	speed: f32,
	interpolate: f32,
}

@group(0) @binding(0) var input_texture_0: texture_2d<f32>;
@group(0) @binding(1) var input_texture_1: texture_2d<f32>;
@group(0) @binding(2) var input_texture_2: texture_2d<f32>;
@group(0) @binding(3) var input_texture_3: texture_2d<f32>;
@group(0) @binding(4) var input_texture_mapping: texture_2d<u32>;
@group(0) @binding(5) var<storage, read> animators: array<Animator, MAX_ANIMATORS>;
@group(0) @binding(6) var output: texture_storage_2d<rgba32float, write>;

@group(1) @binding(0) var<uniform> globals: Globals;

fn sample_atlas(input: texture_2d<f32>, animator_idx: u32, uv: vec2u) -> vec4f {
	// MUST stay the same as in LightingAnimator::sample
	var mul = animators[animator_idx].sequence[u32(globals.time * animators[animator_idx].speed) % animators[animator_idx].sequence_len];
	if animators[animator_idx].interpolate > 0 {
		let next = animators[animator_idx].sequence[(u32(globals.time * animators[animator_idx].speed) + 1) % animators[animator_idx].sequence_len];
		let t = min(((globals.time * animators[animator_idx].speed) % 1) / animators[animator_idx].interpolate, 1.0);

		mul = mix(mul, next, t);
	}

	return textureLoad(input, uv, 0) * vec4f(mul, 1);
}

@compute @workgroup_size(8, 8, 1)
fn main(
	@builtin(global_invocation_id) invocation_id: vec3u,
) {
	var color = vec4f(0, 0, 0, 1);

	let coords = invocation_id.xy;
	let mapping: vec4u = textureLoad(input_texture_mapping, coords % textureDimensions(input_texture_mapping), 0);

	color += sample_atlas(input_texture_0, mapping.x, coords);
	color += sample_atlas(input_texture_1, mapping.y, coords);
	color += sample_atlas(input_texture_2, mapping.z, coords);
	color += sample_atlas(input_texture_3, mapping.w, coords);

	textureStore(output, coords, color);
}
