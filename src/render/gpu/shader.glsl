#version 400 core

// The renderer that paints the picture on the graphics card.
//
// It does exactly what the renderer on the processor does, cell by
// cell. The world, the light, the bloom and the sky are handed to the
// card as textures. The floats in those textures are put in as their
// raw bits, so the card has the very same numbers the processor had.
//
// Every number that goes into the picture is marked precise. That
// stops the card from doing a sum and a product in one go, which
// would round the sum once instead of twice and put the picture a
// shade off the one the processor paints.

uniform sampler2D t_cell;      // the cells of the region, with a border
uniform sampler2D t_cols;      // the tables of the columns
uniform sampler2D t_light;     // the light field
uniform sampler2D t_bloom;     // the bloom
uniform sampler2D t_cloudamt;  // the cloud of each row, at each anchor
uniform sampler2D t_rows;      // the band of colour of each row
uniform sampler2D t_fx;        // the effect of every element
uniform sampler2D t_depth;     // the tint of every depth
uniform sampler2D t_sin;       // the sines the effects need, cell by cell
uniform sampler2D t_shim;      // the sine of the heatwave's shimmer

uniform vec4 u_region;   // x, y, w, h of the region, in cells
uniform vec4 u_world;    // w, h of the world, in cells
uniform vec4 u_sky;      // day, light, season, season red
uniform vec4 u_sky2;     // season green, season blue, haze, flash
uniform vec4 u_sky3;     // haze red, haze green, haze blue, cloud cover
uniform vec4 u_sky4;     // cloud red, cloud green, cloud blue, aurora
uniform vec4 u_grid;     // light x, light y, light w, light h
uniform vec4 u_place;    // where the picture goes on the screen
uniform vec4 u_counts;   // cloud w, cloud h, anchors, screen height
uniform vec4 u_sun;      // sun x, sun y, time, shimmer
uniform vec4 u_bloom;    // bloom w, bloom h, strength, 0

precise out vec4 fragColor;

const int EMPTY = 0;
const int WATER = 9;
const int TUNDRA = 0;
const int FX_NONE = 0;
const int FX_WAVE = 1;
const int FX_FLICKER = 2;
const int FX_HEAT = 3;
const int FX_PULSE = 4;
const int FX_MYSTIC = 5;
const int FX_SPARKLE = 6;
const int FX_EMBER = 7;
const int FX_FOLIAGE = 8;
const int FX_GLINT = 9;
const int FX_GRAIN = 10;
const int FX_SHEEN = 11;
const int FX_ROT = 12;
const int FX_FOUL = 13;
const int FX_STRATA = 14;
const int FX_VEIN = 15;

// One float out of a texture. The float was put in as its raw bits,
// so it comes back out as the very same number.
float ffetch(sampler2D t, ivec2 c) {
    precise vec4 v = texelFetch(t, c, 0);
    precise float bx = v.x * 255.0 + 0.5;
    precise float by = v.y * 255.0 + 0.5;
    precise float bz = v.z * 255.0 + 0.5;
    precise float bw = v.w * 255.0 + 0.5;
    uint bits = uint(floor(bx));
    bits = bits | (uint(floor(by)) << 8);
    bits = bits | (uint(floor(bz)) << 16);
    bits = bits | (uint(floor(bw)) << 24);
    return uintBitsToFloat(bits);
}

// The hash of a point, the same one the processor uses.
uint hash2(int x, int y) {
    uint h = (uint(x) * 0x9E3779B1u) ^ (uint(y) * 0x85EBCA77u);
    h = h ^ (h >> 15);
    h = h * 0x2C1B3C6Du;
    h = h ^ (h >> 12);
    return h;
}

float noiseAt(int x, int y) {
    precise float n = float(hash2(x, y) >> 8);
    return n / 16777216.0;
}

float smoothstepf(float edge0, float edge1, float x) {
    precise float t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    precise float u = 3.0 - 2.0 * t;
    return t * t * u;
}

// Is the cell at this place land? The border of the cell texture holds
// the cells around the region, and the cells outside the world are
// empty, so this answers the same as the processor does.
bool isLand(int cx, int cy) {
    precise vec4 v = texelFetch(t_cell, ivec2(cx * 2, cy), 0);
    precise float k = v.x * 255.0 + 0.5;
    int kind = int(floor(k));
    return kind != EMPTY && kind != WATER;
}

