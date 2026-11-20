// D3D9 overlay shader pair, compiled offline by build.rs (vs/ps_3_0 with a
// vs/ps_2_0 fallback). Source of truth for the bytecode embedded in shader.rs.
//
// D3D9 samples the render target at *integer* coordinates (the centre of pixel
// (0,0) is at 0,0), while D3D10+ and every other backend use half-integer
// centres. Without the half-pixel shift the whole mesh lands half a pixel off
// and every glyph edge is blended across two pixels by the linear atlas
// sampler, which is what makes d3d9 text look soft.

float4 uParams : register(c0);

struct VSIn  { float2 pos : POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };
struct PSIn  { float4 pos : POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };

PSIn vs_main(VSIn i) {
    PSIn o;
    float2 p = (i.pos - 0.5) / uParams.xy * 2.0 - 1.0;
    o.pos = float4(p.x, -p.y, 0.0, 1.0);
    o.uv = i.uv;
    o.col = i.col;
    return o;
}

sampler2D smp : register(s0);

/* D3D9 back buffers are plain 8-bit colour targets, so there is no sRGB decode
   to do here: egui already produced display-space colours. */
float4 ps_main(PSIn i) : COLOR {
    float a = tex2D(smp, i.uv).r;
    return i.col * a;
}
