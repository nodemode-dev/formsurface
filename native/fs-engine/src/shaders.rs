pub const ENV_COMMON: &str = r#"
struct Env {
    a: vec4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
    forward: vec4<f32>,
    look: vec4<f32>,
};

fn agx_curve(x: vec3<f32>) -> vec3<f32> {
    let x2 = x * x;
    let x4 = x2 * x2;
    return 15.5 * x4 * x2 - 40.14 * x4 * x + 31.96 * x4 - 6.868 * x2 * x + 0.4298 * x2 + 0.1191 * x - 0.00232;
}

fn hable(x: vec3<f32>) -> vec3<f32> {
    return ((x * (0.15 * x + 0.05) + 0.004) / (x * (0.15 * x + 0.5) + 0.06)) - 0.02 / 0.3;
}

fn view_transform(c: vec3<f32>) -> vec3<f32> {
    let mode = env.look.x;
    if (mode < 0.5) {
        return (c * (2.51 * c + 0.03)) / (c * (2.43 * c + 0.59) + 0.14);
    }
    if (mode < 1.5) {
        return clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    }
    if (mode < 2.5) {
        let inset = mat3x3<f32>(0.842479062253094, 0.0423282422610123, 0.0423756549057051, 0.0784335999999992, 0.878468636469772, 0.0784336, 0.0792237451477643, 0.0791661274605434, 0.879142973793104);
        let outset = mat3x3<f32>(1.19687900512017, -0.0528968517574562, -0.0529716355144438, -0.0980208811401368, 1.15190312990417, -0.0980434501171241, -0.0990297440797205, -0.0989611768448433, 1.15107367264116);
        var v = inset * max(c, vec3<f32>(1e-10));
        v = (clamp(log2(v), vec3<f32>(-12.47393), vec3<f32>(4.026069)) + 12.47393) / 16.500999;
        v = outset * agx_curve(v);
        return pow(clamp(v, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(2.2));
    }
    return hable(c * 2.0) / hable(vec3<f32>(11.2));
}

fn env_uv(d: vec3<f32>) -> vec2<f32> {
    let c = cos(env.a.y);
    let s = sin(env.a.y);
    let r = vec3<f32>(c * d.x - s * d.z, d.y, s * d.x + c * d.z);
    return vec2<f32>(atan2(r.x, -r.z) / 6.2831853 + 0.5, acos(clamp(r.y, -1.0, 1.0)) / 3.1415927);
}

fn env_sample(d: vec3<f32>, level: f32) -> vec3<f32> {
    let uv = env_uv(d);
    let s0 = textureSampleLevel(env_sharp, samp, uv, 0.0).rgb;
    let s1 = textureSampleLevel(env_mid, samp, uv, 0.0).rgb;
    let s2 = textureSampleLevel(env_small, samp, uv, 0.0).rgb;
    if (level < 1.0) { return mix(s0, s1, clamp(level, 0.0, 1.0)); }
    return mix(s1, s2, clamp(level - 1.0, 0.0, 1.0));
}
"#;

pub const BACKGROUND: &str = r#"
@group(0) @binding(0) var<uniform> env: Env;
@group(0) @binding(1) var env_sharp: texture_2d<f32>;
@group(0) @binding(2) var env_mid: texture_2d<f32>;
@group(0) @binding(3) var env_small: texture_2d<f32>;
@group(0) @binding(4) var samp: sampler;

struct BOut {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fs(i: FOut) -> BOut {
    if (env.a.x < 0.5 || env.a.w < 0.5) { discard; }
    let ndc = vec2<f32>(i.uv.x * 2.0 - 1.0, 1.0 - i.uv.y * 2.0);
    let d = normalize(env.forward.xyz + env.right.xyz * ndc.x * env.right.w + env.up.xyz * ndc.y * env.up.w);
    var color = env_sample(d, env.a.z * 2.0) * env.forward.w;
    color = view_transform(color);
    var o: BOut;
    o.color = vec4<f32>(pow(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2)), 1.0);
    o.depth = 0.99999;
    return o;
}
"#;

pub const TRACE: &str = r#"
struct Trace { params: vec4<f32>, };
@group(0) @binding(0) var<uniform> tr: Trace;
@group(0) @binding(1) var pos_map: texture_2d<f32>;
@group(0) @binding(2) var normal_map: texture_2d<f32>;
@group(0) @binding(3) var prev: texture_2d<f32>;
@group(0) @binding(4) var<storage, read> nodes: array<vec4<u32>>;
@group(0) @binding(5) var<storage, read> tris: array<vec4<f32>>;

fn slab(lo: vec3<f32>, hi: vec3<f32>, o: vec3<f32>, inv: vec3<f32>, tmax: f32) -> bool {
    let t0 = (lo - o) * inv;
    let t1 = (hi - o) * inv;
    let near = max(max(min(t0.x, t1.x), min(t0.y, t1.y)), min(t0.z, t1.z));
    let far = min(min(max(t0.x, t1.x), max(t0.y, t1.y)), max(t0.z, t1.z));
    return far >= max(near, 0.0) && near <= tmax;
}

fn nearest(o: vec3<f32>, d: vec3<f32>, tmax: f32, any_hit: bool) -> f32 {
    let inv = 1.0 / select(d, vec3<f32>(1e-12), abs(d) < vec3<f32>(1e-12));
    var best = tmax;
    let total = nodes[2u].y;
    var i = 0u;
    var guard = 0u;
    loop {
        if (i >= total || guard > total * 2u) { break; }
        guard = guard + 1u;
        let lo = nodes[i * 3u];
        let hi = nodes[i * 3u + 1u];
        let miss = nodes[i * 3u + 2u].x;
        if (!slab(bitcast<vec3<f32>>(lo.xyz), bitcast<vec3<f32>>(hi.xyz), o, inv, best)) {
            i = miss;
            continue;
        }
        let count = hi.w;
        let link = lo.w;
        if (count == 0u) {
            i = i + 1u;
            continue;
        }
        i = miss;
        {
            for (var t = link; t < link + count; t = t + 1u) {
                let a = tris[t * 3u].xyz;
                let e1 = tris[t * 3u + 1u].xyz - a;
                let e2 = tris[t * 3u + 2u].xyz - a;
                let pv = cross(d, e2);
                let det = dot(e1, pv);
                if (abs(det) < 1e-12) { continue; }
                let id = 1.0 / det;
                let tv = o - a;
                let u = dot(tv, pv) * id;
                if (u < 0.0 || u > 1.0) { continue; }
                let qv = cross(tv, e1);
                let v = dot(d, qv) * id;
                if (v < 0.0 || u + v > 1.0) { continue; }
                let dist = dot(e2, qv) * id;
                if (dist > 0.0 && dist < best) {
                    best = dist;
                    if (any_hit) { return best; }
                }
            }
        }
    }
    return best;
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn around(n: vec3<f32>, k: f32, count: f32, spin: f32) -> vec3<f32> {
    let u1 = (k + 0.5) / count;
    let phi = k * 2.39996323 + spin;
    let r = sqrt(u1);
    let local = vec3<f32>(r * cos(phi), r * sin(phi), sqrt(max(1.0 - u1, 0.0)));
    let up = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(n.y) > 0.9);
    let t = normalize(cross(up, n));
    let b = cross(n, t);
    return normalize(t * local.x + b * local.y + n * local.z);
}

@fragment
fn fs(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let px = vec2<i32>(frag.xy);
    let old = textureLoad(prev, px, 0);
    let wp = textureLoad(pos_map, px, 0);
    if (wp.w < 0.5) { return old; }
    let n = normalize(textureLoad(normal_map, px, 0).xyz);
    let rays = tr.params.x;
    let reach = tr.params.y;
    let radius = tr.params.z;
    let eps = radius * 1e-4;
    let spin = hash(frag.xy) * 6.2831853;
    var open = 0.0;
    var depth = 0.0;
    var inside = 0.0;
    for (var k = 0.0; k < rays; k = k + 1.0) {
        let d = around(n, k, rays, spin);
        if (nearest(wp.xyz + n * eps, d, reach, true) >= reach) { open = open + 1.0; }
        let t = nearest(wp.xyz - n * eps, around(-n, k, rays, spin), radius * 2.0, false);
        if (t < radius * 2.0) {
            depth = depth + t;
            inside = inside + 1.0;
        }
    }
    let ao = open / max(rays, 1.0);
    let thickness = clamp(select(1.0, depth / max(inside, 1.0) / radius, inside > 0.0), 0.0, 1.0);
    return vec4<f32>(ao, old.y, thickness, old.w);
}
"#;

pub const REPROJECT: &str = r#"
@group(0) @binding(0) var map: texture_2d<f32>;
@group(0) @binding(1) var src_color: texture_2d<f32>;
@group(0) @binding(2) var src_val: texture_2d<f32>;
@group(0) @binding(3) var src_cov: texture_2d<f32>;
@group(0) @binding(4) var src_mask: texture_2d<f32>;
@group(0) @binding(5) var samp: sampler;

struct ROut {
    @location(0) color: vec4<f32>,
    @location(1) val: vec4<f32>,
    @location(2) cov: vec4<f32>,
    @location(3) mask: vec4<f32>,
};

@fragment
fn fs(i: FOut) -> ROut {
    var o: ROut;
    let m = textureLoad(map, vec2<i32>(i.clip.xy), 0);
    if (m.z < 0.5) {
        o.color = vec4<f32>(0.0);
        o.val = vec4<f32>(0.0);
        o.cov = vec4<f32>(0.0);
        o.mask = vec4<f32>(1.0);
        return o;
    }
    o.color = textureSampleLevel(src_color, samp, m.xy, 0.0);
    o.val = textureSampleLevel(src_val, samp, m.xy, 0.0);
    o.cov = textureSampleLevel(src_cov, samp, m.xy, 0.0);
    o.mask = textureSampleLevel(src_mask, samp, m.xy, 0.0);
    return o;
}
"#;

pub const POST_COMMON: &str = r#"
struct Post {
    a: vec4<f32>,
    b: vec4<f32>,
    c: vec4<f32>,
    d: vec4<f32>,
    e: vec4<f32>,
    f: vec4<f32>,
};
fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}
"#;

