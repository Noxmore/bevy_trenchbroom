#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_functions::get_world_from_local

@group(2) @binding(1) var color_texture: texture_2d<f32>;
@group(2) @binding(2) var color_sampler: sampler;

@group(2) @binding(0) var<uniform> speed: f32;

@fragment
fn fragment(
    mesh: VertexOutput,
    // @builtin(position) frag_coord: vec4<f32>,
) -> @location(0) vec4<f32> {
    let world_from_local = get_world_from_local(mesh.instance_index);
    return textureSample(color_texture, color_sampler, mesh.uv);
    // return vec4<f32>(1, 0, 0, 1);
    // return vec4<f32>(mesh.world_normal, 1);
}