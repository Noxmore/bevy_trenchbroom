const LIGHTSTYLE_COUNT: u32 = 254;

// struct LightstyleInfo {
//     active: bool,
// }

// @group(0) @binding(1) var lightmap_textures: array<texture_2d<f32>, LIGHTSTYLE_COUNT>;
@group(0) @binding(0) var lightmap_textures: texture_2d_array<f32>;
@group(0) @binding(1) var lightmap_sampler: sampler;
// @group(0) @binding(3) var<uniform> infos: array<LightstyleInfo>;
// @group(0) @binding(1) var lightmap_textures: array<texture_storage_2d<rgba8unorm, read>, LIGHTSTYLE_COUNT>;
// @group(0) @binding(1) var lightmap_textures: texture_storage_2d_array<rgba8unorm, read>;
// @group(0) @binding(2) var outout_texture: texture_storage_2d<f32, write>;

@vertex
fn vertex(
    @builtin(vertex_index) vert_idx: u32,
) -> @builtin(position) vec2<f32> {
    // Create a quad
    return vec2<f32>(vert_idx % 2, vert_idx / 2);
}

@fragment
fn fragment(
    @builtin(position) position: vec2<f32>,
) -> @location(0) vec4<f32> {
    var color = vec4<f32>(0, 0, 0, 1);

    for (var i = 0; i < arrayLength(&lightmap_textures); i++) {
        color += textureSample(lightmap_textures, lightmap_sampler, vec3<f32>(position, f32(i)));
    }

    return color;
}
