#import bevy_pbr::mesh_view_bindings::globals
#import bevy_pbr::view_transformations::position_world_to_clip

struct Vertex {
    @builtin(instance_index) instance_index: u32,

    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    @location(3) i_pos_length: vec4<f32>,
    @location(4) i_rotation: vec4<f32>,
    @location(5) i_color: vec3<f32>,
    @location(6) i_birth_time: f32,
    @location(7) i_lifetime: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var pos = vertex.position;

    // Scale length
    pos.y *= vertex.i_pos_length.w;

    // Shrink trail with age
    let ratio = max(0.0, 1.0 - (globals.time - vertex.i_birth_time) / vertex.i_lifetime);
    pos.x *= ratio;
    pos.z *= ratio;

    // Apply quaternion rotation
    pos = pos + 2.0 * cross(vertex.i_rotation.xyz, cross(vertex.i_rotation.xyz, pos) + vertex.i_rotation.w * pos);

    // Apply translation
    pos = pos + vertex.i_pos_length.xyz;

    var out: VertexOutput;
    out.clip_position = position_world_to_clip(pos);
    out.color = vec4<f32>(vertex.i_color, 1.0);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
