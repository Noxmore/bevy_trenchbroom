#import bevy_trenchbroom::composite_lightmaps::MAX_ANIMATORS
#import bevy_trenchbroom::composite_lightmaps::Animator
#import bevy_render::globals::Globals

@group(0) @binding(0) var input_texture_0: texture_3d<f32>;
@group(0) @binding(1) var input_texture_1: texture_3d<f32>;
@group(0) @binding(2) var input_texture_2: texture_3d<f32>;
@group(0) @binding(3) var input_texture_3: texture_3d<f32>;
@group(0) @binding(4) var input_texture_mapping: texture_3d<u32>;
@group(0) @binding(5) var<uniform> animators: array<Animator, MAX_ANIMATORS>;
@group(0) @binding(6) var output: texture_storage_3d<rgba32float, write>;

@group(1) @binding(0) var<uniform> globals: Globals;

fn sample_atlas(input: texture_3d<f32>, animator_idx: u32, coords: vec3u) -> vec4f {
	var mul = animators[animator_idx].sequence[u32(globals.time * animators[animator_idx].speed) % animators[animator_idx].sequence_len];
	if animators[animator_idx].interpolate > 0 {
		let next = animators[animator_idx].sequence[(u32(globals.time * animators[animator_idx].speed) + 1) % animators[animator_idx].sequence_len];
		let t = min(((globals.time * animators[animator_idx].speed) % 1) / animators[animator_idx].interpolate, 1.0);
		
		mul = mix(mul, next, t);
	}

	return textureLoad(input, coords % textureDimensions(input), 0) * vec4f(mul, 1);
}

@compute @workgroup_size(4, 4, 4)
fn main(
	@builtin(global_invocation_id) invocation_id: vec3<u32>
) {
	var color = vec4f(0, 0, 0, 1);

	let mapping: vec4u = textureLoad(input_texture_mapping, invocation_id % textureDimensions(input_texture_mapping), 0);

	color += sample_atlas(input_texture_0, mapping.x, invocation_id);
	color += sample_atlas(input_texture_1, mapping.y, invocation_id);
	color += sample_atlas(input_texture_2, mapping.z, invocation_id);
	color += sample_atlas(input_texture_3, mapping.w, invocation_id);

	textureStore(output, invocation_id, color);
}