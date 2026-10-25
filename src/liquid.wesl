import bevy_pbr::render::{
	pbr_fragment::pbr_input_from_standard_material,
	pbr_functions::alpha_discard,
	mesh_view_bindings::globals,
};

@if(PREPASS_PIPELINE)
import bevy_pbr::{
	prepass::io::{VertexOutput, FragmentOutput},
	deferred::functions::deferred_output,
};
@else
import bevy_pbr::render::{
	forward_io::{VertexOutput, FragmentOutput},
	pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
};

struct LiquidMaterial {
	magnitude: f32,
	cycles: f32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> material: LiquidMaterial;

@fragment
fn fragment(
	in_: VertexOutput,
	@builtin(front_facing) is_front: bool,
) -> FragmentOutput {
	var in = in_;
	in.uv += vec2f(sin(globals.time + in.uv.y * material.cycles + (material.cycles / 2)), sin(globals.time + in.uv.x * material.cycles)) * vec2f(material.magnitude);

	var pbr_input = pbr_input_from_standard_material(in, is_front);

	pbr_input.material.base_color = alpha_discard(
		pbr_input.material.flags,
		pbr_input.material.alpha_cutoff,
		pbr_input.material.base_color,
	);

	var out: FragmentOutput;
	@if(PREPASS_PIPELINE) {
		out = deferred_output(in, pbr_input);
	} @else {
		// TODO: pbr lighting makes the liquid way brighter than it should be??
		out.color = apply_pbr_lighting(pbr_input);
		out.color = main_pass_post_lighting_processing(pbr_input, out.color);
	}

	return out;
}