pub const BLOOM: &str = r#"
@group(0) @binding(0) var<uniform> post: Post;
@group(0) @binding(1) var scene: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

@fragment
fn fs(i: FOut) -> @location(0) vec4<f32> {
    let px = 1.0 / vec2<f32>(textureDimensions(scene));
    let threshold = post.a.y;
    var sum = vec3<f32>(0.0);
    var w = 0.0;
    for (var y = -2; y <= 2; y = y + 1) {
        for (var x = -2; x <= 2; x = x + 1) {
            let k = (3.0 - abs(f32(x))) * (3.0 - abs(f32(y)));
            let c = textureSampleLevel(scene, samp, i.uv + vec2<f32>(f32(x), f32(y)) * px * 2.0, 0.0).rgb;
            let l = luma(c);
            let bright = max(l - threshold, 0.0) / max(1.0 - threshold, 1e-3);
            sum = sum + c * bright * k;
            w = w + k;
        }
    }
    return vec4<f32>(sum / w, 1.0);
}
"#;

pub const POST: &str = r#"
@group(0) @binding(0) var<uniform> post: Post;
@group(0) @binding(1) var scene: texture_2d<f32>;
@group(0) @binding(2) var bloom: texture_2d<f32>;
@group(0) @binding(3) var depth: texture_depth_2d;
@group(0) @binding(4) var samp: sampler;

fn linear_depth(uv: vec2<f32>) -> f32 {
    let dims = vec2<i32>(textureDimensions(depth));
    let q = clamp(vec2<i32>(uv * vec2<f32>(dims)), vec2<i32>(0), dims - vec2<i32>(1));
    let z = textureLoad(depth, q, 0);
    let near = post.e.x;
    let far = post.e.y;
    let a = far / (near - far);
    let b = near * far / (near - far);
    return b / (z + a);
}

fn base(uv: vec2<f32>) -> vec3<f32> {
    let aperture = post.c.y;
    if (aperture < 0.5) {
        return textureSampleLevel(scene, samp, uv, 0.0).rgb;
    }
    let px = 1.0 / vec2<f32>(textureDimensions(scene));
    let focus = post.c.z;
    let coc = clamp(abs(linear_depth(uv) - focus) / max(focus, 1e-3) * aperture * 1.5, 0.0, aperture);
    var sum = textureSampleLevel(scene, samp, uv, 0.0).rgb;
    if (coc < 0.75) {
        return sum;
    }
    var w = 1.0;
    for (var k = 1; k <= 16; k = k + 1) {
        let r = sqrt(f32(k) / 16.0) * coc;
        let t = f32(k) * 2.39996;
        sum = sum + textureSampleLevel(scene, samp, uv + vec2<f32>(cos(t), sin(t)) * r * px, 0.0).rgb;
        w = w + 1.0;
    }
    return sum / w;
}

fn bloom_at(uv: vec2<f32>) -> vec3<f32> {
    if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) { return vec3<f32>(0.0); }
    return textureSampleLevel(bloom, samp, uv, 0.0).rgb;
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@fragment
fn fs(i: FOut) -> @location(0) vec4<f32> {
    let uv = i.uv;
    let px = 1.0 / vec2<f32>(textureDimensions(scene));
    let aberration = post.c.x;
    var col: vec3<f32>;
    if (aberration > 0.0) {
        let off = (uv - 0.5) * aberration * 0.02;
        col = vec3<f32>(base(uv + off).r, base(uv).g, base(uv - off).b);
    } else {
        col = base(uv);
    }
    let sharpen = post.b.w;
    if (sharpen > 0.0) {
        let n = textureSampleLevel(scene, samp, uv + vec2<f32>(px.x, 0.0), 0.0).rgb + textureSampleLevel(scene, samp, uv - vec2<f32>(px.x, 0.0), 0.0).rgb
            + textureSampleLevel(scene, samp, uv + vec2<f32>(0.0, px.y), 0.0).rgb + textureSampleLevel(scene, samp, uv - vec2<f32>(0.0, px.y), 0.0).rgb;
        col = max(col + (col - n * 0.25) * sharpen, vec3<f32>(0.0));
    }
    let bpx = 1.0 / vec2<f32>(textureDimensions(bloom));
    if (post.a.x > 0.0) {
        var b = vec3<f32>(0.0);
        var w = 0.0;
        for (var y = -3; y <= 3; y = y + 1) {
            for (var x = -3; x <= 3; x = x + 1) {
                let k = exp(-f32(x * x + y * y) / 6.0);
                b = b + bloom_at(uv + vec2<f32>(f32(x), f32(y)) * bpx * 1.5) * k;
                w = w + k;
            }
        }
        col = col + b / w * post.a.x;
    }
    if (post.a.z > 0.0) {
        var g = vec3<f32>(0.0);
        let dirs = array<vec2<f32>, 4>(vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, -1.0));
        for (var d = 0; d < 4; d = d + 1) {
            for (var k = 1; k <= 10; k = k + 1) {
                g = g + bloom_at(uv + dirs[d] * f32(k) * bpx * 2.0) * pow(0.82, f32(k));
            }
        }
        col = col + g * post.a.z * 0.25;
    }
    if (post.a.w > 0.0) {
        let to_center = vec2<f32>(0.5) - uv;
        var f = vec3<f32>(0.0);
        let scales = array<f32, 4>(1.0, 0.55, 1.45, 0.3);
        let tints = array<vec3<f32>, 4>(vec3<f32>(1.0, 0.7, 0.4), vec3<f32>(0.4, 0.8, 1.0), vec3<f32>(0.8, 0.5, 1.0), vec3<f32>(0.6, 1.0, 0.6));
        for (var k = 0; k < 4; k = k + 1) {
            let g = uv + to_center * 2.0 * scales[k];
            let fade = 1.0 - smoothstep(0.0, 0.75, length(g - 0.5));
            let s = bloom_at(g);
            f = f + s * s * tints[k] * fade;
        }
        let halo_uv = uv + normalize(to_center + vec2<f32>(1e-5)) * 0.3;
        let hs = bloom_at(halo_uv);
        f = f + hs * hs * (1.0 - smoothstep(0.0, 0.7, length(halo_uv - 0.5))) * 0.5;
        col = col + f * post.a.w * 0.35;
    }
    col = col * post.d.w;
    col = (col - 0.5) * (1.0 + post.c.w) + 0.5;
    col = mix(vec3<f32>(luma(col)), col, post.d.x);
    col = col * vec3<f32>(1.0 + post.d.y * 0.1, 1.0 - post.d.z * 0.1, 1.0 - post.d.y * 0.1);
    let vignette = post.b.x;
    if (vignette > 0.0) {
        let aspect = post.f.x / max(post.f.y, 1.0);
        let r = length((uv - 0.5) * vec2<f32>(aspect, 1.0));
        let inner = mix(0.75, 0.15, post.b.y);
        col = col * mix(1.0, 1.0 - smoothstep(inner, 0.95, r), vignette);
    }
    let grain = post.b.z;
    if (grain > 0.0) {
        let n = hash(i.clip.xy + vec2<f32>(post.e.z * 17.0, post.e.z * 31.0)) - 0.5;
        col = col + n * grain * 0.25;
    }
    return vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
"#;

pub const VIEW: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    cursor: vec4<f32>,
    cursor_normal: vec4<f32>,
    params: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;
@group(0) @binding(1) var color_tex: texture_2d<f32>;
@group(0) @binding(2) var mat_tex: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var baked_normal: texture_2d<f32>;
@group(0) @binding(5) var quick_tex: texture_2d<f32>;
@group(0) @binding(6) var<uniform> env: Env;
@group(0) @binding(7) var env_sharp: texture_2d<f32>;
@group(0) @binding(8) var env_mid: texture_2d<f32>;
@group(0) @binding(9) var env_small: texture_2d<f32>;
@group(0) @binding(10) var<uniform> look: vec4<f32>;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>) -> VOut {
    var o: VOut;
    o.clip = cam.view_proj * vec4<f32>(p, 1.0);
    if (cam.params.w > 0.5) { o.clip = cam.view_proj * vec4<f32>(uv, 0.0, 1.0); }
    o.world = p;
    o.normal = n;
    o.uv = uv;
    return o;
}

fn ggx(nh: f32, a: f32) -> f32 {
    let a2 = a * a;
    let d = nh * nh * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * d * d);
}

fn sky(dir: vec3<f32>) -> vec3<f32> {
    let t = clamp(dir.y * 0.5 + 0.5, 0.0, 1.0);
    return mix(vec3<f32>(0.10, 0.10, 0.11), vec3<f32>(0.62, 0.68, 0.78), t) + vec3<f32>(1.2, 1.1, 1.0) * pow(max(dot(dir, normalize(vec3<f32>(0.5, 0.8, 0.3))), 0.0), 64.0);
}

