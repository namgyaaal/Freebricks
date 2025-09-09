struct Camera {
    view_proj: mat4x4<f32>,
    view_pos: vec4<f32>,
}

struct Light {
    direction: vec3<f32>,
}

@group(0) @binding(2)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) tex_scale: vec2<u32>
}

struct InstancingPartData {
    @location(5) model_matrix_0: vec3<f32>,
    @location(6) model_matrix_1: vec3<f32>,
    @location(7) model_matrix_2: vec3<f32>,
    @location(8) model_matrix_3: vec3<f32>,

    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
    @location(12) color:          vec4<f32>, 
    @location(13) size:          vec3<f32>,
    @location(14) stud_layout:         u32,
}

struct UniformPartData {
    model: mat4x4<f32>, 
    normal: mat4x4<f32>, 
    color: vec4<f32>,
    size: vec3<f32>,

    stud_layout: u32
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) stud_index: u32,
    @location(3) world_normal: vec3<f32>,
    @location(4) world_position: vec3<f32>,
}

// Used only by uniform 
@group(1) @binding(0)
var<uniform> part_uniform: UniformPartData;


@vertex
fn vs_main_uniform(
    model: VertexInput,
    @builtin(vertex_index) vertex_index: u32
) -> VertexOutput{
    let model_matrix = part_uniform.model;
    let normal_matrix = part_uniform.normal; 

    var out: VertexOutput;
    
    var world_position = model_matrix * vec4<f32>(model.position, 1.0);

    out.world_position = world_position.xyz; 
    out.world_normal = (normal_matrix * vec4<f32>(model.normal, 1.0)).xyz;
    out.clip_position = camera.view_proj * world_position; 
    out.color = part_uniform.color.xyz;

    out.tex_coords = model.tex_coords * vec2<f32>(
        part_uniform.size[model.tex_scale.x],
        part_uniform.size[model.tex_scale.y]
    );
    out.stud_index = ((part_uniform.stud_layout) >> ((vertex_index / 4) * 4)) & 0xF;

    return out;
} 

@vertex
fn vs_main_instanced(
    model: VertexInput,
    instance: InstancingPartData,
    @builtin(vertex_index) vertex_index: u32
) -> VertexOutput {
    let model_matrix: mat4x4<f32> = mat4x4<f32>(
        vec4<f32>(instance.model_matrix_0, 0.0),
        vec4<f32>(instance.model_matrix_1, 0.0),
        vec4<f32>(instance.model_matrix_2, 0.0),
        vec4<f32>(instance.model_matrix_3, 1.0) // assuming affine
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );

    var out: VertexOutput;
    
    var world_position = model_matrix * vec4<f32>(model.position, 1.0);

    out.world_position = world_position.xyz; 
    out.world_normal = normal_matrix * model.normal;
    out.clip_position = camera.view_proj * world_position; 
    out.color = instance.color.xyz;

    out.tex_coords = model.tex_coords * vec2<f32>(
        instance.size[model.tex_scale.x],
        instance.size[model.tex_scale.y]
    );
    out.stud_index = ((instance.stud_layout) >> ((vertex_index / 4) * 4)) & 0xF;

    return out;
}


@group(0) @binding(0)
var t_diffuse: texture_2d_array<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;
@group(0) @binding(3)
var<uniform> light: Light;

@fragment
fn fs_main_uniform(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(0.0, 0.0, 0.0, 1.0);
}

@fragment
fn fs_main_instanced(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords; 
    // Specular lighting
    let norm = normalize(in.world_normal);

    let view_dir = normalize(camera.view_pos.xyz - in.world_position);
    let reflect_dir = reflect(-light.direction, norm);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);

    let specular_strength = vec3<f32>(1.0, 1.0, 1.0) * spec; 
    let diffuse_strength = max(dot(norm, -light.direction), 0.0); 
    let ambient_strength = vec3<f32>(1.0, 1.0, 1.0) * 0.1;


    var color = vec4<f32>(in.color, 1.0);
    // Apply stud texturing 
    if in.stud_index > 0 {
        let stud_diffuse = textureSample(t_diffuse, s_diffuse, uv, in.stud_index);
        color = mix(color, vec4<f32>(stud_diffuse.rgb, 1.0), stud_diffuse.a);
    }
    return color * vec4<f32>(ambient_strength + diffuse_strength + specular_strength, 1.0);
}
 