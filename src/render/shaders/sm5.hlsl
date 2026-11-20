// Shader model 5 pair shared by the D3D11 and D3D12 painters, compiled offline
// by build.rs. Source of truth for the bytecode embedded in shader.rs.

cbuffer Params : register(b0) { float2 uScreen; float uSrgb; float uPad; };
struct VSIn  { float2 pos : POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };
struct PSIn  { float4 pos : SV_POSITION; float2 uv : TEXCOORD0; float4 col : COLOR0; };
PSIn vs_main(VSIn i) {
    PSIn o;
    float2 p = i.pos / uScreen * 2.0 - 1.0;
    o.pos = float4(p.x, -p.y, 0.0, 1.0);
    o.uv = i.uv;
    o.col = i.col;
    return o;
}
Texture2D    tex : register(t0);
SamplerState smp : register(s0);

float3 srgb_to_linear(float3 c) {
    float3 lo = c / 12.92;
    float3 hi = pow((c + 0.055) / 1.055, 2.4);
    float3 k = step(c, 0.04045);
    return lerp(hi, lo, k);
}

float4 ps_main(PSIn i) : SV_Target {
    float a = tex.Sample(smp, i.uv).r;
    float4 c = i.col * a;
    if (uSrgb > 0.5) { c.rgb = srgb_to_linear(c.rgb); }
    return c;
}
