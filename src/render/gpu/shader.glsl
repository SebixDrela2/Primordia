#version 400 core

// The renderer that paints the picture on the graphics card.
//
// The world, the light, the bloom and the sky are handed to the card
// as textures. Every cell of the world is a little surface: it has a
// texture, it faces the open air, and the light of the nearest lamp
// and of the sun falls on it. The sky has the sun and the moon in it,
// the stars, the clouds and the aurora.

uniform sampler2D t_cell;      // the cells of the region, with a border
uniform sampler2D t_cols;      // the tables of the columns
uniform sampler2D t_light;     // the light field
uniform sampler2D t_bloom;     // the bloom
uniform sampler2D t_cloudamt;  // the cloud of each row, at each anchor
uniform sampler2D t_rows;      // the band of colour of each row
uniform sampler2D t_fx;        // the effect of every element
uniform sampler2D t_depth;     // the tint of every depth
uniform sampler2D t_sin;       // the sines the effects need, cell by cell
uniform sampler2D t_tex;       // the tile of every element, side by side

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
uniform vec4 u_moon;     // moon x, moon y, phase, 0
uniform vec4 u_horizon;  // how much of the sun is up, how much of the moon
uniform vec4 u_bloom;    // bloom w, bloom h, strength, 0

out vec4 fragColor;

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
const int FX_SWAY = 16;
const int FX_SMOKE = 17;
const int FX_GLOW = 18;

// The atlas of the tiles: how big one tile is, how many tiles are in a
// row of it and how many rows it has. These must agree with the ones
// in render::textures.
const int TEX_TILE = 32;
const int TEX_COLS = 12;
const int TEX_ROWS = 8;
// How many cells of the world one tile covers.
const float TEX_CELLS = 3.0;

// How far into a cell the texture of the neighbour bleeds over.
const float BLEED = 0.5;

// One float out of a texture. The float was put in as its raw bits,
// so it comes back out as the very same number.
float ffetch(sampler2D t, ivec2 c) {
    vec4 v = texelFetch(t, c, 0);
    float bx = v.x * 255.0 + 0.5;
    float by = v.y * 255.0 + 0.5;
    float bz = v.z * 255.0 + 0.5;
    float bw = v.w * 255.0 + 0.5;
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
    float n = float(hash2(x, y) >> 8);
    return n / 16777216.0;
}

float smoothstepf(float edge0, float edge1, float x) {
    float t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    float u = 3.0 - 2.0 * t;
    return t * t * u;
}

// A hash of a point of the sky, for the clouds and the stars. It is
// done with whole numbers, because the sine of a hash is slow on a
// card and the sky is a lot of pixels.
float hash21(vec2 p) {
    uvec2 q = uvec2(ivec2(floor(p))) * uvec2(1597334673u, 3812015801u);
    uint n = (q.x ^ q.y) * 1597334673u;
    n = n ^ (n >> 15);
    return float(n) * (1.0 / 4294967296.0);
}