@fragment
fn fs(i: VOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    if (cam.params.w > 0.5) {
        var flat_color = textureSampleLevel(color_tex, samp, i.uv, 0.0).rgb;
        let flat_channel = cam.cursor_normal.w;
        if (flat_channel > 0.5) {
            let fm = textureSampleLevel(mat_tex, samp, i.uv, 0.0);
            var gray = fm.r;
            if (flat_channel > 1.5) { gray = fm.g; }
            if (flat_channel > 2.5) { gray = fm.b; }
            if (flat_channel > 3.5) { gray = fm.a; }
            flat_color = vec3<f32>(pow(clamp(gray, 0.0, 1.0), 2.2));
        }
        if (cam.cursor.w > 0.0) {
            let d = distance(i.uv, cam.cursor.xy) / cam.cursor.w;
            flat_color = mix(flat_color, vec3<f32>(1.0), smoothstep(0.90, 0.95, d) * (1.0 - smoothstep(0.98, 1.03, d)) * 0.8);
        }
        let flat_quick = textureSampleLevel(quick_tex, samp, i.uv, 0.0).r;
        flat_color = mix(flat_color, vec3<f32>(0.75, 0.08, 0.06), (1.0 - clamp(flat_quick, 0.0, 1.0)) * 0.5 * cam.params.z);
        return vec4<f32>(pow(clamp(flat_color, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2)), 1.0);
    }
    let dims = vec2<f32>(textureDimensions(mat_tex));
    let texel = 1.0 / dims;
    let base = textureSample(color_tex, samp, i.uv).rgb;
    let m = textureSample(mat_tex, samp, i.uv);
    let hx = textureSample(mat_tex, samp, i.uv + vec2<f32>(texel.x, 0.0)).b - textureSample(mat_tex, samp, i.uv - vec2<f32>(texel.x, 0.0)).b;
    let hy = textureSample(mat_tex, samp, i.uv + vec2<f32>(0.0, texel.y)).b - textureSample(mat_tex, samp, i.uv - vec2<f32>(0.0, texel.y)).b;
    var n = normalize(i.normal);
    if (!front) { n = -n; }
    let dp1 = dpdx(i.world);
    let dp2 = dpdy(i.world);
    let duv1 = dpdx(i.uv);
    let duv2 = dpdy(i.uv);
    let dp2perp = cross(dp2, n);
    let dp1perp = cross(n, dp1);
    let t = dp2perp * duv1.x + dp1perp * duv2.x;
    let b = dp2perp * duv1.y + dp1perp * duv2.y;
    let invmax = inverseSqrt(max(dot(t, t), dot(b, b)) + 1e-20);
    let strength = cam.params.y * dims.x / 256.0;
    let bn = textureSample(baked_normal, samp, i.uv).xyz * 2.0 - 1.0;
    let nt = normalize(vec3<f32>(bn.x - hx * strength, bn.y - hy * strength, max(bn.z, 0.05)));
    n = normalize(t * invmax * nt.x + b * invmax * nt.y + n * nt.z);
    let v = normalize(cam.eye.xyz - i.world);
    let rough = clamp(m.r, 0.04, 1.0);
    let metal = clamp(m.g, 0.0, 1.0);
    let f0 = mix(vec3<f32>(0.04), base, metal);
    var color = vec3<f32>(0.0);
    let lights = array<vec3<f32>, 2>(normalize(vec3<f32>(0.5, 0.8, 0.3)), normalize(vec3<f32>(-0.6, 0.2, -0.7)));
    let power = array<f32, 2>(2.6, 0.8);
    for (var k = 0; k < 2; k = k + 1) {
        let l = lights[k];
        let h = normalize(l + v);
        let nl = max(dot(n, l), 0.0);
        let nv = max(dot(n, v), 1e-4);
        let f = f0 + (1.0 - f0) * pow(1.0 - max(dot(h, v), 0.0), 5.0);
        let a = rough * rough;
        let k2 = a * 0.5;
        let g = (nl / (nl * (1.0 - k2) + k2)) * (nv / (nv * (1.0 - k2) + k2));
        let spec = ggx(max(dot(n, h), 0.0), a) * g * f / (4.0 * nl * nv + 1e-4);
        let diff = (1.0 - f) * (1.0 - metal) * base / 3.14159265;
        color = color + (diff + spec) * nl * power[k] * (1.0 - env.a.x);
    }
    let r = reflect(-v, n);
    let fe = f0 + (max(vec3<f32>(1.0 - rough), f0) - f0) * pow(1.0 - max(dot(n, v), 0.0), 5.0);
    if (env.a.x > 0.5) {
        color = color + (1.0 - metal) * base * env_sample(n, 2.0) + env_sample(r, clamp(rough * 2.5, 0.0, 2.0)) * fe;
    } else {
        let env_spec = mix(sky(r), sky(n) * 0.6, rough * rough);
        color = color + (1.0 - metal) * base * sky(n) * 0.35 + env_spec * fe * 0.9;
    }
    let extra = clamp(m.a, 0.0, 1.0);
    if (look.x > 1.5 && look.x < 2.5) {
        let t = fract(sin(dot(floor(i.clip.xy), vec2<f32>(12.9898, 78.233))) * 43758.5453);
        if (extra < t) { discard; }
    }
    if (look.x > 2.5 && look.x < 3.5) { color = color * extra; }
    if (look.x > 3.5) { color = color + base * extra * 4.0; }
    color = color * cam.params.x;
    color = view_transform(color);
    if (cam.cursor.w > 0.0) {
        let d = distance(i.world, cam.cursor.xyz) / cam.cursor.w;
        let ring = smoothstep(0.90, 0.95, d) * (1.0 - smoothstep(0.98, 1.03, d));
        color = mix(color, vec3<f32>(1.0), ring * 0.8 * step(0.0, dot(normalize(i.normal), cam.cursor_normal.xyz) + 0.3));
    }
    let quick = textureSampleLevel(quick_tex, samp, i.uv, 0.0).r;
    color = mix(color, vec3<f32>(0.75, 0.08, 0.06), (1.0 - clamp(quick, 0.0, 1.0)) * 0.5 * cam.params.z);
    return vec4<f32>(pow(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2)), 1.0);
}
"#;

pub const PAINT: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    cursor: vec4<f32>,
    cursor_normal: vec4<f32>,
    params: vec4<f32>,
};
struct Stamp {
    center: vec4<f32>,
    normal: vec4<f32>,
    tangent: vec4<f32>,
    uv: vec4<f32>,
};
struct Stamps {
    count: vec4<u32>,
    brush: vec4<f32>,
    tip: vec4<f32>,
    stencil: vec4<f32>,
    items: array<Stamp, 32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;
@group(0) @binding(1) var<uniform> stamps: Stamps;
@group(0) @binding(2) var depth_tex: texture_depth_2d;
@group(0) @binding(3) var alpha_tex: texture_2d<f32>;
@group(0) @binding(4) var stencil_tex: texture_2d<f32>;
@group(0) @binding(5) var clamp_samp: sampler;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>) -> VOut {
    var o: VOut;
    o.clip = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    o.world = p;
    o.normal = n;
    o.uv = uv;
    return o;
}

fn to_pixels(world: vec3<f32>, size: vec2<f32>) -> vec2<f32> {
    let c = cam.view_proj * vec4<f32>(world, 1.0);
    let n = c.xy / c.w;
    return vec2<f32>((n.x * 0.5 + 0.5) * size.x, (0.5 - n.y * 0.5) * size.y);
}