bool isShore(int cx, int cy) {
    return isLand(cx - 1, cy) || isLand(cx + 1, cy)
        || isLand(cx, cy - 1) || isLand(cx, cy + 1);
}

// The gradient of the sky, with the stars in it and the sun.
vec3 skyGradient(int wx, int wy, float ground, float ground_inv,
                 vec3 sky_base, vec3 sky_slope) {
    precise float depth = float(wy) * ground_inv;
    precise float sky_depth = clamp(depth, 0.0, 1.0);
    precise vec3 rgb = sky_base + sky_depth * sky_slope;

    if (u_sky.x < 0.38 && float(wy) < ground * 0.7) {
        precise float sn = noiseAt(wx, wy);
        if (sn > 0.9968) {
            precise float star = 90.0 + 110.0 * sn;
            rgb.r = max(rgb.r, star * 0.85);
            rgb.g = max(rgb.g, star * 0.9);
            rgb.b = max(rgb.b, star);
        }
    }

    int sdx = wx - int(u_sun.x);
    int sdy = wy - int(u_sun.y);
    if (abs(sdx) < 32 && abs(sdy) < 32) {
        precise float sd = float(sdx * sdx + sdy * sdy);
        precise float q = sd / 900.0;
        precise float halo = max(1.0 - sqrt(q), 0.0);
        precise float core = sd < 36.0 ? 1.0 : 0.0;
        // The sun is white and warm in the day, cold and pale at night.
        precise vec3 sun_col = u_sky.x > 0.25
            ? vec3(255.0, 238.0, 190.0)
            : vec3(200.0, 215.0, 245.0);
        precise float h2 = halo * halo;
        precise float amt = h2 * 0.55 + core;
        rgb = rgb + (sun_col - rgb) * amt;
    }

    return rgb;
}

// The sky above the ground.
vec3 paintSky(int wx, int wy, int rx, int ry, float ground, float ground_inv,
              vec3 sky_base, vec3 sky_slope, float biome, float aurora_col) {
    precise vec3 rgb = skyGradient(wx, wy, ground, ground_inv, sky_base, sky_slope);

    // The aurora, over the tundra.
    if (u_sky4.w > 0.5 && biome == float(TUNDRA)) {
        precise float band = aurora_col - float(wy);
        precise float ab = abs(band);
        if (ab < 30.0) {
            precise float a = 1.0 - ab / 30.0;
            precise float d = u_sky.x / 0.32;
            precise float amt = a * 0.5 * (1.0 - d);
            precise vec3 aurora = band > 0.0
                ? vec3(90.0, 255.0, 170.0)
                : vec3(150.0, 110.0, 255.0);
            rgb = rgb + (aurora - rgb) * amt;
        }
    }

    // The cloud. It is only worked out every third column and then
    // carried along the row, so the amount is read at the anchor.
    if (float(wy) < ground * 0.6) {
        precise float amt = ffetch(t_cloudamt, ivec2(rx / 3, ry));
        if (amt > 0.002) {
            rgb = rgb + (u_sky4.rgb - rgb) * amt;
        }
    }

    return rgb;
}

// The dark ground under the surface.
vec3 paintGround(int wx, int wy, float ground) {
    precise float below = float(wy) - ground;
    precise float under = max(float(u_world.y) - ground, 1.0);
    precise float depth = clamp(below / under, 0.0, 1.0);
    precise float base = 9.0 - depth * 4.0;
    precise float warmth = noiseAt(wx, wy) * 1.5;
    precise float r = base * 0.7 + warmth;
    precise float g = base * 0.9 + warmth;
    precise float b = base * 1.2 + warmth;
    return vec3(r, g, b);
}

