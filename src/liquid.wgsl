#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct LiquidMaterial {
    seconds: f32,
}

@group(2) @binding(100) var<uniform> material: LiquidMaterial;

// TODO Should these be uniforms?
const MAGNITUDE: f32 = 0.05;
const CYCLES: f32 = 6.28;

@fragment
fn fragment(
    in_: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var in = in_;
    in.uv += vec2f(sin(material.seconds + in.uv.y * CYCLES + (CYCLES / 2)), sin(material.seconds + in.uv.x * CYCLES)) * vec2f(MAGNITUDE);
    
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
#endif

    return out;
}
