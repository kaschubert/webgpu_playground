// Vertex shader brown tri

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main_brown_tri(
    [[builtin(vertex_index)]] in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}


// Fragment shader brown tri

[[stage(fragment)]]
fn fs_main_brown_tri(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
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
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.position = vec2<f32>(x, y);
    return out;
}


// Fragment shader colored tri

[[stage(fragment)]]
fn fs_main_colored_tri(in: VertexOutputColoredTri) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(in.position, 0.1, 1.0);
}
