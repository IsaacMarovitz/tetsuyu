struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var position = in.tex_coords;
    var input_resolution = vec2(160.0, 144.0);

    var SCANLINE_DEPTH: f32 = 0.25;
    var BLOOM: f32 = 0.4;

    var pixel = position * input_resolution - vec2(0.5, 0.5);

    var q11 = textureSample(t_diffuse, s_diffuse, (floor(pixel) + 0.5) / input_resolution);
    var q12 = textureSample(t_diffuse, s_diffuse, (vec2(floor(pixel.x), ceil(pixel.y)) + 0.5) / input_resolution);
    var q21 = textureSample(t_diffuse, s_diffuse, (vec2(ceil(pixel.x), floor(pixel.y)) + 0.5) / input_resolution);
    var q22 = textureSample(t_diffuse, s_diffuse, (ceil(pixel) + 0.5) / input_resolution);

    var s = vec2(smoothstep(0.0, 1.0, fract(pixel.x)), smoothstep(0.0, 1.0, fract(pixel.y)));

    var r1 = mix(q11, q21, s.x);
    var r2 = mix(q12, q22, s.x);

    var pos = fract(position * input_resolution);
    var sub_pos = fract(position * input_resolution * 6);

    var multiplier: f32 = 1.0;

    if (pos.y < 1.0 / 6.0) {
        multiplier *= sub_pos.y * SCANLINE_DEPTH + (1 - SCANLINE_DEPTH);
    }
    else if (pos.y > 5.0 / 6.0) {
        multiplier *= (1.0 - sub_pos.y) * SCANLINE_DEPTH + (1 - SCANLINE_DEPTH);
    }

    if (pos.x < 1.0 / 6.0) {
        multiplier *= sub_pos.x * SCANLINE_DEPTH + (1 - SCANLINE_DEPTH);
    }
    else if (pos.x > 5.0 / 6.0) {
        multiplier *= (1.0 - sub_pos.x) * SCANLINE_DEPTH + (1 - SCANLINE_DEPTH);
    }

    var pre_shadow = mix(textureSample(t_diffuse, s_diffuse, position) * multiplier, mix(r1, r2, s.y), BLOOM);
    pixel += vec2(-0.6, -0.8);

    q11 = textureSample(t_diffuse, s_diffuse, (floor(pixel) + 0.5) / input_resolution);
    q12 = textureSample(t_diffuse, s_diffuse, (vec2(floor(pixel.x), ceil(pixel.y)) + 0.5) / input_resolution);
    q21 = textureSample(t_diffuse, s_diffuse, (vec2(ceil(pixel.x), floor(pixel.y)) + 0.5) / input_resolution);
    q22 = textureSample(t_diffuse, s_diffuse, (ceil(pixel) + 0.5) / input_resolution);

    r1 = mix(q11, q21, fract(pixel.x));
    r2 = mix(q12, q22, fract(pixel.x));

    var shadow = mix(r1, r2, fract(pixel.y));
    return mix(min(shadow, pre_shadow), pre_shadow, 0.75);
}