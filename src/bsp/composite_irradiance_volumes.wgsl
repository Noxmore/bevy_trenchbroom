#import bevy_trenchbroom::composite_lightmaps::MAX_ANIMATORS
#import bevy_trenchbroom::composite_lightmaps::Animator
#import bevy_render::globals::Globals

@group(0) @binding(0) var input_texture_0: texture_3d<f32>;
@group(0) @binding(1) var input_texture_1: texture_3d<f32>;
@group(0) @binding(2) var input_texture_2: texture_3d<f32>;
@group(0) @binding(3) var input_texture_3: texture_3d<f32>;
@group(0) @binding(4) var input_texture_mapping: texture_3d<u32>;
@group(0) @binding(5) var<uniform> full_texture_size: vec3u;
@group(0) @binding(6) var<uniform> animators: array<Animator, MAX_ANIMATORS>;

@group(1) @binding(0) var<uniform> globals: Globals;

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec3f,
}

@vertex
fn vertex(
    @builtin(vertex_index) vert_idx: u32,
) -> VertexOutput {
    // Create a quad
    var output: VertexOutput;
    output.uv = vec3f(f32(vert_idx % 2), f32((vert_idx % 4) / 2), f32(vert_idx / 4));
    output.position = vec4f(output.uv * 2 - 1, 1);
    output.uv.y = 1 - output.uv.y; // TODO this is hacky

    return output;
}

fn sample_atlas(input: texture_3d<f32>, animator_idx: u32, coords: vec3u) -> vec4f {
    var mul = animators[animator_idx].sequence[u32(globals.time * animators[animator_idx].speed) % animators[animator_idx].sequence_len];
    if animators[animator_idx].interpolate != 0 {
        mul = mix(mul, animators[animator_idx].sequence[(u32(globals.time * animators[animator_idx].speed) + 1) % animators[animator_idx].sequence_len], (globals.time * animators[animator_idx].speed) % 1);
    }

    return textureLoad(input, coords % textureDimensions(input), 0) * vec4f(mul, 1);
}

@fragment
fn fragment(
    input: VertexOutput,
) -> @location(0) vec4f {
    var color = vec4f(0, 0, 0, 1);

    let coords = vec3u(input.uv * vec3f(full_texture_size)) % textureDimensions(input_texture_mapping);
    let mapping: vec4u = textureLoad(input_texture_mapping, coords, 0);

    color += sample_atlas(input_texture_0, mapping.x, coords);
    color += sample_atlas(input_texture_1, mapping.y, coords);
    color += sample_atlas(input_texture_2, mapping.z, coords);
    color += sample_atlas(input_texture_3, mapping.w, coords);

    return color;
}