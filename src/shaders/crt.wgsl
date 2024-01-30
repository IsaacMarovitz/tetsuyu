// =========================================================================== //
//                                                                             //
// CRT Shader                                                                  //
// Derived from: https://github.com/LIJI32/SameBoy/blob/master/Shaders/CRT.fsh //
//                                                                             //
// =========================================================================== //

const COLOR_LOW = 0.45;
const COLOR_HIGH = 1.0;
const VERTICAL_BORDER_DEPTH = 0.6;
const SCANLINE_DEPTH = 0.55;
const CURVENESS = 0.3;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var position = in.tex_coords;

    // Curve and pixel ratio
    var y_curve = cos(position.x - 0.5) * CURVENESS + (1 - CURVENESS);
    var y_multiplier = 8.0 / 7.0 / y_curve;
    position.y *= y_multiplier;
    position.y -= (y_multiplier - 1) / 2;
    if (position.y < 0.0) {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }
    if (position.y > 1.0) {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }

    var x_curve = cos(position.y - 0.5) * CURVENESS + (1 - CURVENESS);
    var x_multiplier = 1/x_curve;
    position.x *= x_multiplier;
    position.x -= (x_multiplier - 1) / 2;
    if (position.y < 0.0) {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }
    if (position.y > 1.0) {
        return vec4(0.0, 0.0, 0.0, 0.0);
    }

    // Setting up common vars
    var pos = fract(position * globals.input_resolution);
    var sub_pos = fract(position * globals.input_resolution * 6);

    var center = textureSample(t_diffuse, s_diffuse, position, vec2(0, 0));
    var left = textureSample(t_diffuse, s_diffuse, position, vec2(-1, 0));
    var right = textureSample(t_diffuse, s_diffuse, position, vec2(1, 0));

    // Vertical blurring
    if (pos.y < 1.0 / 6.0) {
        center = mix(center, textureSample(t_diffuse, s_diffuse, position, vec2( 0, -1)), 0.5 - sub_pos.y / 2.0);
        left =   mix(left,   textureSample(t_diffuse, s_diffuse, position, vec2(-1, -1)), 0.5 - sub_pos.y / 2.0);
        right =  mix(right,  textureSample(t_diffuse, s_diffuse, position, vec2( 1, -1)), 0.5 - sub_pos.y / 2.0);
    }
    else if (pos.y > 5.0 / 6.0) {
        center = mix(center, textureSample(t_diffuse, s_diffuse, position, vec2( 0, 1)), sub_pos.y / 2.0);
        left =   mix(left,   textureSample(t_diffuse, s_diffuse, position, vec2(-1, 1)), sub_pos.y / 2.0);
        right =  mix(right,  textureSample(t_diffuse, s_diffuse, position, vec2( 1, 1)), sub_pos.y / 2.0);
    }

    // Scanlines
    var scanline_multiplier = 0.0;
    if (pos.y < 0.5) {
        scanline_multiplier = (pos.y * 2) * SCANLINE_DEPTH + (1 - SCANLINE_DEPTH);
    }
    else  {
        scanline_multiplier = ((1 - pos.y) * 2) * SCANLINE_DEPTH + (1 - SCANLINE_DEPTH);
    }

    center *= scanline_multiplier;
    left *= scanline_multiplier;
    right *= scanline_multiplier;

    // Vertical seperator for shadow masks
    var odd = bool(u32((position * globals.input_resolution).x) & 1);
    if (odd) {
        pos.y += 0.5;
        pos.y = fract(pos.y);
    }

    if (pos.y < 1.0 / 3.0) {
        var gradient_position = pos.y * 3.0;
        center *= gradient_position * VERTICAL_BORDER_DEPTH + (1 - VERTICAL_BORDER_DEPTH);
        left *= gradient_position * VERTICAL_BORDER_DEPTH + (1 - VERTICAL_BORDER_DEPTH);
        right *= gradient_position * VERTICAL_BORDER_DEPTH + (1 - VERTICAL_BORDER_DEPTH);
    }
    else if (pos.y > 2.0 / 3.0) {
        var gradient_position = (1 - pos.y) * 3.0;
        center *= gradient_position * VERTICAL_BORDER_DEPTH + (1 - VERTICAL_BORDER_DEPTH);
        left *= gradient_position * VERTICAL_BORDER_DEPTH + (1 - VERTICAL_BORDER_DEPTH);
        right *= gradient_position * VERTICAL_BORDER_DEPTH + (1 - VERTICAL_BORDER_DEPTH);
    }

    // Blur the edges of the separators of adjacent columns
    if (pos.x < 1.0 / 6.0 || pos.x > 5.0 / 6.0) {
        pos.y += 0.5;
        pos.y = fract(pos.y);

        if (pos.y < 1.0 / 3.0) {
            var gradient_position = pos.y * 3.0;
            if (pos.x < 0.5) {
                gradient_position = 1 - (1 - gradient_position) * (1 - (pos.x) * 6.0);
            }
            else {
                gradient_position = 1 - (1 - gradient_position) * (1 - (1 - pos.x) * 6.0);
            }
            center *= gradient_position * VERTICAL_BORDER_DEPTH + (1 - VERTICAL_BORDER_DEPTH);
            left *= gradient_position * VERTICAL_BORDER_DEPTH + (1 - VERTICAL_BORDER_DEPTH);
            right *= gradient_position * VERTICAL_BORDER_DEPTH + (1 - VERTICAL_BORDER_DEPTH);
        }
        else if (pos.y > 2.0 / 3.0) {
            var gradient_position = (1 - pos.y) * 3.0;
            if (pos.x < 0.5) {
                gradient_position = 1 - (1 - gradient_position) * (1 - (pos.x) * 6.0);
            }
            else {
                gradient_position = 1 - (1 - gradient_position) * (1 - (1 - pos.x) * 6.0);
            }
            center *= gradient_position * VERTICAL_BORDER_DEPTH + (1 - VERTICAL_BORDER_DEPTH);
            left *= gradient_position * VERTICAL_BORDER_DEPTH + (1 - VERTICAL_BORDER_DEPTH);
            right *= gradient_position * VERTICAL_BORDER_DEPTH + (1 - VERTICAL_BORDER_DEPTH);
        }
    }


    // Subpixel blurring, like LCD filter

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

    // Anti alias the curve
    var pixel_position = position * globals.output_resolution;
    if (pixel_position.x < 1) {
        ret *= pixel_position.x;
    }
    else if (pixel_position.x > globals.output_resolution.x - 1) {
        ret *= globals.output_resolution.x - pixel_position.x;
    }
    if (pixel_position.y < 1) {
        ret *= pixel_position.y;
    }
    else if (pixel_position.y > globals.output_resolution.y - 1) {
        ret *= globals.output_resolution.y - pixel_position.y;
    }

    return ret;
}