fn rotate2(v: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(c * v.x + s * v.y, -s * v.x + c * v.y);
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn tip_shape(l: vec2<f32>, hardness: f32, seed: f32, texel: vec2<f32>) -> f32 {
    let kind = stamps.tip.x;
    var d = length(l);
    if (kind > 0.5 && kind < 1.5) { d = max(abs(l.x), abs(l.y)); }
    if (d >= 1.0) { return 0.0; }
    if (kind > 2.5) {
        return textureSampleLevel(alpha_tex, clamp_samp, vec2<f32>(l.x, -l.y) * 0.5 + 0.5, 0.0).r;
    }
    var c = 1.0;
    if (d > hardness && hardness < 1.0) {
        let t = (1.0 - d) / (1.0 - hardness);
        c = t * t * (3.0 - 2.0 * t);
    }
    if (kind > 1.5) {
        c = c * step(1.0 - stamps.tip.y, hash2(floor(texel) + vec2<f32>(seed * 17.0, seed * 31.0)));
    }
    return c;
}

@fragment
fn fs(i: VOut) -> @location(0) vec4<f32> {
    let n = normalize(i.normal);
    let hardness = stamps.brush.x;
    let flow = stamps.brush.y;
    let cull = stamps.brush.z;
    let align = stamps.brush.w;
    let clip = cam.view_proj * vec4<f32>(i.world, 1.0);
    let ndc = clip.xyz / clip.w;
    let size = vec2<f32>(textureDimensions(depth_tex));
    let frag_px = vec2<f32>((ndc.x * 0.5 + 0.5) * size.x, (0.5 - ndc.y * 0.5) * size.y);
    let px = vec2<i32>(frag_px);
    var visible = false;
    if (px.x >= 0 && px.y >= 0 && px.x < i32(size.x) && px.y < i32(size.y) && clip.w > 0.0) {
        let stored = textureLoad(depth_tex, px, 0);
        visible = ndc.z <= stored + 0.0015;
    }
    let facing = dot(n, normalize(cam.eye.xyz - i.world)) > 0.0;
    var stencil = 1.0;
    if (stamps.tip.z > 0.5 && stamps.items[0].normal.w < 1.5) {
        let suv = rotate2(frag_px / size - 0.5 - stamps.stencil.zw, stamps.stencil.y) / max(stamps.stencil.x, 0.01) + 0.5;
        if (any(suv < vec2<f32>(0.0)) || any(suv > vec2<f32>(1.0))) { stencil = 0.0; } else {
            stencil = textureSampleLevel(stencil_tex, clamp_samp, suv, 0.0).r;
        }
        if (stencil <= 0.0) { discard; }
    }
    let texel = i.clip.xy;
    var keep = 1.0;
    for (var k = 0u; k < stamps.count.x; k = k + 1u) {
        let s = stamps.items[k];
        let mirrored = s.normal.w > 0.5;
        if (!mirrored && !visible) { continue; }
        if (!mirrored && cull > 0.5 && !facing) { continue; }
        let in_uv = s.normal.w > 1.5;
        if (!in_uv && align < 2.5 && dot(n, s.normal.xyz) < 0.15) { continue; }
        let r = s.center.w;
        let off = i.world - s.center.xyz;
        let t = s.tangent.xyz;
        let b = cross(s.normal.xyz, t);
        var l = vec2<f32>(0.0);
        if (in_uv) {
            l = (i.uv - s.uv.xy) / max(s.uv.z, 1e-6);
            l = vec2<f32>(l.x, -l.y);
        } else if (align < 0.5) {
            if (mirrored) {
                l = vec2<f32>(dot(off, t), dot(off, b)) / r;
            } else {
                let center_px = to_pixels(s.center.xyz, size);
                let edge_px = to_pixels(s.center.xyz + t * r, size);
                let r_px = max(distance(center_px, edge_px), 0.5);
                if (abs(dot(off, s.normal.xyz)) > r * 2.0) { continue; }
                l = (frag_px - center_px) / r_px;
                l = vec2<f32>(l.x, -l.y);
            }
        } else if (align < 1.5) {
            let flat_l = vec2<f32>(dot(off, t), dot(off, b));
            let d3 = length(off) / r;
            let fl = length(flat_l);
            if (fl > 1e-6) { l = flat_l / fl * d3; } else { l = vec2<f32>(d3, 0.0); }
        } else if (align < 2.5) {
            if (abs(dot(off, s.normal.xyz)) > r) { continue; }
            l = vec2<f32>(dot(off, t), dot(off, b)) / r;
        } else if (mirrored) {
            if (abs(dot(off, s.normal.xyz)) > r) { continue; }
            l = vec2<f32>(dot(off, t), dot(off, b)) / r;
        } else {
            l = (i.uv - s.uv.xy) / max(s.uv.z, 1e-6);
            l = vec2<f32>(l.x, -l.y);
        }
        l = rotate2(l, s.tangent.w);
        let c = tip_shape(l, hardness, f32(k) + s.uv.w * 7.0, texel);
        if (c <= 0.0) { continue; }
        keep = keep * (1.0 - clamp(c * flow * s.uv.w, 0.0, 1.0));
    }
    keep = 1.0 - (1.0 - keep) * stencil;
    let a = 1.0 - keep;
    if (a <= 0.0) { discard; }
    return vec4<f32>(a, 0.0, 0.0, a);
}
"#;

pub const FULLSCREEN: &str = r#"
struct FOut { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32>, };
@vertex
fn vs(@builtin(vertex_index) i: u32) -> FOut {
    var o: FOut;
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    o.clip = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    o.uv = vec2<f32>(x, y);
    return o;
}
"#;

pub const APPLY: &str = r#"
struct Paint {
    color: vec4<f32>,
    values: vec4<f32>,
    flags: vec4<f32>,
    mode: vec4<f32>,
    extra: vec4<f32>,
};
@group(0) @binding(0) var<uniform> paint: Paint;
@group(0) @binding(1) var snap_color: texture_2d<f32>;
@group(0) @binding(2) var snap_val: texture_2d<f32>;
@group(0) @binding(3) var snap_cov: texture_2d<f32>;
@group(0) @binding(4) var mask: texture_2d<f32>;
@group(0) @binding(5) var clone_color: texture_2d<f32>;
@group(0) @binding(6) var clone_mat: texture_2d<f32>;
@group(0) @binding(7) var clone_cov: texture_2d<f32>;

struct Out {
    @location(0) color: vec4<f32>,
    @location(1) val: vec4<f32>,
    @location(2) cov: vec4<f32>,
};

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> Out {
    let p = vec2<i32>(pos.xy);
    var source_cov = vec4<f32>(1.0);
    let sc = textureLoad(snap_color, p, 0);
    let sv = textureLoad(snap_val, p, 0);
    let sk = textureLoad(snap_cov, p, 0);
    let a = clamp(textureLoad(mask, p, 0).r, 0.0, 1.0) * paint.mode.y;
    var o: Out;
    if (paint.mode.x > 0.5) {
        let k = 1.0 - a;
        o.color = vec4<f32>(sc.rgb * select(1.0, k, paint.flags.x > 0.5), sc.a * select(1.0, k, paint.flags.x > 0.5));
        let ks = vec3<f32>(select(1.0, k, paint.flags.y > 0.5), select(1.0, k, paint.flags.z > 0.5), select(1.0, k, paint.flags.w > 0.5));
        let ke = select(1.0, k, paint.extra.y > 0.5);
        o.val = vec4<f32>(sv.rgb * ks, sv.a * ke);
        o.cov = vec4<f32>(sk.rgb * ks, sk.a * ke);
        return o;
    }
    var src_extra = paint.extra.x;
    var extra_cov = 1.0;
    var src_color = paint.color.rgb;
    var src_values = paint.values.rgb;
    if (paint.mode.z > 0.5) {
        let dims = vec2<i32>(textureDimensions(clone_color));
        let q = clamp(p + vec2<i32>(i32(paint.color.w), i32(paint.values.w)), vec2<i32>(0), dims - vec2<i32>(1));
        let cc = textureLoad(clone_color, q, 0);
        let cm = textureLoad(clone_mat, q, 0);
        src_color = cc.rgb;
        src_values = cm.rgb;
        src_extra = cm.a;
        if (paint.mode.w > 0.5) {
            let ck = textureLoad(clone_cov, q, 0);
            source_cov = vec4<f32>(cc.a, ck.rgb);
            src_color = cc.rgb / max(cc.a, 1e-4);
            src_values = cm.rgb / max(ck.rgb, vec3<f32>(1e-4));
            extra_cov = ck.a;
            src_extra = cm.a / max(ck.a, 1e-4);
        }
    }
    let ac = a * paint.flags.x * source_cov.x;
    o.color = vec4<f32>(src_color * ac + sc.rgb * (1.0 - ac), ac + sc.a * (1.0 - ac));
    let am = a * paint.flags.yzw * source_cov.yzw;
    let ae = a * paint.extra.y * extra_cov;
    o.val = vec4<f32>(src_values * am + sv.rgb * (1.0 - am), src_extra * ae + sv.a * (1.0 - ae));
    o.cov = vec4<f32>(am + sk.rgb * (1.0 - am), ae + sk.a * (1.0 - ae));
    return o;
}
"#;

pub const BLEND: &str = r#"
fn rgb_to_hsv(c: vec3<f32>) -> vec3<f32> {
    let mx = max(c.r, max(c.g, c.b));
    let mn = min(c.r, min(c.g, c.b));
    let d = mx - mn;
    var h = 0.0;
    if (d > 1e-6) {
        if (mx == c.r) { h = (c.g - c.b) / d; }
        else if (mx == c.g) { h = 2.0 + (c.b - c.r) / d; }
        else { h = 4.0 + (c.r - c.g) / d; }
        h = fract(h / 6.0 + 1.0);
    }
    return vec3<f32>(h, select(0.0, d / mx, mx > 1e-6), mx);
}

fn hsv_to_rgb(c: vec3<f32>) -> vec3<f32> {
    let k = vec3<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0);
    let p = abs(fract(vec3<f32>(c.x) + k) * 6.0 - 3.0);
    return c.z * mix(vec3<f32>(1.0), clamp(p - 1.0, vec3<f32>(0.0), vec3<f32>(1.0)), c.y);
}

fn soft_light(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    let d = select(sqrt(a), ((16.0 * a - 12.0) * a + 4.0) * a, a <= vec3<f32>(0.25));
    return select(a + (2.0 * b - 1.0) * (d - a), a - (1.0 - 2.0 * b) * a * (1.0 - a), b <= vec3<f32>(0.5));
}

fn blend_any(m: u32, a: vec3<f32>, b: vec3<f32>, values: bool) -> vec3<f32> {
    let e = vec3<f32>(1e-4);
    switch m {
        case 1u: { return a * b; }
        case 2u: { return 1.0 - (1.0 - a) * (1.0 - b); }
        case 3u: { return select(1.0 - 2.0 * (1.0 - a) * (1.0 - b), 2.0 * a * b, a < vec3<f32>(0.5)); }
        case 4u: { return a + b; }
        case 5u: { return a - b; }
        case 6u: { return min(a, b); }
        case 7u: { return max(a, b); }
        case 8u: { return a; }
        case 9u: { return b; }
        case 10u: { return a / max(b, e); }
        case 11u: { return b / max(a, e); }
        case 12u: { return abs(a - b); }
        case 13u: { return a + b - 2.0 * a * b; }
        case 14u: { return a + 2.0 * b - 1.0; }
        case 15u: { return a + b - 1.0; }
        case 16u: { return 1.0 - (1.0 - a) / max(b, e); }
        case 17u: { return a / max(1.0 - b, e); }
        case 18u: { return soft_light(a, b); }
        case 19u: { return select(1.0 - 2.0 * (1.0 - a) * (1.0 - b), 2.0 * a * b, b < vec3<f32>(0.5)); }
        case 20u: { return select(a / max(2.0 * (1.0 - b), e), 1.0 - (1.0 - a) / max(2.0 * b, e), b < vec3<f32>(0.5)); }
        case 21u: { return a + 2.0 * b - 1.0; }
        case 22u: { return select(max(a, 2.0 * b - 1.0), min(a, 2.0 * b), b < vec3<f32>(0.5)); }
        case 23u: {
            if (values) { return b; }
            let l = dot(a, vec3<f32>(0.2126, 0.7152, 0.0722));
            return b * l / max(dot(b, vec3<f32>(0.2126, 0.7152, 0.0722)), 1e-4);
        }
        case 24u: {
            if (values) { return b; }
            let ha = rgb_to_hsv(a);
            return hsv_to_rgb(vec3<f32>(ha.x, rgb_to_hsv(b).y, ha.z));
        }
        case 25u: {
            if (values) { return b; }
            let hb = rgb_to_hsv(b);
            return hsv_to_rgb(vec3<f32>(hb.x, hb.y, rgb_to_hsv(a).z));
        }
        case 26u: {
            if (values) { return b; }
            let ha = rgb_to_hsv(a);
            return hsv_to_rgb(vec3<f32>(ha.x, ha.y, rgb_to_hsv(b).z));
        }
        case 27u, 28u: {
            if (!values) { return b; }
            return vec3<f32>(b.x, b.y, a.z + b.z - 0.5);
        }
        case 29u: {
            if (!values) { return b; }
            return vec3<f32>(b.x, b.y, a.z - (b.z - 0.5));
        }
        case 30u: { return b - a; }
        default: { return b; }
    }
}
"#;

