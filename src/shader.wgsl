// Vertex shader brown tri

[[block]] // 1.
struct CameraUniform {
    view_proj: mat4x4<f32>;
};
[[group(1), binding(0)]] // 2.
var<uniform> camera: CameraUniform;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] tex_coords: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coords: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main_brown_tri(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}


// Fragment shader brown tri

[[group(0), binding(0)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(1)]]
var s_diffuse: sampler;

[[stage(fragment)]]
fn fs_main_brown_tri(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}

// Vertex shader colored tri

struct VertexOutputColoredTri {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] position: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main_colored_tri(
    [[builtin(vertex_index)]] in_vertex_index: u32,
) -> VertexOutputColoredTri {
    var out: VertexOutputColoredTri;
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = camera.view_proj * vec4<f32>(x, y, 0.0, 1.0);
    out.position = vec2<f32>(x, y);
    return out;
}


// Fragment shader colored tri

[[stage(fragment)]]
fn fs_main_colored_tri(in: VertexOutputColoredTri) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(in.position, 0.1, 1.0);
}