// The noise between the points of a grid.
float vnoise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    float a = hash21(i);
    float b = hash21(i + vec2(1.0, 0.0));
    float c = hash21(i + vec2(0.0, 1.0));
    float d = hash21(i + vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// The noise of two octaves: the clouds are made of it.
float fbm2(vec2 p) {
    float sum = 0.5 * vnoise(p);
    sum += 0.25 * vnoise(p * 2.03 + vec2(17.3, 9.1));
    return sum;
}

// The kind of the cell at this place of the region. The border of the
// cell texture holds the cells around the region, so the neighbours
// of a cell can be read without going outside the texture.
int kindAt(int rx, int ry) {
    vec4 v = texelFetch(t_cell, ivec2((rx + 1) * 3, ry + 1), 0);
    float k = v.x * 255.0 + 0.5;
    return int(floor(k));
}

// Is the cell at this place land?
bool isLand(int cx, int cy) {
    int kind = kindAt(cx, cy);
    return kind != EMPTY && kind != WATER;
}

bool isShore(int cx, int cy) {
    return isLand(cx - 1, cy) || isLand(cx + 1, cy)
        || isLand(cx, cy - 1) || isLand(cx, cy + 1);
}

// Which way the face of the cell looks: towards the open air.
vec3 surfaceNormal(int up, int down, int left, int right) {
    vec3 acc = vec3(0.0);
    float open = 0.0;
    if (up == EMPTY) { acc += vec3(0.0, -1.0, 0.0); open += 1.0; }
    if (down == EMPTY) { acc += vec3(0.0, 1.0, 0.0); open += 1.0; }
    if (left == EMPTY) { acc += vec3(-1.0, 0.0, 0.0); open += 1.0; }
    if (right == EMPTY) { acc += vec3(1.0, 0.0, 0.0); open += 1.0; }
    if (open <= 0.0) {
        // A cell that is buried in the rock has no face of its own.
        return vec3(0.0);
    }
    return acc / open;
}

// The tile of an element: the shade and the glint of it.
//
// The picture is read at the place of the world and not at the place
// of the cell, so that it runs through the whole field: a wall of
// stone is one surface and not a wall of squares.
vec2 tileOf(int kind, float tx, float ty) {
    int lx = min(int(tx * float(TEX_TILE)), TEX_TILE - 1);
    int ly = min(int(ty * float(TEX_TILE)), TEX_TILE - 1);
    int ax = (kind % TEX_COLS) * TEX_TILE + lx;
    int ay = (kind / TEX_COLS) * TEX_TILE + ly;
    vec3 t = texelFetch(t_tex, ivec2(ax, ay), 0).rgb;
    return vec2(0.45 + t.r * 1.3, t.g);
}

// The texture of the element, with the texture of the neighbour
// bleeding over the edge. Two cells of the same thing then look like
// one surface and not like a wall of squares.
vec2 elementTexture(int wx, int wy, int kind, int up, int down, int left, int right,
                    float su, float sv, float ppc) {
    // Where in the tile the place of the world is. The place of the
    // fragment, not the place of the cell, so that the picture runs
    // through the cell.
    float tx = fract((float(wx) + su) / TEX_CELLS);
    float ty = fract((float(wy) + sv) / TEX_CELLS);
    if (ppc < 2.0) {
        // The cell is smaller than a pixel: take the middle of the
        // cell, so that the picture is not all noise.
        tx = (floor(float(wx)) + 0.5) / TEX_CELLS;
        ty = (floor(float(wy)) + 0.5) / TEX_CELLS;
    }

    vec2 own = tileOf(kind, tx, ty);

    // When the cell is smaller on the screen than a pixel the bleed is
    // not worth the work.
    if (ppc < 2.0) {
        return own;
    }

    // The edges of the cell, and how near the neighbour is.
    float w_right = smoothstepf(1.0 - BLEED, 1.0, su);
    float w_left = smoothstepf(1.0 - BLEED, 1.0, 1.0 - su);
    float w_down = smoothstepf(1.0 - BLEED, 1.0, sv);
    float w_up = smoothstepf(1.0 - BLEED, 1.0, 1.0 - sv);

    vec2 sum = own;
    float weight = 1.0;

    if (w_right > 0.001 && right != kind && right < TEX_COLS * TEX_ROWS) {
        sum += tileOf(right, tx, ty) * w_right;
        weight += w_right;
    }
    if (w_left > 0.001 && left != kind && left < TEX_COLS * TEX_ROWS) {
        sum += tileOf(left, tx, ty) * w_left;
        weight += w_left;
    }
    if (w_down > 0.001 && down != kind && down < TEX_COLS * TEX_ROWS) {
        sum += tileOf(down, tx, ty) * w_down;
        weight += w_down;
    }
    if (w_up > 0.001 && up != kind && up < TEX_COLS * TEX_ROWS) {
        sum += tileOf(up, tx, ty) * w_up;
        weight += w_up;
    }

    return sum / weight;
}

// The sun, with its corona and its light on the sky around it.
vec3 sunDisc(float wxf, float wyf, vec3 rgb, float day) {
    // The sun is only in the sky when it is well up, and it fades in
    // and out the same way as the moon does.
    float seen = u_horizon.x;
    if (seen <= 0.002) {
        return rgb;
    }
    vec2 d = vec2(wxf - u_sun.x, wyf - u_sun.y);
    float dist = length(d);

    // The wide glow of the sun over the sky.
    float wide = 1.0 - smoothstepf(0.0, 260.0, dist);
    vec3 warm = vec3(255.0, 176.0, 92.0);
    rgb += warm * wide * wide * (0.16 + 0.30 * (1.0 - day)) * seen;

    // The corona, close around it.
    float corona = 1.0 - smoothstepf(4.0, 34.0, dist);
    rgb += warm * corona * corona * 0.45 * seen;

    // The disc itself.
    if (dist < 9.0) {
        float edge = (1.0 - smoothstepf(6.5, 9.0, dist)) * seen;
        vec3 core = mix(vec3(255.0, 214.0, 150.0), vec3(255.0, 252.0, 232.0), day);
        rgb = mix(rgb, core, edge);
    }
    return rgb;
}

// The craters of the moon, in the face of it.
const vec2 MOON_CRATERS[5] = vec2[5](
    vec2(-3.4, -2.6),
    vec2(2.6, 0.6),
    vec2(-0.6, 3.4),
    vec2(4.4, -3.2),
    vec2(-4.6, 2.2)
);

// The dark patches of the moon.
const vec2 MOON_MARIA[3] = vec2[3](
    vec2(-2.0, -1.0),
    vec2(3.0, -2.5),
    vec2(1.0, 3.5)
);

// The moon: a face with the maria and the craters on it, and the dark
// of the moon on the side that the sun does not reach.
vec3 moonDisc(float wxf, float wyf, vec3 rgb) {
    // The moon is only in the sky when it is well up, and it fades in
    // and out the same way as the sun does.
    float seen = u_horizon.y;
    if (seen <= 0.002) {
        return rgb;
    }
    vec2 d = vec2(wxf - u_moon.x, wyf - u_moon.y);
    float dist = length(d);
    float r = 9.0;

    // The halo of the moon over the sky.
    float halo = 1.0 - smoothstepf(3.0, 54.0, dist);
    rgb += vec3(138.0, 156.0, 205.0) * halo * halo * 0.20 * seen;

    if (dist > r + 1.0) {
        return rgb;
    }

    // The face of the moon: the maria, and the craters with their
    // bright rims. The face turns a little over the month, so that
    // the moon is not the same every night.
    float turn = u_moon.z * 6.2831853;
    float face = 0.86 + vnoise(vec2(wxf * 0.8, wyf * 0.8)) * 0.12;
    for (int i = 0; i < 3; i++) {
        float a = turn + float(i) * 2.0944;
        vec2 centre = MOON_MARIA[i];
        vec2 turned = vec2(
            centre.x * cos(a) - centre.y * sin(a),
            centre.x * sin(a) + centre.y * cos(a)
        );
        float md = length(d - turned);
        face -= (1.0 - smoothstepf(0.0, 4.5, md)) * 0.26;
    }
    for (int i = 0; i < 5; i++) {
        float cd = length(d - MOON_CRATERS[i]);
        if (cd < 3.0) {
            // The rim of the crater is bright and the bowl is dark.
            face += (1.0 - smoothstepf(1.6, 2.9, cd)) * (1.0 - smoothstepf(0.8, 1.8, cd)) * 0.55;
            face -= (1.0 - smoothstepf(0.0, 1.6, cd)) * 0.20;
        }
    }

    // The whole disc, with a soft edge, and the edge of the moon a
    // little darker than the middle of it, so that it is round.
    float disc = (1.0 - smoothstepf(r - 0.7, r + 0.7, dist)) * seen;
    float limb = sqrt(max(0.0, 1.0 - (dist / r) * (dist / r)));
    vec3 stone = vec3(244.0, 246.0, 252.0) * face * (0.70 + 0.30 * limb);
    rgb = mix(rgb, stone, disc);
    return rgb;
}

// The stars, with their twinkle and the band of the milky way.
//
// Every cell has one star in it, but the star is a small point and not
// the whole cell: the point is put at a random place in the cell and
// the light of it falls off with the distance.
vec3 stars(int wx, int wy, float su, float sv, vec3 rgb, float ground) {
    if (u_sky.x >= 0.42 || float(wy) > ground * 0.75) {
        return rgb;
    }
    float night = 1.0 - u_sky.x / 0.42;

    // The band of the milky way, across the sky.
    float band = abs((float(wx) * 0.6 + float(wy) * 1.7) / 190.0 - 0.5);
    float milky = (1.0 - smoothstepf(0.0, 0.22, band)) * night * 0.16;
    if (milky > 0.001) {
        float dust = vnoise(vec2(float(wx) * 0.05, float(wy) * 0.09));
        rgb += vec3(120.0, 128.0, 170.0) * milky * (0.4 + dust);
    }

    // The star itself. The cell is only a star if the hash says so,
    // and the point of it is at a random place in the cell.
    float h = noiseAt(wx, wy);
    if (h < 0.988) {
        return rgb;
    }
    float px = hash21(vec2(float(wx), float(wy) * 1.7));
    float py = hash21(vec2(float(wx) * 2.3 + 11.0, float(wy)));
    vec2 d = vec2(su - px, sv - py);
    float dist = length(d);
    float radius = 0.16 + h * 0.18;
    float bright = 1.0 - smoothstepf(0.0, radius, dist);
    float twinkle = 0.6 + 0.4 * sin(u_sun.z * (1.4 + h * 3.0) + h * 40.0);
    float power = bright * bright * twinkle * night * (0.35 + (h - 0.988) * 70.0);
    vec3 tint = mix(vec3(255.0, 245.0, 225.0), vec3(200.0, 220.0, 255.0), h);
    rgb += tint * power * 210.0;
    return rgb;
}

// The gradient of the sky: the deep blue above, the pale band at the
// horizon, and the warm light of the sun in the west.
vec3 skyGradient(int wx, int wy, float su, float sv, float ground, float ground_inv,
                 vec3 sky_base, vec3 sky_slope) {
    float depth = float(wy) * ground_inv;
    float sky_depth = clamp(depth, 0.0, 1.0);
    vec3 rgb = sky_base + sky_depth * sky_slope;

    // The band of the horizon: the sky is paler close to the ground.
    float horizon = 1.0 - smoothstepf(0.0, 0.30, sky_depth);
    rgb += vec3(30.0, 36.0, 46.0) * horizon * horizon * u_sky.x;

    // The warm light of the rising and the setting sun.
    float sun_low = 1.0 - smoothstepf(0.05, 0.45, u_sky.x);
    float to_sun = 1.0 - smoothstepf(0.0, 260.0, abs(float(wx) - u_sun.x));
    rgb += vec3(96.0, 44.0, 16.0) * sun_low * to_sun * horizon;
    rgb += vec3(34.0, 20.0, 12.0) * sun_low * to_sun * (1.0 - horizon) * 0.5;

    rgb = stars(wx, wy, su, sv, rgb, ground);

    return rgb;
}

// The sky above the ground.
vec3 paintSky(int wx, int wy, int rx, int ry, float ground, float ground_inv,
              vec3 sky_base, vec3 sky_slope, float biome, float aurora_col,
              float su, float sv) {
    vec3 rgb = skyGradient(wx, wy, su, sv, ground, ground_inv, sky_base, sky_slope);

    // The aurora, over the tundra.
    if (u_sky4.w > 0.5 && biome == float(TUNDRA)) {
        float band = aurora_col - float(wy);
        float ab = abs(band);
        if (ab < 30.0) {
            float a = 1.0 - ab / 30.0;
            float d = u_sky.x / 0.32;
            float amt = a * 0.5 * (1.0 - d);
            vec3 aurora = band > 0.0
                ? vec3(90.0, 255.0, 170.0)
                : vec3(150.0, 110.0, 255.0);
            rgb = rgb + (aurora - rgb) * amt;
        }
    }

    // The clouds. The amount of them comes from the table of the row,
    // and the shape of them is worked out here, so that they have an
    // edge and a belly and not one flat colour.
    if (float(wy) < ground * 0.75) {
        float cover = ffetch(t_cloudamt, ivec2(rx / 3, ry));
        float t = u_sun.z * 0.012;
        vec2 p = vec2(float(wx) * 0.045 + t, float(wy) * 0.080);
        float n = fbm2(p);
        float n2 = vnoise(p * 1.7 + vec2(3.3, 1.1) + t * 0.7);
        float body = smoothstepf(0.50 - cover * 0.40, 0.78 - cover * 0.26, n);
        if (body > 0.001) {
            // The top of the cloud catches the sun, the belly is grey.
            float lit = smoothstepf(0.30, 0.72, n2);
            vec3 top = mix(u_sky4.rgb, vec3(255.0, 248.0, 236.0), lit * (0.45 + 0.5 * u_sky.x));
            vec3 belly = mix(u_sky4.rgb * 0.55, vec3(104.0, 110.0, 132.0), 0.5);
            vec3 cloud = mix(belly, top, lit);
            // The edge of the cloud is thin and lets the sky through.
            float soft = body * (0.45 + 0.55 * body);
            rgb = rgb + (cloud - rgb) * soft * (0.55 + 0.45 * cover);
        }
    }

    return rgb;
}

// The dark ground under the surface.
vec3 paintGround(int wx, int wy, float ground) {
    float below = float(wy) - ground;
    float under = max(float(u_world.y) - ground, 1.0);
    float depth = clamp(below / under, 0.0, 1.0);
    float base = 9.0 - depth * 4.0;
    float warmth = noiseAt(wx, wy) * 1.5;
    float r = base * 0.7 + warmth;
    float g = base * 0.9 + warmth;
    float b = base * 1.2 + warmth;
    return vec3(r, g, b);
}

// Put the element's own effect on the colour of the cell.
//
// The sines are not worked out here: they are worked out on the
// processor and handed over, cell by cell, and they are read from the
// texture.
vec3 applyFx(int wx, int wy, int rx, int ry, int kind, float heat,
             float noise_value, vec3 sky_base, float row_band, vec3 rgb_in) {
    vec3 rgb = rgb_in;
    int fk = int(ffetch(t_fx, ivec2(kind, 0)));
    if (fk == FX_NONE) {
        return rgb;
    }
    vec3 fc = vec3(ffetch(t_fx, ivec2(kind, 1)),
                   ffetch(t_fx, ivec2(kind, 2)),
                   ffetch(t_fx, ivec2(kind, 3)));
    float gain = ffetch(t_fx, ivec2(kind, 5));
    float s1 = ffetch(t_sin, ivec2(rx * 3, ry));

    if (fk == FX_WAVE) {
        float wave = s1 * 0.5 + 0.5;
        rgb = rgb + wave * fc;

        rgb = rgb + sky_base * vec3(0.02, 0.02, 0.03);

        float ccol = ffetch(t_cols, ivec2(rx, 16));
        float crow = ffetch(t_rows, ivec2(ry, 1));
        float c1 = (ccol + crow) * 0.5;
        float c2 = c1 * c1 * 2.0;
        float caustic = max(c2, 0.0) * 24.0;
        rgb = rgb + caustic * vec3(0.7, 0.92, 1.0);

        if (isShore(rx, ry)) {
            float surf = ffetch(t_sin, ivec2(rx * 3 + 1, ry)) * 0.5 + 0.5;
            float foam = (0.35 + 0.65 * surf) * 46.0;
            rgb = rgb + foam * vec3(0.9, 0.95, 1.0);
        }
    } else if (fk == FX_FLICKER) {
        float s = s1 * 0.5 + 0.5;
        float flicker = 0.7 + 0.3 * s;
        rgb = rgb * 0.9 + fc * flicker;
    } else if (fk == FX_HEAT) {
        float pulse = 0.5 + 0.5 * s1;
        float hot = 40.0 + heat * 0.35 + pulse * 38.0;
        rgb = rgb + hot * fc;
    } else if (fk == FX_PULSE || fk == FX_MYSTIC || fk == FX_EMBER) {
        float pulse = 0.5 + 0.5 * s1;
        rgb = rgb + fc * pulse;
    } else if (fk == FX_SPARKLE) {
        float s = s1 * 0.5 + 0.5;
        float sparkle = s * 26.0 * noise_value;
        rgb = rgb + sparkle * fc;
    } else if (fk == FX_FOLIAGE) {
        float foliage_amount = (noise_value - 0.5) * 26.0;
        vec3 add = fc * foliage_amount / 26.0;
        rgb = max(rgb + add, vec3(0.0));
    } else if (fk == FX_GLINT) {
        rgb.b = rgb.b + 10.0;
        float s = s1 * 0.5 + 0.5;
        float glint = s * 8.0;
        rgb = rgb + glint * fc;
    } else if (fk == FX_GRAIN) {
        rgb.r = rgb.r + fc.r * noise_value;
        rgb.g = rgb.g + fc.g * noise_value;
    } else if (fk == FX_SHEEN) {
        float s = s1 * 0.5 + 0.5;
        float sheen = s * 10.0;
        rgb = rgb + sheen * fc;
    } else if (fk == FX_ROT) {
        float beat = s1 * 0.5 + 0.5;
        float rot = 0.35 + beat * 0.65;
        rgb = rgb + fc * rot;
        rgb = rgb * vec3(0.86, 0.9, 0.88);
    } else if (fk == FX_FOUL) {
        float s = s1 * 0.5 + 0.5;
        float shimmer = s * gain;
        rgb = rgb + fc * shimmer;
    } else if (fk == FX_STRATA) {
        float a = row_band * 0.9;
        float b = noise_value * 0.4;
        float amt = a + b;
        rgb = rgb + fc * amt;
    } else if (fk == FX_VEIN) {
        float s = s1 * 0.5 + 0.5;
        float t = s * 0.7;
        float u = noise_value * 0.6;
        float twinkle = t + u;
        float amt = twinkle * gain;
        rgb = rgb + fc * amt;
    } else if (fk == FX_SWAY) {
        // The leaves move in the wind: a shade that travels over them.
        float s = s1 * 0.5 + 0.5;
        float breeze = sin(float(wx) * 0.7 + float(wy) * 0.3 + u_sun.z * 2.2) * 0.5 + 0.5;
        float sway = (s * 0.6 + breeze * 0.4) - 0.5;
        rgb = rgb + fc * sway * 1.4;
        rgb = rgb * (1.0 + sway * 0.10);
    } else if (fk == FX_SMOKE) {
        // The smoke drifts up and thins out.
        float s = s1 * 0.5 + 0.5;
        float rise = fract(s + float(wy) * 0.03 + u_sun.z * 0.05);
        float thin = 1.0 - rise;
        float puff = fbm2(vec2(float(wx) * 0.18 + rise * 2.0, float(wy) * 0.18));
        rgb = rgb + fc * thin * (0.35 + puff * 0.9);
    } else if (fk == FX_GLOW) {
        // The soft light of the things that give off a glow.
        float s = s1 * 0.5 + 0.5;
        float pulse = 0.45 + 0.55 * s;
        rgb = rgb + fc * pulse;
        rgb = rgb * (1.0 + pulse * 0.12);
    }
    return rgb;
}

// A cell that holds something.
vec3 paintCell(int wx, int wy, int rx, int ry, int kind, float heat,
               vec3 cell_col, float ground, float ground_inv, vec3 biome_delta,
               float cloud_shadow, float sun_shaft, float vig_col,
               vec3 sky_base, float su, float sv, float ppc,
               int k_up, int k_down, int k_left, int k_right) {
    float noise_value = noiseAt(wx, wy);
    float below = float(wy) - ground;
    float depth = max(below, 0.0);

    // The colour of the cell, before the element's own effect.
    vec3 rgb = cell_col;

    // The texture of the element, with the texture of the neighbour
    // bleeding over the edge.
    if (kind >= 0 && kind < TEX_COLS * TEX_ROWS) {
        vec2 tex = elementTexture(wx, wy, kind, k_up, k_down, k_left, k_right,
                                  su, sv, ppc);
        rgb = rgb * tex.x + vec3(tex.y * 40.0);
    }

    // The face of the cell and the light that falls on it.
    vec3 normal = surfaceNormal(k_up, k_down, k_left, k_right);
    float lit = 1.0;

    // How much of the sky the cell can see. It is what tells a cell
    // in the open from one under a roof, and it changes gently from
    // one row to the next.
    float exposure = texelFetch(t_cell, ivec2((rx + 1) * 3 + 2, ry + 1), 0).w;

    if (normal.z == 0.0 && dot(normal, normal) > 0.0) {
        // The light of the sun, from where the sun is.
        vec2 to_sun = vec2(u_sun.x - float(wx), u_sun.y - float(wy));
        float sun_len = max(length(to_sun), 1.0);
        vec3 sun_dir = vec3(to_sun.x / sun_len, to_sun.y / sun_len, 0.0);
        float sun_diffuse = max(dot(normal, sun_dir), 0.0);
        float sun_light = smoothstepf(-0.12, 0.22, sun_dir.y) * u_sky.x;

        // The light of the nearest lamp.
        vec3 lamp = texelFetch(t_cell, ivec2((rx + 1) * 3 + 2, ry + 1), 0).xyz;
        vec3 lamp_dir = vec3(lamp.x * 2.0 - 1.0, lamp.y * 2.0 - 1.0, 0.0);
        float lamp_len = length(lamp_dir.xy);
        if (lamp_len > 0.001) {
            lamp_dir = lamp_dir / lamp_len;
        }
        float lamp_diffuse = max(dot(normal, lamp_dir), 0.0);
        float lamp_power = lamp.z * lamp_diffuse;

        lit = 0.42 + 0.50 * exposure
            + 0.26 * sun_diffuse * sun_light * exposure
            + 0.55 * lamp_power;

        // The shadow where the cell leans against the one next to it.
        float solid_r = k_right != EMPTY ? 1.0 : 0.0;
        float solid_l = k_left != EMPTY ? 1.0 : 0.0;
        float solid_d = k_down != EMPTY ? 1.0 : 0.0;
        float solid_u = k_up != EMPTY ? 1.0 : 0.0;
        float edge = smoothstepf(1.0 - BLEED, 1.0, su) * solid_r
                   + smoothstepf(1.0 - BLEED, 1.0, 1.0 - su) * solid_l
                   + smoothstepf(1.0 - BLEED, 1.0, sv) * solid_d
                   + smoothstepf(1.0 - BLEED, 1.0, 1.0 - sv) * solid_u;
        lit *= 1.0 - 0.16 * edge;
    } else {
        lit = 0.42 + 0.50 * exposure;
    }

    float d1 = depth * 0.0032;
    float depth_fade = clamp(1.0 - d1, 0.62, 1.0);
    float n1 = noise_value * 0.12;
    float shade = depth_fade * u_sky.y * (0.94 + n1) * cloud_shadow * lit;
    rgb = rgb * shade;

    if (depth > 26.0) {
        int z = min(int(depth) / 8, 31);
        vec3 zt = vec3(ffetch(t_depth, ivec2(z, 0)),
                       ffetch(t_depth, ivec2(z, 1)),
                       ffetch(t_depth, ivec2(z, 2)));
        rgb = rgb * zt;
    }

    rgb = rgb * vec3(u_sky.w, u_sky2.x, u_sky2.y);

    if (int(u_sky.z) == 3 && depth < 3.0) {
        float snow_fade = (1.0 - depth * (1.0 / 3.0)) * 0.6;
        vec3 snow = vec3(232.0, 238.0, 248.0);
        rgb = rgb + (snow - rgb) * snow_fade;
    }

    float d2 = depth * (1.0 / 60.0);
    float fade = clamp(1.0 - d2, 0.0, 1.0);
    rgb = rgb + biome_delta * fade;

    if (u_sky2.z > 0.0) {
        float d3 = depth * (1.0 / 90.0);
        float lim = clamp(1.0 - d3, 0.25, 1.0);
        float veiling = u_sky2.z * lim;
        rgb = rgb + (u_sky3.rgb - rgb) * veiling;
    }

    // The element's own effect.
    float row_band = ffetch(t_rows, ivec2(ry, 0));
    rgb = applyFx(wx, wy, rx, ry, kind, heat, noise_value, sky_base, row_band, rgb);

    // The light, the shafts of the sun and the bloom.
    int ly = max(int(floor(float(wy) / 4.0)) - int(u_grid.y), 0);
    if (ly < int(u_grid.w)) {
        int lc = clamp((wx >> 2) - int(u_grid.x), 0, int(u_grid.z) - 1);
        vec3 lf = vec3(ffetch(t_light, ivec2(lc, ly * 3)),
                       ffetch(t_light, ivec2(lc, ly * 3 + 1)),
                       ffetch(t_light, ivec2(lc, ly * 3 + 2)));
        rgb = rgb + lf;
    }

    if (depth < 26.0) {
        float d4 = depth * (1.0 / 26.0);
        float shaft = sun_shaft * (1.0 - d4);
        rgb = rgb + shaft * vec3(30.0, 27.0, 17.0);
    }

    if (u_sun.w > 0.5 && depth < 9.0) {
        float s = ffetch(t_sin, ivec2(rx * 3 + 2, ry));
        float d5 = depth * (1.0 / 9.0);
        float sh = s * 6.0 * (1.0 - d5);
        rgb = rgb + vec3(sh, sh * 0.8, sh * 0.35);
    }

    if (u_sky2.w > 0.0) {
        rgb = rgb + u_sky2.w * vec3(110.0, 116.0, 132.0);
    }

    int gcol = min(rx / 4 + 1, int(u_bloom.x) - 1);
    int grow = min(ry / 4, int(u_bloom.y) - 1);
    vec3 gl = vec3(ffetch(t_bloom, ivec2(gcol, grow * 3)),
                   ffetch(t_bloom, ivec2(gcol, grow * 3 + 1)),
                   ffetch(t_bloom, ivec2(gcol, grow * 3 + 2)));
    rgb = rgb + gl * u_bloom.z;

    float ny = (float(ry) / max(float(u_region.w), 1.0) - 0.5) * 2.0;
    float v1 = vig_col * 0.8;
    float v2 = (ny * ny) * 0.5;
    float vig = 1.0 - 0.30 * (v1 + v2);
    rgb = rgb * vig;

    return rgb;
}

void main() {
    // Which cell of the region this fragment shows.
    float fx = (gl_FragCoord.x - u_place.x) / u_place.z;
    float fy = (u_counts.w - u_place.y - gl_FragCoord.y) / u_place.w;
    float cell_fx = fx * u_region.z;
    float cell_fy = fy * u_region.w;
    int rx = clamp(int(floor(cell_fx)), 0, int(u_region.z) - 1);
    int ry = clamp(int(floor(cell_fy)), 0, int(u_region.w) - 1);

    // How far into the cell this fragment lies, and how many pixels of
    // the screen one cell takes.
    float su = cell_fx - float(rx);
    float sv = cell_fy - float(ry);
    float ppc_x = u_place.z / max(u_region.z, 1.0);
    float ppc_y = u_place.w / max(u_region.w, 1.0);
    float ppc = min(ppc_x, ppc_y);

    int wx = int(u_region.x) + rx;
    int wy = int(u_region.y) + ry;

    if (wx < 0 || wx >= int(u_world.x) || wy < 0 || wy >= int(u_world.y)) {
        fragColor = vec4(0.0, 0.0, 0.0, 0.0);
        return;
    }

    // The cell.
    ivec2 c0 = ivec2((rx + 1) * 3, ry + 1);
    vec4 ca = texelFetch(t_cell, c0, 0);
    vec4 cb = texelFetch(t_cell, c0 + ivec2(1, 0), 0);
    float ka = ca.x * 255.0 + 0.5;
    float ha = ca.y * 255.0 + 0.5;
    float cr = cb.x * 255.0 + 0.5;
    float cg = cb.y * 255.0 + 0.5;
    float cb2 = cb.z * 255.0 + 0.5;
    int kind = int(floor(ka));
    float heat = float(int(floor(ha)));
    vec3 cell_col = vec3(floor(cr), floor(cg), floor(cb2));

    // The tables of the column.
    vec3 sky_base = vec3(ffetch(t_cols, ivec2(rx, 0)),
                         ffetch(t_cols, ivec2(rx, 1)),
                         ffetch(t_cols, ivec2(rx, 2)));
    vec3 sky_slope = vec3(ffetch(t_cols, ivec2(rx, 3)),
                          ffetch(t_cols, ivec2(rx, 4)),
                          ffetch(t_cols, ivec2(rx, 5)));
    vec3 biome_delta = vec3(ffetch(t_cols, ivec2(rx, 6)),
                            ffetch(t_cols, ivec2(rx, 7)),
                            ffetch(t_cols, ivec2(rx, 8)));
    float aurora_col = ffetch(t_cols, ivec2(rx, 9));
    float cloud_shadow = ffetch(t_cols, ivec2(rx, 10));
    float sun_shaft = ffetch(t_cols, ivec2(rx, 11));
    float vig_col = ffetch(t_cols, ivec2(rx, 12));
    float ground = ffetch(t_cols, ivec2(rx, 13));
    float ground_inv = ffetch(t_cols, ivec2(rx, 14));
    float biome = ffetch(t_cols, ivec2(rx, 15));

    vec3 rgb;
    if (kind != EMPTY) {
        int k_up = kindAt(rx, ry - 1);
        int k_down = kindAt(rx, ry + 1);
        int k_left = kindAt(rx - 1, ry);
        int k_right = kindAt(rx + 1, ry);
        rgb = paintCell(wx, wy, rx, ry, kind, heat, cell_col, ground, ground_inv,
                        biome_delta, cloud_shadow, sun_shaft, vig_col, sky_base,
                        su, sv, ppc, k_up, k_down, k_left, k_right);
    } else if (float(wy) >= ground) {
        rgb = paintGround(wx, wy, ground);
    } else {
        rgb = paintSky(wx, wy, rx, ry, ground, ground_inv, sky_base, sky_slope,
                       biome, aurora_col, su, sv);

        int gcol = min(rx / 4 + 1, int(u_bloom.x) - 1);
        int grow = min(ry / 4, int(u_bloom.y) - 1);
        vec3 gl = vec3(ffetch(t_bloom, ivec2(gcol, grow * 3)),
                       ffetch(t_bloom, ivec2(gcol, grow * 3 + 1)),
                       ffetch(t_bloom, ivec2(gcol, grow * 3 + 2)));
        rgb = rgb + gl * u_bloom.z;

        // The sun and the moon are put on the sky after the bloom, so
        // that the glow of the world does not put dots on them.
        float wxf = float(wx) + su;
        float wyf = float(wy) + sv;
        rgb = sunDisc(wxf, wyf, rgb, u_sky.x);
        rgb = moonDisc(wxf, wyf, rgb);

        if (u_sky2.w > 0.0) {
            rgb = rgb + u_sky2.w * vec3(80.0, 86.0, 104.0);
        }

        float ny = (float(ry) / max(float(u_region.w), 1.0) - 0.5) * 2.0;
        float v1 = vig_col * 0.8;
        float v2 = (ny * ny) * 0.5;
        float vig = 1.0 - 0.30 * (v1 + v2);
        rgb = rgb * vig;
    }

    // The picture is cut off at the byte, and the card would round it
    // instead, so the cut is made here.
    vec3 out_rgb = floor(max(rgb, vec3(0.0)));
    fragColor = vec4(out_rgb / 255.0, 1.0);
}
