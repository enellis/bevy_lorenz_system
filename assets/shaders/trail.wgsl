// The time since startup data is in the globals binding which is part of the mesh_view_bindings import
#import bevy_pbr::{
    mesh_view_bindings::globals,
}

@group(2) @binding(0) var<uniform> material_color: vec4<f32>;
@group(2) @binding(1) var<uniform> birth_time: f32;
@group(2) @binding(2) var<uniform> lifetime: f32;

@fragment
fn fragment() -> @location(0) vec4<f32> {
    let alpha = 1 - ((globals.time - birth_time) / lifetime);
    return vec4<f32>(material_color.rgb, alpha);
}
