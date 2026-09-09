struct Instance {
    pos_size: vec4<f32>,
    uv_rect: vec4<f32>,
    color: vec4<f32>,
    meta: vec4<f32>,
};

struct Globals {
    viewport_ambient: vec4<f32>,
    lights: array<vec4<f32>, 16>,
};

@group(0) @binding(0)
var<storage, read> instances: array<Instance>;
@group(0) @binding(1)
var<uniform> globals: Globals;
@group(1) @binding(0)
var page_texture: texture_2d_array<f32>;
@group(1) @binding(1)
var page_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) screen_px: vec2<f32>,
    @location(3) meta: vec4<f32>,
    @location(4) @interpolate(flat) layer: u32,
};

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5),
        vec2<f32>(0.5, -0.5),
        vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, -0.5),
        vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, 0.5),
    );
    let instance = instances[instance_index];
    let corner = corners[vertex_index];
    let screen = instance.pos_size.xy + corner * instance.pos_size.zw;
    let viewport = globals.viewport_ambient.xy;
    var output: VertexOutput;
    output.clip_position = vec4<f32>(
        screen.x * 2.0 / viewport.x,
        -screen.y * 2.0 / viewport.y,
        0.0,
        1.0,
    );
    output.uv = corner + vec2<f32>(0.5, 0.5);
    output.color = instance.color;
    output.screen_px = screen;
    output.meta = instance.meta;
    output.layer = u32(instance.uv_rect.x);
    return output;
}

fn local_light(screen_px: vec2<f32>) -> f32 {
    var contribution = 0.0;
    let light_count = u32(globals.viewport_ambient.w);
    for (var index: u32 = 0u; index < 16u; index = index + 1u) {
        if (index >= light_count) {
            break;
        }
        let light = globals.lights[index];
        let distance_px = distance(screen_px, light.xy);
        let falloff = clamp(1.0 - distance_px / max(light.z, 1.0), 0.0, 1.0);
        contribution = contribution + falloff * light.w;
    }
    return contribution;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if (input.meta.y > 0.5) {
        return input.color;
    }
    var color = textureSample(page_texture, page_sampler, input.uv, i32(input.layer)) * input.color;
    let illumination = clamp(globals.viewport_ambient.z + local_light(input.screen_px), 0.12, 1.45);
    color.rgb = color.rgb * illumination + color.rgb * input.meta.x;
    return color;
}