// Put the element's own effect on the colour of the cell.
//
// The sines are not worked out here: the sine of the card is not the
// sine of the processor, so the processor hands them over, cell by
// cell, and they are read from the texture.
//
// The colour is worked on in a local of its own, because a local that
// is marked precise keeps the card from rounding the sum in a
// different order than the processor does.
precise vec3 applyFx(int wx, int wy, int rx, int ry, int kind, float heat,
             float noise_value, vec3 sky_base, float row_band, vec3 rgb_in) {
    precise vec3 rgb = rgb_in;
    precise int fk = int(ffetch(t_fx, ivec2(kind, 0)));
    if (fk == FX_NONE) {
        return rgb;
    }
    precise vec3 fc = vec3(ffetch(t_fx, ivec2(kind, 1)),
                           ffetch(t_fx, ivec2(kind, 2)),
                           ffetch(t_fx, ivec2(kind, 3)));
    precise float gain = ffetch(t_fx, ivec2(kind, 5));
    precise float s1 = ffetch(t_sin, ivec2(rx * 2, ry));

    if (fk == FX_WAVE) {
        precise float wave = s1 * 0.5 + 0.5;
        rgb = rgb + wave * fc;

        rgb = rgb + sky_base * vec3(0.02, 0.02, 0.03);

        precise float ccol = ffetch(t_cols, ivec2(rx, 16));
        precise float crow = ffetch(t_rows, ivec2(ry, 1));
        precise float c1 = (ccol + crow) * 0.5;
        precise float c2 = c1 * c1 * 2.0;
        precise float caustic = max(c2, 0.0) * 24.0;
        rgb = rgb + caustic * vec3(0.7, 0.92, 1.0);

        if (isShore(rx + 1, ry + 1)) {
            precise float surf = ffetch(t_sin, ivec2(rx * 2 + 1, ry)) * 0.5 + 0.5;
            precise float foam = (0.35 + 0.65 * surf) * 46.0;
            rgb = rgb + foam * vec3(0.9, 0.95, 1.0);
        }
    } else if (fk == FX_FLICKER) {
        precise float s = s1 * 0.5 + 0.5;
        precise float flicker = 0.7 + 0.3 * s;
        rgb = rgb * 0.9 + fc * flicker;
    } else if (fk == FX_HEAT) {
        precise float pulse = 0.5 + 0.5 * s1;
        precise float hot = 40.0 + heat * 0.35 + pulse * 38.0;
        rgb = rgb + hot * fc;
    } else if (fk == FX_PULSE || fk == FX_MYSTIC || fk == FX_EMBER) {
        precise float pulse = 0.5 + 0.5 * s1;
        rgb = rgb + fc * pulse;
    } else if (fk == FX_SPARKLE) {
        precise float s = s1 * 0.5 + 0.5;
        precise float sparkle = s * 26.0 * noise_value;
        rgb = rgb + sparkle * fc;
    } else if (fk == FX_FOLIAGE) {
        precise float foliage_amount = (noise_value - 0.5) * 26.0;
        precise vec3 add = fc * foliage_amount / 26.0;
        rgb = max(rgb + add, vec3(0.0));
    } else if (fk == FX_GLINT) {
        rgb.b = rgb.b + 10.0;
        precise float s = s1 * 0.5 + 0.5;
        precise float glint = s * 8.0;
        rgb = rgb + glint * fc;
    } else if (fk == FX_GRAIN) {
        rgb.r = rgb.r + fc.r * noise_value;
        rgb.g = rgb.g + fc.g * noise_value;
    } else if (fk == FX_SHEEN) {
        precise float s = s1 * 0.5 + 0.5;
        precise float sheen = s * 10.0;
        rgb = rgb + sheen * fc;
    } else if (fk == FX_ROT) {
        precise float beat = s1 * 0.5 + 0.5;
        precise float rot = 0.35 + beat * 0.65;
        rgb = rgb + fc * rot;
        rgb = rgb * vec3(0.86, 0.9, 0.88);
    } else if (fk == FX_FOUL) {
        precise float s = s1 * 0.5 + 0.5;
        precise float shimmer = s * gain;
        rgb = rgb + fc * shimmer;
    } else if (fk == FX_STRATA) {
        precise float a = fma(row_band, 0.9, 0.0);
        precise float b = fma(noise_value, 0.4, 0.0);
        precise float amt = a + b;
        precise vec3 add = fma(fc, vec3(amt), vec3(0.0));
        rgb = rgb + add;
    } else if (fk == FX_VEIN) {
        precise float s = s1 * 0.5 + 0.5;
        precise float t = fma(s, 0.7, 0.0);
        precise float u = fma(noise_value, 0.6, 0.0);
        precise float twinkle = t + u;
        precise float amt = twinkle * gain;
        precise vec3 add = fma(fc, vec3(amt), vec3(0.0));
        rgb = rgb + add;
    }
    return rgb;
}

