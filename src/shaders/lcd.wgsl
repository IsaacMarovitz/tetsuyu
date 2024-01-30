// =========================================================================== //
//                                                                             //
// LCD Shader                                                                  //
// Derived from: https://github.com/LIJI32/SameBoy/blob/master/Shaders/LCD.fsh //
//                                                                             //
// =========================================================================== //

const COLOR_LOW: f32 = 0.6;
const COLOR_HIGH: f32 = 1.0;
const SCANLINE_DEPTH_2: f32 = 0.2;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var position = in.tex_coords;

    var pos = fract(position * globals.input_resolution);
    var sub_pos = fract(position * globals.input_resolution * 6);

    var center = textureSample(t_diffuse, s_diffuse, position, vec2(0, 0));
    var left = textureSample(t_diffuse, s_diffuse, position, vec2(-1, 0));
    var right = textureSample(t_diffuse, s_diffuse, position, vec2(1, 0));

    if (pos.y < 1.0 / 6.0) {
        center = mix(center, textureSample(t_diffuse, s_diffuse, position, vec2( 0, -1)), 0.5 - sub_pos.y / 2.0);
        left =   mix(left,   textureSample(t_diffuse, s_diffuse, position, vec2(-1, -1)), 0.5 - sub_pos.y / 2.0);
        right =  mix(right,  textureSample(t_diffuse, s_diffuse, position, vec2( 1, -1)), 0.5 - sub_pos.y / 2.0);
        center *= sub_pos.y * SCANLINE_DEPTH_2 + (1 - SCANLINE_DEPTH_2);
        left *= sub_pos.y * SCANLINE_DEPTH_2 + (1 - SCANLINE_DEPTH_2);
        right *= sub_pos.y * SCANLINE_DEPTH_2 + (1 - SCANLINE_DEPTH_2);
    }
    else if (pos.y > 5.0 / 6.0) {
        center = mix(center, textureSample(t_diffuse, s_diffuse, position, vec2( 0, 1)), sub_pos.y / 2.0);
        left =   mix(left,   textureSample(t_diffuse, s_diffuse, position, vec2(-1, 1)), sub_pos.y / 2.0);
        right =  mix(right,  textureSample(t_diffuse, s_diffuse, position, vec2( 1, 1)), sub_pos.y / 2.0);
        center *= (1.0 - sub_pos.y) * SCANLINE_DEPTH_2 + (1 - SCANLINE_DEPTH_2);
        left *= (1.0 - sub_pos.y) * SCANLINE_DEPTH_2 + (1 - SCANLINE_DEPTH_2);
        right *= (1.0 - sub_pos.y) * SCANLINE_DEPTH_2 + (1 - SCANLINE_DEPTH_2);
    }


    var midleft = mix(left, center, 0.5);
    var midright = mix(right, center, 0.5);

    var ret = vec4(0.0, 0.0, 0.0, 0.0);
    if (pos.x < 1.0 / 6.0) {
        ret = mix(vec4(COLOR_HIGH * center.r, COLOR_LOW * center.g, COLOR_HIGH * left.b, 1),
                  vec4(COLOR_HIGH * center.r, COLOR_LOW * center.g, COLOR_LOW  * left.b, 1),
                  sub_pos.x);
    }
    else if (pos.x < 2.0 / 6.0) {
        ret = mix(vec4(COLOR_HIGH * center.r, COLOR_LOW  * center.g, COLOR_LOW * left.b, 1),
                  vec4(COLOR_HIGH * center.r, COLOR_HIGH * center.g, COLOR_LOW * midleft.b, 1),
                  sub_pos.x);
    }
    else if (pos.x < 3.0 / 6.0) {
        ret = mix(vec4(COLOR_HIGH * center.r  , COLOR_HIGH * center.g, COLOR_LOW * midleft.b, 1),
                  vec4(COLOR_LOW  * midright.r, COLOR_HIGH * center.g, COLOR_LOW * center.b, 1),
                  sub_pos.x);
    }
    else if (pos.x < 4.0 / 6.0) {
        ret = mix(vec4(COLOR_LOW * midright.r, COLOR_HIGH * center.g , COLOR_LOW  * center.b, 1),
                  vec4(COLOR_LOW * right.r   , COLOR_HIGH  * center.g, COLOR_HIGH * center.b, 1),
                  sub_pos.x);
    }
    else if (pos.x < 5.0 / 6.0) {
        ret = mix(vec4(COLOR_LOW * right.r, COLOR_HIGH * center.g  , COLOR_HIGH * center.b, 1),
                  vec4(COLOR_LOW * right.r, COLOR_LOW  * midright.g, COLOR_HIGH * center.b, 1),
                  sub_pos.x);
    }
    else {
        ret = mix(vec4(COLOR_LOW  * right.r, COLOR_LOW * midright.g, COLOR_HIGH * center.b, 1),
                  vec4(COLOR_HIGH * right.r, COLOR_LOW * right.g  ,  COLOR_HIGH * center.b, 1),
                  sub_pos.x);
    }

    return ret;
}