pub const COMPOSITE: &str = r#"
struct Layer {
    opacity: vec4<f32>,
    mode: vec4<u32>,
    base_color: vec4<f32>,
    base_values: vec4<f32>,
    levels: vec4<f32>,
    style: vec4<f32>,
};
@group(0) @binding(0) var<uniform> layer: Layer;
@group(0) @binding(1) var prev_color: texture_2d<f32>;
@group(0) @binding(2) var prev_mat: texture_2d<f32>;
@group(0) @binding(3) var l_color: texture_2d<f32>;
@group(0) @binding(4) var l_val: texture_2d<f32>;
@group(0) @binding(5) var l_cov: texture_2d<f32>;
@group(0) @binding(6) var l_mask: texture_2d<f32>;
@group(0) @binding(7) var uv_cover: texture_2d<f32>;

struct Out { @location(0) color: vec4<f32>, @location(1) mat: vec4<f32>, };

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> Out {
    var o: Out;
    if (layer.mode.y == 1u) {
        o.color = vec4<f32>(layer.base_color.rgb, 1.0);
        o.mat = layer.base_values;
        return o;
    }
    let p = vec2<i32>(pos.xy);
    let pc = textureLoad(prev_color, p, 0);
    let pm = textureLoad(prev_mat, p, 0);
    let lc = textureLoad(l_color, p, 0);
    let lv = textureLoad(l_val, p, 0);
    let lk = textureLoad(l_cov, p, 0);
    var op = layer.opacity.x;
    if (layer.opacity.y > 0.5) { op = op * clamp(textureLoad(l_mask, p, 0).r, 0.0, 1.0); }
    if (layer.mode.z == 1u) {
        var pcb = pc.rgb;
        var pmb = pm.rgb;
        let block = i32(layer.style.y);
        if (block > 1) {
            let dims_p = vec2<i32>(textureDimensions(prev_color));
            let q = clamp((p / vec2<i32>(block)) * block + vec2<i32>(block / 2), vec2<i32>(0), dims_p - vec2<i32>(1));
            pcb = textureLoad(prev_color, q, 0).rgb;
            pmb = textureLoad(prev_mat, q, 0).rgb;
        }
        let radius = layer.opacity.z;
        let sharpen = layer.opacity.w;
        if (radius > 0.5 || sharpen > 0.0) {
            let r = max(radius, 1.0);
            let dims = vec2<i32>(textureDimensions(prev_color));
            var sc = vec3<f32>(0.0);
            var sm = vec3<f32>(0.0);
            var w = 0.0;
            for (var y = -3; y <= 3; y = y + 1) {
                for (var x = -3; x <= 3; x = x + 1) {
                    let off = vec2<f32>(f32(x), f32(y)) * (r / 3.0);
                    let q = clamp(p + vec2<i32>(round(off)), vec2<i32>(0), dims - vec2<i32>(1));
                    let k = exp(-dot(off, off) / (0.5 * r * r)) * textureLoad(uv_cover, q, 0).r;
                    sc = sc + textureLoad(prev_color, q, 0).rgb * k;
                    sm = sm + textureLoad(prev_mat, q, 0).rgb * k;
                    w = w + k;
                }
            }
            if (w > 1e-4) {
                let bc = sc / w;
                let bm = sm / w;
                if (radius > 0.5) {
                    pcb = bc;
                    pmb = bm;
                }
                if (sharpen > 0.0) {
                    pcb = max(pcb + (pc.rgb - bc) * sharpen, vec3<f32>(0.0));
                    pmb = clamp(pmb + (pm.rgb - bm) * sharpen, vec3<f32>(0.0), vec3<f32>(1.0));
                }
            }
        }
        var c = pow(max(pcb, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));
        c = clamp((c - layer.levels.x) / max(layer.levels.y - layer.levels.x, 1e-3), vec3<f32>(0.0), vec3<f32>(1.0));
        c = layer.levels.z + c * (layer.levels.w - layer.levels.z);
        if (layer.style.z > 0.5) { c = 1.0 - c; }
        if (layer.style.x > 1.5) { c = clamp(floor(c * layer.style.x) / (layer.style.x - 1.0), vec3<f32>(0.0), vec3<f32>(1.0)); }
        c = (c - 0.5) * (1.0 + layer.base_color.y) + 0.5 + layer.base_color.x;
        let l = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
        c = mix(vec3<f32>(l), c, layer.base_color.w);
        let k = vec3<f32>(0.57735);
        let a = layer.base_color.z;
        c = c * cos(a) + cross(k, c) * sin(a) + k * dot(k, c) * (1.0 - cos(a));
        c = pow(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(2.2 * layer.base_values.x));
        o.color = vec4<f32>(mix(pc.rgb, c, op), 1.0);
        let v = clamp(pmb + vec3<f32>(layer.base_values.y, layer.base_values.z, 0.0), vec3<f32>(0.0), vec3<f32>(1.0));
        o.mat = vec4<f32>(mix(pm.rgb, v, op), pm.a);
        return o;
    }
    var col = pc.rgb;
    if (lc.a > 1e-5) {
        let straight = lc.rgb / lc.a;
        col = mix(pc.rgb, clamp(blend_any(layer.mode.x, pc.rgb, straight, false), vec3<f32>(0.0), vec3<f32>(1.0)), lc.a * op);
    }
    var vals = pm.rgb;
    let straight_v = select(vec3<f32>(0.0), lv.rgb / max(lk.rgb, vec3<f32>(1e-5)), lk.rgb > vec3<f32>(1e-5));
    vals = mix(pm.rgb, clamp(blend_any(layer.mode.x, pm.rgb, straight_v, true), vec3<f32>(0.0), vec3<f32>(1.0)), lk.rgb * op);
    var extra = pm.a;
    if (lk.a > 1e-5) {
        extra = mix(pm.a, clamp(lv.a / lk.a, 0.0, 1.0), clamp(lk.a, 0.0, 1.0) * op);
    }
    o.color = vec4<f32>(col, 1.0);
    o.mat = vec4<f32>(vals, extra);
    return o;
}
"#;

pub const APPLY_MASK: &str = r#"
struct Paint {
    color: vec4<f32>,
    values: vec4<f32>,
    flags: vec4<f32>,
    mode: vec4<f32>,
};
@group(0) @binding(0) var<uniform> paint: Paint;
@group(0) @binding(1) var snap_mask: texture_2d<f32>;
@group(0) @binding(2) var stroke: texture_2d<f32>;

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(pos.xy);
    let old = textureLoad(snap_mask, p, 0).r;
    let a = clamp(textureLoad(stroke, p, 0).r, 0.0, 1.0) * paint.mode.y;
    let target_value = select(paint.color.x, 0.0, paint.mode.x > 0.5);
    let v = mix(old, target_value, a);
    return vec4<f32>(v, v, v, 1.0);
}
"#;

pub const GEOMETRY: &str = r#"
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
};
@vertex
fn vs(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>) -> VOut {
    var o: VOut;
    o.clip = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    o.world = p;
    o.normal = n;
    return o;
}
struct Out { @location(0) pos: vec4<f32>, @location(1) normal: vec4<f32>, };
@fragment
fn fs(i: VOut) -> Out {
    var o: Out;
    o.pos = vec4<f32>(i.world, 1.0);
    o.normal = vec4<f32>(normalize(i.normal), 1.0);
    return o;
}
"#;

pub const FILL: &str = r#"
struct Fill {
    color: vec4<f32>,
    values: vec4<f32>,
    flags: vec4<f32>,
    proj: vec4<f32>,
    proj2: vec4<f32>,
    mesh: vec4<f32>,
    generator: vec4<f32>,
    builder: vec4<f32>,
    direction: vec4<f32>,
    warp: array<vec4<f32>, 8>,
    extra: vec4<f32>,
};
@group(0) @binding(0) var<uniform> fill: Fill;

fn warp_point(i: i32, j: i32) -> vec2<f32> {
    let k = j * 4 + i;
    let v = fill.warp[k / 2];
    if (k % 2 == 0) { return v.xy; }
    return v.zw;
}

fn cross2(a: vec2<f32>, b: vec2<f32>) -> f32 {
    return a.x * b.y - a.y * b.x;
}

fn solve_u(h: vec2<f32>, e: vec2<f32>, f: vec2<f32>, g: vec2<f32>, v: f32) -> f32 {
    let den = e + g * v;
    if (abs(den.x) > abs(den.y)) { return (h.x - f.x * v) / den.x; }
    return (h.y - f.y * v) / den.y;
}

fn inv_bilinear(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>, c: vec2<f32>, d: vec2<f32>) -> vec2<f32> {
    let e = b - a;
    let f = d - a;
    let g = a - b + c - d;
    let h = p - a;
    let k2 = cross2(g, f);
    let k1 = cross2(e, f) + cross2(h, g);
    let k0 = cross2(h, e);
    if (abs(k2) < 1e-6) {
        let v = -k0 / select(k1, 1e-6, abs(k1) < 1e-9);
        return vec2<f32>(solve_u(h, e, f, g, v), v);
    }
    let w = k1 * k1 - 4.0 * k0 * k2;
    if (w < 0.0) { return vec2<f32>(-100.0); }
    let r = sqrt(w);
    let ik = 0.5 / k2;
    let v1 = (-k1 - r) * ik;
    let s1 = vec2<f32>(solve_u(h, e, f, g, v1), v1);
    let v2 = (-k1 + r) * ik;
    let s2 = vec2<f32>(solve_u(h, e, f, g, v2), v2);
    let d1 = length(s1 - clamp(s1, vec2<f32>(0.0), vec2<f32>(1.0)));
    let d2 = length(s2 - clamp(s2, vec2<f32>(0.0), vec2<f32>(1.0)));
    return select(s2, s1, d1 <= d2);
}

