struct Instance {
    pos_size: vec4<f32>,
    color: vec4<f32>,
    shape: vec4<f32>,
};

@group(0) @binding(0)
var<storage, read> instances: array<Instance>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) shape: vec4<f32>,
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
    output.local = corner * 2.0;
    output.color = instance.color;
    output.shape = instance.shape;
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if input.shape.x < 0.5 {
        return input.color;
    }
    let radius = length(input.local);
    let ring_distance = abs(radius - 0.76);
    let ring = 1.0 - smoothstep(0.055, 0.135, ring_distance);
    let outer = 1.0 - smoothstep(0.94, 1.0, radius);
    return vec4<f32>(input.color.rgb, input.color.a * ring * outer);
}
