#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::pbr_fragment::pbr_input_from_vertex_output
#import bevy_pbr::mesh_view_bindings::globals

struct QuakeSkyMaterial {
    fg_speed: f32,
    bg_speed: f32,
    texture_scale: f32,
    sphere_scale: vec3f,
}

@group(2) @binding(1) var color_texture: texture_2d<f32>;
@group(2) @binding(2) var color_sampler: sampler;

@group(2) @binding(0) var<uniform> material: QuakeSkyMaterial;

fn half_uv(uv: vec2f) -> vec2f {
    return vec2f(uv.x % 0.5, uv.y);
}

@fragment
fn fragment(
    mesh: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    // We're creating a PbrInput just to have the `V` vield from it. Probably a waste of processing time, but it's fine for now.
    let pbr_input = pbr_input_from_vertex_output(mesh, is_front, false);
    var normal: vec3f = pbr_input.V;
    normal *= material.sphere_scale;
    normal = normalize(normal);
    let uv = vec2f(dot(normal, vec3f(0, 0, 1)), dot(normal, vec3f(1, 0, 0)));

    // Starting offset so we don't see negative offset in the sky
    let scroll = globals.time + material.texture_scale; // TODO

    // TODO weird overflows when texture filtering
    let fg_uv = abs(half_uv(uv * material.texture_scale + scroll * material.fg_speed));
    let fg = textureSample(color_texture, color_sampler, fg_uv);

    // If the foreground is black, render the background.
    if fg.r == 0 && fg.g == 0 && fg.b == 0 {
        // TODO black borders, store alpha channel?
        let bg_uv = abs(half_uv(uv * material.texture_scale + scroll * material.bg_speed)) + vec2f(0.5, 0);
        return textureSample(color_texture, color_sampler, bg_uv);
    } else {
        return fg;
    }
}