fn unwarp(q: vec2<f32>) -> vec2<f32> {
    var best = q;
    var best_err = 1e9;
    for (var j = 0; j < 3; j = j + 1) {
        for (var i = 0; i < 3; i = i + 1) {
            let st = inv_bilinear(q, warp_point(i, j), warp_point(i + 1, j), warp_point(i + 1, j + 1), warp_point(i, j + 1));
            let err = length(st - clamp(st, vec2<f32>(0.0), vec2<f32>(1.0)));
            if (err < best_err) {
                best_err = err;
                best = (vec2<f32>(f32(i), f32(j)) + st) / 3.0;
            }
        }
    }
    return best;
}

fn hash3(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(127.1, 311.7, 74.7))) * 43758.5453);
}

fn vnoise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(mix(hash3(i), hash3(i + vec3<f32>(1.0, 0.0, 0.0)), u.x), mix(hash3(i + vec3<f32>(0.0, 1.0, 0.0)), hash3(i + vec3<f32>(1.0, 1.0, 0.0)), u.x), u.y);
    let b = mix(mix(hash3(i + vec3<f32>(0.0, 0.0, 1.0)), hash3(i + vec3<f32>(1.0, 0.0, 1.0)), u.x), mix(hash3(i + vec3<f32>(0.0, 1.0, 1.0)), hash3(i + vec3<f32>(1.0, 1.0, 1.0)), u.x), u.y);
    return mix(a, b, u.z);
}

fn fbm3(p: vec3<f32>) -> f32 {
    var v = 0.0;
    var amp = 0.5;
    var q = p;
    for (var k = 0; k < 5; k = k + 1) {
        v = v + amp * vnoise3(q);
        q = q * 2.03 + vec3<f32>(17.0, 3.0, 11.0);
        amp = amp * 0.5;
    }
    return v / 0.96875;
}
@group(0) @binding(5) var bake_tex: texture_2d<f32>;
@group(0) @binding(1) var pos_map: texture_2d<f32>;
@group(0) @binding(2) var normal_map: texture_2d<f32>;
@group(0) @binding(3) var pattern_tex: texture_2d<f32>;
@group(0) @binding(4) var repeat_samp: sampler;

struct Out {
    @location(0) color: vec4<f32>,
    @location(1) val: vec4<f32>,
    @location(2) cov: vec4<f32>,
};

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x), mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn pattern(c0: vec2<f32>) -> vec4<f32> {
    let a = fill.proj.w;
    let rot = vec2<f32>(c0.x * cos(a) - c0.y * sin(a), c0.x * sin(a) + c0.y * cos(a));
    let c = rot * fill.proj.z + fill.proj2.xy;
    let kind = fill.proj.y;
    if (kind < 0.5) { return vec4<f32>(1.0); }
    if (kind < 1.5) {
        let k = (floor(c.x) + floor(c.y)) - 2.0 * floor((floor(c.x) + floor(c.y)) * 0.5);
        return vec4<f32>(vec3<f32>(k), 1.0);
    }
    if (kind < 2.5) { return vec4<f32>(vec3<f32>(step(0.5, fract(c.x))), 1.0); }
    if (kind < 3.5) {
        var v = 0.0;
        var amp = 0.5;
        var q = c * 4.0;
        for (var o = 0; o < 5; o = o + 1) {
            v = v + amp * vnoise(q);
            q = q * 2.03;
            amp = amp * 0.5;
        }
        return vec4<f32>(vec3<f32>(v), 1.0);
    }
    let image = textureSampleLevel(pattern_tex, repeat_samp, c, 0.0);
    return vec4<f32>(pow(image.rgb, vec3<f32>(2.2)), image.a);
}

fn axis_coords(p: vec3<f32>, axis: f32) -> vec2<f32> {
    if (axis < 0.5) { return p.zy; }
    if (axis < 1.5) { return p.xz; }
    return p.xy;
}

@fragment
fn fs(@builtin(position) frag: vec4<f32>) -> Out {
    let px = vec2<i32>(frag.xy);
    let size = vec2<f32>(textureDimensions(pos_map));
    let uv = frag.xy / size;
    let wp = textureLoad(pos_map, px, 0);
    let n = normalize(textureLoad(normal_map, px, 0).xyz + vec3<f32>(1e-6));
    let local = (wp.xyz - fill.mesh.xyz) / max(fill.mesh.w, 1e-6);
    let mode = fill.proj.x;
    var s = vec4<f32>(1.0);
    if (mode < 0.5) {
        s = pattern(uv);
    } else if (mode < 1.5) {
        var w = pow(abs(n), vec3<f32>(fill.proj2.z));
        w = w / max(w.x + w.y + w.z, 1e-6);
        s = pattern(local.zy * 0.5) * w.x + pattern(local.xz * 0.5) * w.y + pattern(local.xy * 0.5) * w.z;
    } else if (mode < 2.5) {
        s = pattern(axis_coords(local, fill.proj2.w) * vec2<f32>(0.5, -0.5) + 0.5);
    } else if (mode < 3.5) {
        let d = normalize(local + vec3<f32>(1e-6));
        s = pattern(vec2<f32>(atan2(d.z, d.x) / 6.2831853 + 0.5, acos(clamp(d.y, -1.0, 1.0)) / 3.1415927));
    } else if (mode < 4.5) {
        s = pattern(vec2<f32>(atan2(local.z, local.x) / 6.2831853 + 0.5, local.y * 0.5 + 0.5));
    } else if (mode < 5.5) {
        s = pattern(unwarp(axis_coords(local, fill.proj2.w) * vec2<f32>(0.5, -0.5) + 0.5));
    } else if (fill.proj.y > 3.5) {
        let image = textureSampleLevel(pattern_tex, repeat_samp, uv, 0.0);
        s = vec4<f32>(pow(image.rgb, vec3<f32>(2.2)), image.a);
    } else {
        s = pattern(uv);
    }
    var lum = dot(s.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) * s.a;
    if (fill.color.w > 0.5) { lum = s.a; }
    let gk = fill.generator.x;
    if (gk > 0.5) {
        let baked = textureLoad(bake_tex, px, 0);
        var v = 0.0;
        if (gk < 1.5) { v = 1.0 - baked.r; }
        else if (gk < 2.5) { v = clamp((baked.g - 0.5) * 2.0, 0.0, 1.0); }
        else if (gk < 3.5) { v = 1.0 - baked.b; }
        else if (gk < 4.5) { v = clamp(local.y * 0.5 + 0.5, 0.0, 1.0); }
        else if (gk < 5.5) { v = clamp(dot(n, fill.direction.xyz) * 0.5 + 0.5, 0.0, 1.0); }
        else if (gk < 6.5) { v = clamp(dot(local, fill.direction.xyz) * 0.5 + 0.5, 0.0, 1.0); }
        else {
            let noise = fbm3(local * fill.builder.w);
            let cavity = 1.0 - baked.r;
            let edge = clamp((baked.g - 0.5) * 2.0, 0.0, 1.0);
            if (gk < 7.5) { v = noise; }
            else if (gk < 8.5) {
                let w = fill.builder.x + fill.builder.y + fill.builder.z;
                v = (cavity * fill.builder.x + edge * fill.builder.y + noise * fill.builder.z) / max(w, 1e-4);
            }
            else if (gk < 9.5) { v = clamp(cavity * 1.4 - (1.0 - noise) * 0.45, 0.0, 1.0); }
            else { v = clamp(edge * 1.6 - (1.0 - noise) * 0.6, 0.0, 1.0); }
        }
        if (fill.direction.w > 0.0 && gk < 6.5) {
            v = clamp(v - (1.0 - fbm3(local * fill.builder.w)) * fill.direction.w, 0.0, 1.0);
        }
        let soft = max(fill.generator.z, 0.001);
        var g = smoothstep(fill.generator.y - soft, fill.generator.y + soft, v);
        if (fill.generator.w > 0.5) { g = 1.0 - g; }
        lum = lum * g;
    }
    let a = clamp(lum, 0.0, 1.0) * fill.values.w;
    var base = fill.color.rgb;
    if (fill.color.w > 0.5) { base = s.rgb; }
    var o: Out;
    let ac = a * fill.flags.x;
    o.color = vec4<f32>(base * ac, ac);
    let am = a * fill.flags.yzw;
    let ae = a * fill.extra.y;
    o.val = vec4<f32>(fill.values.xyz * am, fill.extra.x * ae);
    o.cov = vec4<f32>(am, ae);
    if (wp.w < 0.5) {
        o.color = vec4<f32>(0.0);
        o.val = vec4<f32>(0.0);
        o.cov = vec4<f32>(0.0);
    }
    return o;
}
"#;

pub const LAYER_EXPORT: &str = r#"
@group(0) @binding(0) var<uniform> sel: vec4<u32>;
@group(0) @binding(1) var l_color: texture_2d<f32>;
@group(0) @binding(2) var l_val: texture_2d<f32>;
@group(0) @binding(3) var l_cov: texture_2d<f32>;

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(pos.xy);
    let c = textureLoad(l_color, p, 0);
    let v = textureLoad(l_val, p, 0);
    let k = textureLoad(l_cov, p, 0);
    switch sel.x {
        case 0u: {
            let a = clamp(c.a, 0.0, 1.0);
            return vec4<f32>(pow(clamp(c.rgb / max(c.a, 1e-4), vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2)), a);
        }
        case 1u: { return vec4<f32>(vec3<f32>(clamp(v.r / max(k.r, 1e-4), 0.0, 1.0)), clamp(k.r, 0.0, 1.0)); }
        case 2u: { return vec4<f32>(vec3<f32>(clamp(v.g / max(k.g, 1e-4), 0.0, 1.0)), clamp(k.g, 0.0, 1.0)); }
        default: { return vec4<f32>(vec3<f32>(clamp(v.b / max(k.b, 1e-4), 0.0, 1.0)), clamp(k.b, 0.0, 1.0)); }
    }
}
"#;

