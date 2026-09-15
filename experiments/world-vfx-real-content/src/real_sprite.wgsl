struct Instance {
    pos_size: vec4<f32>,
    resource_data: vec4<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<storage, read> instances: array<Instance>;
@group(0) @binding(1)
var sprite_texture: texture_2d_array<f32>;
@group(0) @binding(2)
var sprite_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) layer: u32,
};

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, -0.5),
        vec2<f32>(0.5, 0.5), vec2<f32>(-0.5, -0.5),
        vec2<f32>(0.5, 0.5), vec2<f32>(-0.5, 0.5),
    );
    let instance = instances[instance_index];
    let corner = corners[vertex_index];
    var output: VertexOutput;
    output.clip_position = vec4<f32>(
        instance.pos_size.xy + corner * instance.pos_size.zw,
        0.0,
        1.0,
    );
    output.uv = corner + vec2<f32>(0.5, 0.5);
    output.color = instance.color;
    output.layer = u32(instance.resource_data.x);
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(sprite_texture, sprite_sampler, input.uv, input.layer) * input.color;
}
