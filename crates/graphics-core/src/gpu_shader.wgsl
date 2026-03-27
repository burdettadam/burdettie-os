// GPU renderer shader — quad batching + glyph atlas.
//
// Vertex: position (NDC), color (RGBA), tex_coord (UV), use_texture (0 or 1).
// When use_texture > 0.5, samples the glyph atlas R8 texture and uses
// its value as alpha, tinted by the vertex color.

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
    @location(3) use_texture: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) use_texture: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.color = in.color;
    out.tex_coord = in.tex_coord;
    out.use_texture = in.use_texture;
    return out;
}

@group(0) @binding(0) var atlas_texture: texture_2d<f32>;
@group(0) @binding(1) var atlas_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (in.use_texture > 0.5) {
        // Text rendering: sample alpha from glyph atlas, tint with vertex color
        let alpha = textureSample(atlas_texture, atlas_sampler, in.tex_coord).r;
        return vec4<f32>(in.color.rgb, in.color.a * alpha);
    } else {
        // Solid color rendering
        return in.color;
    }
}
