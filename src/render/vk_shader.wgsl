struct Uniforms {
    screen: vec2<f32>,
    pad: vec2<f32>,
}
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var atlas: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) col: vec4<f32>,
}

@vertex
fn vs_main(
    @location(0) a_pos: vec2<f32>,
    @location(1) a_uv: vec2<f32>,
    @location(2) a_col: vec4<f32>,
) -> VsOut {
    var out: VsOut;
    let p = a_pos / u.screen * 2.0 - vec2<f32>(1.0, 1.0);
    out.pos = vec4<f32>(p.x, -p.y, 0.0, 1.0);
    out.uv = a_uv;
    out.col = a_col;
    return out;
}

@fragment
fn ps_main(in: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(atlas, smp, in.uv).r;
    var c = in.col * a;
    if (u.pad.x > 0.5) {
        c = vec4<f32>(srgb_to_linear(c.rgb), c.a);
    }
    return c;
}