// A cell that holds something.
vec3 paintCell(int wx, int wy, int rx, int ry, int kind, float heat,
               vec3 cell_col, float ground, float ground_inv, vec3 biome_delta,
               float cloud_shadow, float sun_shaft, float vig_col,
               vec3 sky_base) {
    precise float noise_value = noiseAt(wx, wy);
    precise float below = float(wy) - ground;
    precise float depth = max(below, 0.0);

    // The colour of the cell, before the element's own effect.
    precise vec3 rgb = cell_col;

    precise float d1 = depth * 0.0032;
    precise float depth_fade = clamp(1.0 - d1, 0.62, 1.0);
    precise float n1 = noise_value * 0.12;
    precise float shade = depth_fade * u_sky.y * (0.94 + n1) * cloud_shadow;
    rgb = rgb * shade;

    if (depth > 26.0) {
        int z = min(int(depth) / 8, 31);
        precise vec3 zt = vec3(ffetch(t_depth, ivec2(z, 0)),
                               ffetch(t_depth, ivec2(z, 1)),
                               ffetch(t_depth, ivec2(z, 2)));
        rgb = rgb * zt;
    }

    rgb = rgb * vec3(u_sky.w, u_sky2.x, u_sky2.y);

    if (int(u_sky.z) == 3 && depth < 3.0) {
        precise float snow_fade = (1.0 - depth * (1.0 / 3.0)) * 0.6;
        precise vec3 snow = vec3(232.0, 238.0, 248.0);
        rgb = rgb + (snow - rgb) * snow_fade;
    }

    precise float d2 = depth * (1.0 / 60.0);
    precise float fade = clamp(1.0 - d2, 0.0, 1.0);
    rgb = rgb + biome_delta * fade;

    if (u_sky2.z > 0.0) {
        precise float d3 = depth * (1.0 / 90.0);
        precise float lim = clamp(1.0 - d3, 0.25, 1.0);
        precise float veiling = u_sky2.z * lim;
        rgb = rgb + (u_sky3.rgb - rgb) * veiling;
    }

    // The element's own effect.
    precise float row_band = ffetch(t_rows, ivec2(ry, 0));
    rgb = applyFx(wx, wy, rx, ry, kind, heat, noise_value, sky_base, row_band, rgb);

    // The light, the shafts of the sun and the bloom.
    int ly = max(int(floor(float(wy) / 4.0)) - int(u_grid.y), 0);
    if (ly < int(u_grid.w)) {
        int lc = clamp((wx >> 2) - int(u_grid.x), 0, int(u_grid.z) - 1);
        precise vec3 lf = vec3(ffetch(t_light, ivec2(lc, ly * 3)),
                               ffetch(t_light, ivec2(lc, ly * 3 + 1)),
                               ffetch(t_light, ivec2(lc, ly * 3 + 2)));
        rgb = rgb + lf;
    }

    if (depth < 26.0) {
        precise float d4 = depth * (1.0 / 26.0);
        precise float shaft = sun_shaft * (1.0 - d4);
        rgb = rgb + shaft * vec3(30.0, 27.0, 17.0);
    }

    if (u_sun.w > 0.5 && depth < 9.0) {
        precise float s = ffetch(t_shim, ivec2(rx, ry));
        precise float d5 = depth * (1.0 / 9.0);
        precise float sh = s * 6.0 * (1.0 - d5);
        rgb = rgb + vec3(sh, sh * 0.8, sh * 0.35);
    }

    if (u_sky2.w > 0.0) {
        rgb = rgb + u_sky2.w * vec3(110.0, 116.0, 132.0);
    }

    int gcol = min(rx / 4 + 1, int(u_bloom.x) - 1);
    int grow = min(ry / 4, int(u_bloom.y) - 1);
    precise vec3 gl = vec3(ffetch(t_bloom, ivec2(gcol, grow * 3)),
                           ffetch(t_bloom, ivec2(gcol, grow * 3 + 1)),
                           ffetch(t_bloom, ivec2(gcol, grow * 3 + 2)));
    rgb = rgb + gl * u_bloom.z;

    precise float ny = (float(ry) / max(float(u_region.w), 1.0) - 0.5) * 2.0;
    precise float v1 = vig_col * 0.8;
    precise float v2 = (ny * ny) * 0.5;
    precise float vig = 1.0 - 0.30 * (v1 + v2);
    rgb = rgb * vig;

    return rgb;
}