pub const MERGE: &str = r#"
struct Merge {
    top: vec4<f32>,
    bottom: vec4<f32>,
    mode: vec4<u32>,
};
@group(0) @binding(0) var<uniform> merge: Merge;
@group(0) @binding(1) var top_color: texture_2d<f32>;
@group(0) @binding(2) var top_val: texture_2d<f32>;
@group(0) @binding(3) var top_cov: texture_2d<f32>;
@group(0) @binding(4) var top_mask: texture_2d<f32>;
@group(0) @binding(5) var bottom_color: texture_2d<f32>;
@group(0) @binding(6) var bottom_val: texture_2d<f32>;
@group(0) @binding(7) var bottom_cov: texture_2d<f32>;
@group(0) @binding(8) var bottom_mask: texture_2d<f32>;

struct Out { @location(0) color: vec4<f32>, @location(1) val: vec4<f32>, @location(2) cov: vec4<f32>, };

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> Out {
    let p = vec2<i32>(pos.xy);
    var kt = merge.top.x;
    if (merge.top.y > 0.5) { kt = kt * clamp(textureLoad(top_mask, p, 0).r, 0.0, 1.0); }
    var kb = merge.bottom.x;
    if (merge.bottom.y > 0.5) { kb = kb * clamp(textureLoad(bottom_mask, p, 0).r, 0.0, 1.0); }
    let tc = textureLoad(top_color, p, 0);
    let tv = textureLoad(top_val, p, 0);
    let tk = textureLoad(top_cov, p, 0);
    let bc = textureLoad(bottom_color, p, 0);
    let bv = textureLoad(bottom_val, p, 0);
    let bk = textureLoad(bottom_cov, p, 0);
    let at = clamp(tc.a, 0.0, 1.0) * kt;
    let ab = clamp(bc.a, 0.0, 1.0) * kb;
    let top_rgb = tc.rgb / max(tc.a, 1e-4);
    let bottom_rgb = bc.rgb / max(bc.a, 1e-4);
    let blended = mix(top_rgb, blend_any(merge.mode.x, bottom_rgb, top_rgb, false), select(0.0, 1.0, ab > 0.0));
    let color_a = at + ab * (1.0 - at);
    let color_pm = at * (ab * blended + (1.0 - ab) * top_rgb) + (1.0 - at) * ab * bottom_rgb;
    let vt = clamp(tk.rgb, vec3<f32>(0.0), vec3<f32>(1.0)) * kt;
    let vb = clamp(bk.rgb, vec3<f32>(0.0), vec3<f32>(1.0)) * kb;
    let top_v = tv.rgb / max(tk.rgb, vec3<f32>(1e-4));
    let bottom_v = bv.rgb / max(bk.rgb, vec3<f32>(1e-4));
    var o: Out;
    o.color = vec4<f32>(color_pm, color_a);
    let et = clamp(tk.a, 0.0, 1.0) * kt;
    let eb = clamp(bk.a, 0.0, 1.0) * kb;
    let extra = et * (tv.a / max(tk.a, 1e-4)) + (1.0 - et) * eb * (bv.a / max(bk.a, 1e-4));
    o.val = vec4<f32>(vt * top_v + (1.0 - vt) * vb * bottom_v, extra);
    o.cov = vec4<f32>(vt + vb * (1.0 - vt), et + eb * (1.0 - et));
    return o;
}
"#;

pub const PAIR_TO_SET: &str = r#"
@group(0) @binding(0) var final_color: texture_2d<f32>;
@group(0) @binding(1) var final_mat: texture_2d<f32>;

struct Out { @location(0) color: vec4<f32>, @location(1) val: vec4<f32>, @location(2) cov: vec4<f32>, };

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> Out {
    let p = vec2<i32>(pos.xy);
    var o: Out;
    o.color = vec4<f32>(textureLoad(final_color, p, 0).rgb, 1.0);
    o.val = textureLoad(final_mat, p, 0);
    o.cov = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    return o;
}
"#;

pub const MASK_FX: &str = r#"
struct Fx { p: vec4<f32>, q: vec4<f32>, };
@group(0) @binding(0) var<uniform> fx: Fx;
@group(0) @binding(1) var acc_tex: texture_2d<f32>;
@group(0) @binding(2) var src_tex: texture_2d<f32>;

fn mix_mode(a: f32, v: f32, mode: f32) -> f32 {
    if (mode < 0.5) { return v; }
    if (mode < 1.5) { return a * v; }
    if (mode < 2.5) { return min(a + v, 1.0); }
    if (mode < 3.5) { return max(a - v, 0.0); }
    if (mode < 4.5) { return max(a, v); }
    if (mode < 5.5) { return min(a, v); }
    return 1.0 - (1.0 - a) * (1.0 - v);
}

@fragment
fn fs(i: FOut) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(acc_tex));
    let px = vec2<i32>(i.clip.xy);
    let a = clamp(textureLoad(acc_tex, px, 0).r, 0.0, 1.0);
    let kind = fx.p.x;
    var r = a;
    if (kind < 0.5) {
        r = mix_mode(a, fx.p.w, fx.p.y);
    } else if (kind < 1.5) {
        r = mix_mode(a, clamp(textureLoad(src_tex, px, 0).r, 0.0, 1.0), fx.p.y);
    } else if (kind < 2.5) {
        let t = clamp((a - fx.p.w) / max(fx.q.x - fx.p.w, 0.001), 0.0, 1.0);
        r = mix(fx.q.y, fx.q.z, t);
    } else if (kind < 3.5) {
        r = 1.0 - a;
    } else if (kind < 4.5) {
        let radius = i32(round(fx.p.w));
        var dir = vec2<i32>(1, 0);
        if (fx.q.x > 0.5) { dir = vec2<i32>(0, 1); }
        var sum = 0.0;
        var n = 0.0;
        for (var k = -radius; k <= radius; k = k + 1) {
            let q = clamp(px + dir * k, vec2<i32>(0), size - vec2<i32>(1));
            sum = sum + textureLoad(acc_tex, q, 0).r;
            n = n + 1.0;
        }
        r = sum / max(n, 1.0);
    } else {
        r = mix_mode(a, clamp(textureLoad(src_tex, px, 0).r, 0.0, 1.0), fx.p.y);
    }
    let o = mix(a, r, clamp(fx.p.z, 0.0, 1.0));
    return vec4<f32>(o, o, o, 1.0);
}
"#;

pub const RESAMPLE: &str = r#"
@group(0) @binding(0) var src_color: texture_2d<f32>;
@group(0) @binding(1) var src_val: texture_2d<f32>;
@group(0) @binding(2) var src_cov: texture_2d<f32>;
@group(0) @binding(3) var src_mask: texture_2d<f32>;
@group(0) @binding(4) var samp: sampler;
struct Four {
    @location(0) color: vec4<f32>,
    @location(1) val: vec4<f32>,
    @location(2) cov: vec4<f32>,
    @location(3) mask: vec4<f32>,
};

@fragment
fn fs(i: FOut) -> Four {
    var o: Four;
    o.color = textureSampleLevel(src_color, samp, i.uv, 0.0);
    o.val = textureSampleLevel(src_val, samp, i.uv, 0.0);
    o.cov = textureSampleLevel(src_cov, samp, i.uv, 0.0);
    o.mask = textureSampleLevel(src_mask, samp, i.uv, 0.0);
    return o;
}
"#;

pub const RESAMPLE_ONE: &str = r#"
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

@fragment
fn fs(i: FOut) -> @location(0) vec4<f32> {
    return textureSampleLevel(src, samp, i.uv, 0.0);
}
"#;

pub const MASK_IMAGE: &str = r#"
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

@fragment
fn fs(i: FOut) -> @location(0) vec4<f32> {
    let c = textureSampleLevel(image, samp, i.uv, 0.0);
    let l = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) * c.a;
    return vec4<f32>(l, l, l, 1.0);
}
"#;

pub const MASK_ID: &str = r#"
struct Pick { color: vec4<f32>, };
@group(0) @binding(0) var<uniform> pick: Pick;
@group(0) @binding(1) var id_tex: texture_2d<f32>;

@fragment
fn fs(i: FOut) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(id_tex));
    let p = clamp(vec2<i32>(i.uv * vec2<f32>(dims)), vec2<i32>(0), dims - vec2<i32>(1));
    let c = textureLoad(id_tex, p, 0).rgb;
    let m = select(0.0, 1.0, distance(c, pick.color.rgb) <= pick.color.w);
    return vec4<f32>(m, m, m, 1.0);
}
"#;

pub const WHITE: &str = r#"
@fragment
fn fs() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0);
}
"#;

pub const MASK_MUL: &str = r#"
@group(0) @binding(0) var source_mask: texture_2d<f32>;

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let v = textureLoad(source_mask, vec2<i32>(pos.xy), 0).r;
    return vec4<f32>(v, v, v, 1.0);
}
"#;

pub const DILATE: &str = r#"
@group(0) @binding(0) var src_color: texture_2d<f32>;
@group(0) @binding(1) var src_mat: texture_2d<f32>;
@group(0) @binding(2) var coverage: texture_2d<f32>;

struct Out { @location(0) color: vec4<f32>, @location(1) mat: vec4<f32>, };

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> Out {
    let p = vec2<i32>(pos.xy);
    let size = vec2<i32>(textureDimensions(coverage));
    var o: Out;
    o.color = textureLoad(src_color, p, 0);
    o.mat = textureLoad(src_mat, p, 0);
    if (textureLoad(coverage, p, 0).r > 0.5) { return o; }
    let dirs = array<vec2<i32>, 8>(vec2<i32>(1, 0), vec2<i32>(-1, 0), vec2<i32>(0, 1), vec2<i32>(0, -1), vec2<i32>(1, 1), vec2<i32>(-1, 1), vec2<i32>(1, -1), vec2<i32>(-1, -1));
    for (var r = 1; r <= 16; r = r + 1) {
        for (var k = 0; k < 8; k = k + 1) {
            let q = clamp(p + dirs[k] * r, vec2<i32>(0), size - 1);
            if (textureLoad(coverage, q, 0).r > 0.5) {
                o.color = textureLoad(src_color, q, 0);
                o.mat = textureLoad(src_mat, q, 0);
                return o;
            }
        }
    }
    return o;
}
"#;

