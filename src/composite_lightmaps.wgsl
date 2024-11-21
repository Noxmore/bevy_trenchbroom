// Must match const value in `bsp_lighting.rs`
const MAX_ANIMATORS: u32 = 254;

struct Animator {
    sequence: array<vec3f, 64>,
    sequence_len: u32,
    speed: f32,
    interpolate: u32,
}

@group(0) @binding(0) var input_texture: texture_3d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> layers: u32;
@group(0) @binding(3) var<uniform> animators: array<Animator, MAX_ANIMATORS>;

@group(1) @binding(0) var<uniform> seconds: f32;

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

// fn get_anim_multiplier(i: u32) -> vec3f {
//     return animators[i].sequence[u32(seconds * animators[i].speed) % animators[i].sequence_len];
// }

@fragment
fn fragment(
    input: VertexOutput,
) -> @location(0) vec4f {
    // return position;
    var color = vec4f(0, 0, 0, 1);

    for (var i: u32 = 0; i < layers; i++) {
        var mul = animators[i].sequence[u32(seconds * animators[i].speed) % animators[i].sequence_len];
        if animators[i].interpolate != 0 {
            mul = mix(mul, animators[i].sequence[(u32(seconds * animators[i].speed) + 1) % animators[i].sequence_len], (seconds * animators[i].speed) % 1);
        }

        // 0.5 is being added to i here because it was sampling layer 0 twice, and not sampling the last layer at all -- probably floating point imprecision.
        color += textureSample(input_texture, input_sampler, vec3f(input.uv, (f32(i) + 0.5) / f32(layers))) * vec4(mul, 1);
    }

    return color;
}
