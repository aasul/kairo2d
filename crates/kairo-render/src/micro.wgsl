struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};
struct Palette {
    colors: array<vec4<f32>, 16>,
    info: vec4<u32>,
};
@group(0) @binding(0) var canvas: texture_2d<f32>;
@group(0) @binding(1) var nearest_sampler: sampler;
@group(1) @binding(0) var<uniform> palette: Palette;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> Output {
    var positions = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    let p = positions[index];
    var output: Output;
    output.position = vec4<f32>(p, 0.0, 1.0);
    output.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5);
    return output;
}

@fragment
fn fs_main(input: Output) -> @location(0) vec4<f32> {
    let original = textureSample(canvas, nearest_sampler, input.uv);
    if palette.info.x == 0u { return original; }
    var best = palette.colors[0].rgb;
    var distance = 100.0;
    for (var i = 0u; i < palette.info.x; i = i + 1u) {
        let candidate = palette.colors[i].rgb;
        let delta = original.rgb - candidate;
        let d = dot(delta, delta);
        if d < distance { distance = d; best = candidate; }
    }
    return vec4<f32>(best, original.a);
}
