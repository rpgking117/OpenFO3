// The face menu's screen effect: the game's ISTV.pso from shader package 13
// (Fallout3.exe's 0x1e effect, set up by 00b3b220, its constants written by
// FORenderedMenuRaceSex 0069b270), on stored values.
//
// The screen model carries two pieces of glass. The round one (texture x up to
// 0.56) is a television: red, green and blue each read from a slightly different
// place, darkening away from its middle (0.29, 0.28), brightened by the pulse.
// The small one reads the corner of the picture the menu's list is in, with
// scanlines.
//
// As with the Pip-Boy's effect (pipboy.wgsl) the result is what the game's render
// target would hold, handed on decoded to linear light, since the screen's material
// encodes what it samples back to stored values.

#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct Tv {
    // c0 Offsets: a texel of the picture.
    offsets: vec4<f32>,
    // c1 EffectParams: the pulse.
    effect: vec4<f32>,
    // c2 DistortParams: vertical scale, a random place, horizontal scale.
    distort: vec4<f32>,
    // c3 FakeDistortParams: vertical scale (0: none), how far it's run, horizontal scale.
    fake: vec4<f32>,
    // c4 TintColor: the terminal colour.
    tint: vec4<f32>,
}

@group(2) @binding(0) var<uniform> u: Tv;
@group(2) @binding(1) var src: texture_2d<f32>;
@group(2) @binding(2) var src_sampler: sampler;
@group(2) @binding(3) var band: texture_2d<f32>;
@group(2) @binding(4) var band_sampler: sampler;
@group(2) @binding(5) var scanlines: texture_2d<f32>;
@group(2) @binding(6) var scanlines_sampler: sampler;

fn decode(c: f32) -> f32 {
    if c <= 0.04045 {
        return c / 12.92;
    }
    return pow((c + 0.055) / 1.055, 2.4);
}

fn at(p: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(src, src_sampler, p, 0.0);
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let t = mesh.uv;

    // The round glass.
    let from_middle = length(vec2<f32>(0.29, 0.28) - t);
    var v = max(1.0 - 2.5 * from_middle, 0.0);
    v = v * u.effect.x + v;
    let o = u.offsets.xy;
    let right_up = at(vec2<f32>(t.x + o.x, t.y - o.y));
    let left_up = at(vec2<f32>(t.x - o.x, t.y - o.y));
    let left_down = at(vec2<f32>(t.x - o.x, t.y + o.y));
    let right_down = at(vec2<f32>(t.x + o.x, t.y + o.y));
    let here = at(t);
    let colour = vec3<f32>(
        right_down.r * 0.15 + right_up.r * 0.7 + left_down.r * 0.15,
        right_down.g * 0.15 + here.g * 0.7 + left_down.g * 0.15,
        right_down.b * 0.15 + left_up.b * 0.7 + left_down.b * 0.15,
    );
    var jitter = vec3<f32>(0.0);
    if u.distort.x > 0.0 {
        let p = vec2<f32>(0.5, t.y * u.distort.x - 4.0 * u.distort.y + 1.0);
        jitter = textureSampleLevel(band, band_sampler, p, 0.0).rgb * u.distort.z * 0.05;
    }
    let round_glass = vec4<f32>(
        (colour * v * 0.85 + 0.075) * 1.2 - jitter,
        (v * 0.85 + 0.075) * 1.2,
    );

    // The small glass.
    var passing = vec3<f32>(0.0);
    if u.fake.x > 0.0 {
        let p = vec2<f32>(0.5, t.y * u.fake.x - u.fake.y + 1.0);
        passing = textureSampleLevel(band, band_sampler, p, 0.0).rgb * u.fake.z * u.tint.rgb * 0.35;
    }
    let scan = textureSampleLevel(scanlines, scanlines_sampler, t * vec2<f32>(1.0, 135.0), 0.0).rgb;
    let corner = at(t * vec2<f32>(0.63, 0.65) + vec2<f32>(0.37, 0.34));
    let small_glass = vec4<f32>(
        (scan * 0.1 + corner.rgb) * clamp(scan + vec3<f32>(0.8), vec3<f32>(0.0), vec3<f32>(1.0)) + passing,
        corner.a,
    );

    var s = small_glass;
    if 0.56 - t.x >= 0.0 {
        s = round_glass;
    }
    s = clamp(s, vec4<f32>(0.0), vec4<f32>(1.0));
    return vec4<f32>(decode(s.r), decode(s.g), decode(s.b), s.a);
}