pub const COVERAGE: &str = r#"
@vertex
fn vs(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
}
@fragment
fn fs() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#;

pub const PATH_COVER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    cursor: vec4<f32>,
    cursor_normal: vec4<f32>,
    params: vec4<f32>,
};
struct Cover { bounds: vec4<f32>, params: vec4<f32>, };
@group(0) @binding(0) var<uniform> cam: Camera;
@group(0) @binding(1) var<uniform> cover: Cover;
@group(0) @binding(2) var depth_tex: texture_depth_2d;
@group(0) @binding(3) var cover_tex: texture_2d<f32>;
@group(0) @binding(4) var clamp_samp: sampler;

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>) -> VOut {
    var o: VOut;
    o.clip = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    o.world = p;
    o.normal = n;
    o.uv = uv;
    return o;
}

@fragment
fn fs(i: VOut) -> @location(0) vec4<f32> {
    var at = i.uv;
    if (cover.params.x < 0.5) {
        let clip = cam.view_proj * vec4<f32>(i.world, 1.0);
        if (clip.w <= 0.0) { return vec4<f32>(0.0); }
        let ndc = clip.xyz / clip.w;
        let size = vec2<f32>(textureDimensions(depth_tex));
        at = vec2<f32>((ndc.x * 0.5 + 0.5) * size.x, (0.5 - ndc.y * 0.5) * size.y);
        let px = vec2<i32>(at);
        if (px.x < 0 || px.y < 0 || px.x >= i32(size.x) || px.y >= i32(size.y)) { return vec4<f32>(0.0); }
        if (ndc.z > textureLoad(depth_tex, px, 0) + 0.0015) { return vec4<f32>(0.0); }
        let facing = dot(normalize(i.normal), normalize(cam.eye.xyz - i.world)) > 0.0;
        if (cover.params.y > 0.5 && !facing) { return vec4<f32>(0.0); }
    }
    let t = (at - cover.bounds.xy) / max(cover.bounds.zw - cover.bounds.xy, vec2<f32>(1e-6));
    if (any(t < vec2<f32>(0.0)) || any(t > vec2<f32>(1.0))) { return vec4<f32>(0.0); }
    let c = textureSampleLevel(cover_tex, clamp_samp, t, 0.0).a;
    return vec4<f32>(c, 0.0, 0.0, 1.0);
}
"#;

pub const BAKE_DEPTH: &str = r#"
struct Light { view_proj: mat4x4<f32>, dir: vec4<f32>, params: vec4<f32>, };
@group(0) @binding(0) var<uniform> light: Light;
@vertex
fn vs(@location(0) p: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>) -> @builtin(position) vec4<f32> {
    return light.view_proj * vec4<f32>(p, 1.0);
}
"#;

pub const BAKE_ACCUM: &str = r#"
struct Light { view_proj: mat4x4<f32>, dir: vec4<f32>, params: vec4<f32>, };
@group(0) @binding(0) var<uniform> light: Light;
@group(0) @binding(1) var pos_map: texture_2d<f32>;
@group(0) @binding(2) var normal_map: texture_2d<f32>;
@group(0) @binding(3) var depth_map: texture_depth_2d;

@fragment
fn fs(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let px = vec2<i32>(frag.xy);
    let wp = textureLoad(pos_map, px, 0);
    if (wp.w < 0.5) { return vec4<f32>(0.0); }
    let n = normalize(textureLoad(normal_map, px, 0).xyz);
    let d = light.dir.xyz;
    let c = light.view_proj * vec4<f32>(wp.xyz, 1.0);
    let ndc = c.xyz / c.w;
    let size = vec2<f32>(textureDimensions(depth_map));
    let tp = vec2<i32>(i32((ndc.x * 0.5 + 0.5) * size.x), i32((0.5 - ndc.y * 0.5) * size.y));
    let stored = textureLoad(depth_map, clamp(tp, vec2<i32>(0), vec2<i32>(size) - 1), 0);
    let facing = dot(n, d);
    var o = vec4<f32>(0.0);
    if (facing > 0.0) {
        let visible = select(0.0, 1.0, ndc.z <= stored + light.params.x);
        o.x = visible * facing;
        o.y = facing;
    } else {
        let depth_range = light.params.y;
        let thickness = max(ndc.z - stored, 0.0) * depth_range;
        o.z = thickness * -facing;
        o.w = -facing;
    }
    return o;
}
"#;

pub const BAKE_FINAL: &str = r#"
struct Light { view_proj: mat4x4<f32>, dir: vec4<f32>, params: vec4<f32>, };
@group(0) @binding(0) var<uniform> light: Light;
@group(0) @binding(1) var accum: texture_2d<f32>;
@group(0) @binding(2) var pos_map: texture_2d<f32>;
@group(0) @binding(3) var normal_map: texture_2d<f32>;

fn sample_ok(p: vec2<i32>, size: vec2<i32>) -> bool {
    if (any(p < vec2<i32>(0)) || any(p >= size)) { return false; }
    return textureLoad(pos_map, p, 0).w > 0.5;
}

@fragment
fn fs(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let px = vec2<i32>(frag.xy);
    let size = vec2<i32>(textureDimensions(pos_map));
    let a = textureLoad(accum, px, 0);
    let wp = textureLoad(pos_map, px, 0);
    if (wp.w < 0.5) { return vec4<f32>(1.0, 0.5, 0.0, 0.0); }
    let ao = select(1.0, a.x / a.y, a.y > 1e-5);
    let thickness = clamp(select(0.0, a.z / a.w, a.w > 1e-5) / max(light.params.z, 1e-6), 0.0, 1.0);
    let n = normalize(textureLoad(normal_map, px, 0).xyz);
    var curv = 0.0;
    var count = 0.0;
    let offsets = array<vec2<i32>, 4>(vec2<i32>(2, 0), vec2<i32>(-2, 0), vec2<i32>(0, 2), vec2<i32>(0, -2));
    for (var k = 0; k < 4; k = k + 1) {
        let q = px + offsets[k];
        if (!sample_ok(q, size)) { continue; }
        let dp = textureLoad(pos_map, q, 0).xyz - wp.xyz;
        let dn = normalize(textureLoad(normal_map, q, 0).xyz) - n;
        let l2 = dot(dp, dp);
        if (l2 < 1e-12) { continue; }
        curv = curv + dot(dn, dp) / l2;
        count = count + 1.0;
    }
    if (count > 0.0) { curv = curv / count; }
    let c = clamp(0.5 + curv * light.params.w, 0.0, 1.0);
    return vec4<f32>(ao, c, thickness, 1.0);
}
"#;

pub const EXPORT: &str = r#"
struct Sel { channel: vec4<u32>, params: vec4<f32>, };
@group(0) @binding(0) var<uniform> sel: Sel;
@group(0) @binding(1) var color_tex: texture_2d<f32>;
@group(0) @binding(2) var mat_tex: texture_2d<f32>;
@group(0) @binding(3) var bake_tex: texture_2d<f32>;
@group(0) @binding(4) var pos_map: texture_2d<f32>;
@group(0) @binding(5) var normal_map: texture_2d<f32>;
@group(0) @binding(6) var baked_normal: texture_2d<f32>;
@group(0) @binding(7) var baked_id: texture_2d<f32>;
@group(0) @binding(8) var lin: sampler;

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(pos.xy);
    let size = vec2<i32>(textureDimensions(mat_tex));
    let m = textureLoad(mat_tex, p, 0);
    let b = textureLoad(bake_tex, p, 0);
    let uv = (vec2<f32>(p) + 0.5) / vec2<f32>(size);
    let bn = textureSampleLevel(baked_normal, lin, uv, 0.0).xyz * 2.0 - 1.0;
    switch sel.channel.x {
        case 10u: { return vec4<f32>(textureSampleLevel(baked_id, lin, uv, 0.0).rgb, 1.0); }
        case 11u: { return vec4<f32>(normalize(vec3<f32>(bn.x, -bn.y, max(bn.z, 0.05))) * 0.5 + 0.5, 1.0); }
        case 5u: { return vec4<f32>(vec3<f32>(b.r), 1.0); }
        case 6u: { return vec4<f32>(vec3<f32>(b.g), 1.0); }
        case 7u: { return vec4<f32>(vec3<f32>(b.b), 1.0); }
        case 8u: { return vec4<f32>(clamp((textureLoad(pos_map, p, 0).xyz - sel.params.yzw) / max(bitcast<f32>(sel.channel.y) * 2.0, 1e-6) + 0.5, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0); }
        case 9u: { return vec4<f32>(normalize(textureLoad(normal_map, p, 0).xyz + vec3<f32>(1e-6)) * 0.5 + 0.5, 1.0); }
        case 0u: { return vec4<f32>(pow(clamp(textureLoad(color_tex, p, 0).rgb, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2)), 1.0); }
        case 1u: { return vec4<f32>(vec3<f32>(m.r), 1.0); }
        case 2u: { return vec4<f32>(vec3<f32>(m.g), 1.0); }
        case 3u: { return vec4<f32>(vec3<f32>(m.b), 1.0); }
        case 12u: { return vec4<f32>(vec3<f32>(clamp(m.a, 0.0, 1.0)), 1.0); }
        default: {
            let hx = textureLoad(mat_tex, clamp(p + vec2<i32>(1, 0), vec2<i32>(0), size - 1), 0).b - textureLoad(mat_tex, clamp(p - vec2<i32>(1, 0), vec2<i32>(0), size - 1), 0).b;
            let hy = textureLoad(mat_tex, clamp(p + vec2<i32>(0, 1), vec2<i32>(0), size - 1), 0).b - textureLoad(mat_tex, clamp(p - vec2<i32>(0, 1), vec2<i32>(0), size - 1), 0).b;
            let s = sel.params.x * f32(size.x) / 256.0;
            let n = normalize(vec3<f32>(bn.x - hx * s, -bn.y + hy * s, max(bn.z, 0.05)));
            return vec4<f32>(n * 0.5 + 0.5, 1.0);
        }
    }
}
"#;
