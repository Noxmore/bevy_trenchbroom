const LIGHTSTYLE_COUNT: u32 = 254;

// struct LightstyleInfo {
//     active: bool,
// }

// @group(0) @binding(1) var lightmap_textures: array<texture_2d<f32>, LIGHTSTYLE_COUNT>;
@group(0) @binding(0) var input_texture: texture_3d<f32>;
@group(0) @binding(1) var input_sampler: sampler;
@group(0) @binding(2) var<uniform> layers: u32;
// @group(0) @binding(3) var<uniform> infos: array<LightstyleInfo>;
// @group(0) @binding(1) var lightmap_textures: array<texture_storage_2d<rgba8unorm, read>, LIGHTSTYLE_COUNT>;
// @group(0) @binding(1) var lightmap_textures: texture_storage_2d_array<rgba8unorm, read>;
// @group(0) @binding(2) var outout_texture: texture_storage_2d<f32, write>;

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
// @vertex
// fn vertex(
//     @location(0) position: vec2f,
// ) -> @builtin(position) vec4f {
//     return vec4f(position.x, position.y, 1, 1);
// }

@fragment
fn fragment(
    input: VertexOutput,
) -> @location(0) vec4f {
    // return position;
    var color = vec4f(0, 0, 0, 1);

    for (var i: u32 = 0; i < layers; i++) {
        color += textureSample(input_texture, input_sampler, vec3f(input.uv, f32(i) / f32(layers)));
    }

    return color;
}