void main() {
    // Which cell of the region this fragment shows. The picture is put
    // on the screen at the nearest-neighbour rule, the same rule the
    // texture of the processor renderer is drawn with.
    precise float fx = (gl_FragCoord.x - u_place.x) / u_place.z;
    precise float fy = (u_counts.w - u_place.y - gl_FragCoord.y) / u_place.w;
    int rx = clamp(int(floor(fx * u_region.z)), 0, int(u_region.z) - 1);
    int ry = clamp(int(floor(fy * u_region.w)), 0, int(u_region.w) - 1);

    int wx = int(u_region.x) + rx;
    int wy = int(u_region.y) + ry;

    if (wx < 0 || wx >= int(u_world.x) || wy < 0 || wy >= int(u_world.y)) {
        fragColor = vec4(0.0, 0.0, 0.0, 0.0);
        return;
    }

    // The cell.
    ivec2 c0 = ivec2((rx + 1) * 2, ry + 1);
    precise vec4 ca = texelFetch(t_cell, c0, 0);
    precise vec4 cb = texelFetch(t_cell, c0 + ivec2(1, 0), 0);
    precise float ka = ca.x * 255.0 + 0.5;
    precise float ha = ca.y * 255.0 + 0.5;
    precise float cr = cb.x * 255.0 + 0.5;
    precise float cg = cb.y * 255.0 + 0.5;
    precise float cb2 = cb.z * 255.0 + 0.5;
    int kind = int(floor(ka));
    float heat = float(int(floor(ha)));
    precise vec3 cell_col = vec3(floor(cr), floor(cg), floor(cb2));

    // The tables of the column.
    precise vec3 sky_base = vec3(ffetch(t_cols, ivec2(rx, 0)),
                                 ffetch(t_cols, ivec2(rx, 1)),
                                 ffetch(t_cols, ivec2(rx, 2)));
    precise vec3 sky_slope = vec3(ffetch(t_cols, ivec2(rx, 3)),
                                  ffetch(t_cols, ivec2(rx, 4)),
                                  ffetch(t_cols, ivec2(rx, 5)));
    precise vec3 biome_delta = vec3(ffetch(t_cols, ivec2(rx, 6)),
                                    ffetch(t_cols, ivec2(rx, 7)),
                                    ffetch(t_cols, ivec2(rx, 8)));
    precise float aurora_col = ffetch(t_cols, ivec2(rx, 9));
    precise float cloud_shadow = ffetch(t_cols, ivec2(rx, 10));
    precise float sun_shaft = ffetch(t_cols, ivec2(rx, 11));
    precise float vig_col = ffetch(t_cols, ivec2(rx, 12));
    precise float ground = ffetch(t_cols, ivec2(rx, 13));
    precise float ground_inv = ffetch(t_cols, ivec2(rx, 14));
    precise float biome = ffetch(t_cols, ivec2(rx, 15));

    precise vec3 rgb;
    if (kind != EMPTY) {
        rgb = paintCell(wx, wy, rx, ry, kind, heat, cell_col, ground, ground_inv,
                        biome_delta, cloud_shadow, sun_shaft, vig_col, sky_base);
    } else if (float(wy) >= ground) {
        rgb = paintGround(wx, wy, ground);
    } else {
        rgb = paintSky(wx, wy, rx, ry, ground, ground_inv, sky_base, sky_slope,
                       biome, aurora_col);

        int gcol = min(rx / 4 + 1, int(u_bloom.x) - 1);
        int grow = min(ry / 4, int(u_bloom.y) - 1);
        precise vec3 gl = vec3(ffetch(t_bloom, ivec2(gcol, grow * 3)),
                               ffetch(t_bloom, ivec2(gcol, grow * 3 + 1)),
                               ffetch(t_bloom, ivec2(gcol, grow * 3 + 2)));
        rgb = rgb + gl * u_bloom.z;

        if (u_sky2.w > 0.0) {
            rgb = rgb + u_sky2.w * vec3(80.0, 86.0, 104.0);
        }

        precise float ny = (float(ry) / max(float(u_region.w), 1.0) - 0.5) * 2.0;
        precise float v1 = vig_col * 0.8;
        precise float v2 = (ny * ny) * 0.5;
        precise float vig = 1.0 - 0.30 * (v1 + v2);
        rgb = rgb * vig;
    }

    // The processor casts the float to a byte, which cuts it off. The
    // card would round it instead, so the cut is made here and the
    // whole number is put into the picture.
    precise vec3 out_rgb = floor(max(rgb, vec3(0.0)));
    precise vec3 divided = out_rgb / 255.0;
    fragColor = vec4(divided, 1.0);
}
