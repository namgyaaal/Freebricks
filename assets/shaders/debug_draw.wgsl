struct Camera {
    view_proj: mat4x4<f32>,
    view_pos: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;



struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    @builtin(vertex_index) vertex_index: u32
) -> VertexOutput {

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);

    return out;
}



@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.2, 0.2, 1.0);
}
 