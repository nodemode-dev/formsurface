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
    let nt = normalize(vec3<f32>(-hx * strength, hy * strength, 1.0));
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
        color = color + (diff + spec) * nl * power[k];
    }
    let r = reflect(-v, n);
    let fe = f0 + (max(vec3<f32>(1.0 - rough), f0) - f0) * pow(1.0 - max(dot(n, v), 0.0), 5.0);
    let env_spec = mix(sky(r), sky(n) * 0.6, rough * rough);
    color = color + (1.0 - metal) * base * sky(n) * 0.35 + env_spec * fe * 0.9;
    color = color * cam.params.x;
    color = (color * (2.51 * color + 0.03)) / (color * (2.43 * color + 0.59) + 0.14);
    if (cam.cursor.w > 0.0) {
        let d = distance(i.world, cam.cursor.xyz) / cam.cursor.w;
        let ring = smoothstep(0.90, 0.95, d) * (1.0 - smoothstep(0.98, 1.03, d));
        color = mix(color, vec3<f32>(1.0), ring * 0.8 * step(0.0, dot(normalize(i.normal), cam.cursor_normal.xyz) + 0.3));
    }
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
};
struct Stamps {
    count: vec4<u32>,
    brush: vec4<f32>,
    items: array<Stamp, 32>,
};
@group(0) @binding(0) var<uniform> cam: Camera;
@group(0) @binding(1) var<uniform> stamps: Stamps;
@group(0) @binding(2) var depth_tex: texture_depth_2d;

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

@fragment
fn fs(i: VOut) -> @location(0) vec4<f32> {
    let n = normalize(i.normal);
    let hardness = stamps.brush.x;
    let flow = stamps.brush.y;
    let cull = stamps.brush.z;
    let clip = cam.view_proj * vec4<f32>(i.world, 1.0);
    let ndc = clip.xyz / clip.w;
    let size = vec2<f32>(textureDimensions(depth_tex));
    let px = vec2<i32>(i32((ndc.x * 0.5 + 0.5) * size.x), i32((0.5 - ndc.y * 0.5) * size.y));
    var visible = false;
    if (px.x >= 0 && px.y >= 0 && px.x < i32(size.x) && px.y < i32(size.y) && clip.w > 0.0) {
        let stored = textureLoad(depth_tex, px, 0);
        visible = ndc.z <= stored + 0.0015;
    }
    let facing = dot(n, normalize(cam.eye.xyz - i.world)) > 0.0;
    var keep = 1.0;
    for (var k = 0u; k < stamps.count.x; k = k + 1u) {
        let s = stamps.items[k];
        let mirrored = s.normal.w > 0.5;
        if (!mirrored && !visible) { continue; }
        if (!mirrored && cull > 0.5 && !facing) { continue; }
        if (dot(n, s.normal.xyz) < 0.15) { continue; }
        let d = distance(i.world, s.center.xyz) / s.center.w;
        if (d >= 1.0) { continue; }
        var c = 1.0;
        if (d > hardness && hardness < 1.0) {
            let t = (1.0 - d) / (1.0 - hardness);
            c = t * t * (3.0 - 2.0 * t);
        }
        keep = keep * (1.0 - c * flow * stamps.brush.w);
    }
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
};
@group(0) @binding(0) var<uniform> paint: Paint;
@group(0) @binding(1) var snap_color: texture_2d<f32>;
@group(0) @binding(2) var snap_val: texture_2d<f32>;
@group(0) @binding(3) var snap_cov: texture_2d<f32>;
@group(0) @binding(4) var mask: texture_2d<f32>;

struct Out {
    @location(0) color: vec4<f32>,
    @location(1) val: vec4<f32>,
    @location(2) cov: vec4<f32>,
};

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> Out {
    let p = vec2<i32>(pos.xy);
    let sc = textureLoad(snap_color, p, 0);
    let sv = textureLoad(snap_val, p, 0);
    let sk = textureLoad(snap_cov, p, 0);
    let a = clamp(textureLoad(mask, p, 0).r, 0.0, 1.0) * paint.mode.y;
    var o: Out;
    if (paint.mode.x > 0.5) {
        let k = 1.0 - a;
        o.color = vec4<f32>(sc.rgb * select(1.0, k, paint.flags.x > 0.5), sc.a * select(1.0, k, paint.flags.x > 0.5));
        let ks = vec3<f32>(select(1.0, k, paint.flags.y > 0.5), select(1.0, k, paint.flags.z > 0.5), select(1.0, k, paint.flags.w > 0.5));
        o.val = vec4<f32>(sv.rgb * ks, 0.0);
        o.cov = vec4<f32>(sk.rgb * ks, 0.0);
        return o;
    }
    let ac = a * paint.flags.x;
    o.color = vec4<f32>(paint.color.rgb * ac + sc.rgb * (1.0 - ac), ac + sc.a * (1.0 - ac));
    let am = a * paint.flags.yzw;
    o.val = vec4<f32>(paint.values.rgb * am + sv.rgb * (1.0 - am), 0.0);
    o.cov = vec4<f32>(am + sk.rgb * (1.0 - am), 0.0);
    return o;
}
"#;

pub const COMPOSITE: &str = r#"
struct Layer {
    opacity: vec4<f32>,
    mode: vec4<u32>,
    base_color: vec4<f32>,
    base_values: vec4<f32>,
};
@group(0) @binding(0) var<uniform> layer: Layer;
@group(0) @binding(1) var prev_color: texture_2d<f32>;
@group(0) @binding(2) var prev_mat: texture_2d<f32>;
@group(0) @binding(3) var l_color: texture_2d<f32>;
@group(0) @binding(4) var l_val: texture_2d<f32>;
@group(0) @binding(5) var l_cov: texture_2d<f32>;

struct Out { @location(0) color: vec4<f32>, @location(1) mat: vec4<f32>, };

fn blend3(m: u32, a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    switch m {
        case 1u: { return a * b; }
        case 2u: { return 1.0 - (1.0 - a) * (1.0 - b); }
        case 3u: { return select(1.0 - 2.0 * (1.0 - a) * (1.0 - b), 2.0 * a * b, a < vec3<f32>(0.5)); }
        case 4u: { return a + b; }
        case 5u: { return a - b; }
        case 6u: { return min(a, b); }
        case 7u: { return max(a, b); }
        default: { return b; }
    }
}

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> Out {
    var o: Out;
    if (layer.mode.y == 1u) {
        o.color = vec4<f32>(layer.base_color.rgb, 1.0);
        o.mat = vec4<f32>(layer.base_values.rgb, 1.0);
        return o;
    }
    let p = vec2<i32>(pos.xy);
    let pc = textureLoad(prev_color, p, 0);
    let pm = textureLoad(prev_mat, p, 0);
    let lc = textureLoad(l_color, p, 0);
    let lv = textureLoad(l_val, p, 0);
    let lk = textureLoad(l_cov, p, 0);
    let op = layer.opacity.x;
    var col = pc.rgb;
    if (lc.a > 1e-5) {
        let straight = lc.rgb / lc.a;
        col = mix(pc.rgb, clamp(blend3(layer.mode.x, pc.rgb, straight), vec3<f32>(0.0), vec3<f32>(1.0)), lc.a * op);
    }
    var vals = pm.rgb;
    let straight_v = select(vec3<f32>(0.0), lv.rgb / max(lk.rgb, vec3<f32>(1e-5)), lk.rgb > vec3<f32>(1e-5));
    vals = mix(pm.rgb, clamp(blend3(layer.mode.x, pm.rgb, straight_v), vec3<f32>(0.0), vec3<f32>(1.0)), lk.rgb * op);
    o.color = vec4<f32>(col, 1.0);
    o.mat = vec4<f32>(vals, 1.0);
    return o;
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

pub const EXPORT: &str = r#"
struct Sel { channel: vec4<u32>, params: vec4<f32>, };
@group(0) @binding(0) var<uniform> sel: Sel;
@group(0) @binding(1) var color_tex: texture_2d<f32>;
@group(0) @binding(2) var mat_tex: texture_2d<f32>;

@fragment
fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(pos.xy);
    let size = vec2<i32>(textureDimensions(mat_tex));
    let m = textureLoad(mat_tex, p, 0);
    switch sel.channel.x {
        case 0u: { return vec4<f32>(pow(clamp(textureLoad(color_tex, p, 0).rgb, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2)), 1.0); }
        case 1u: { return vec4<f32>(vec3<f32>(m.r), 1.0); }
        case 2u: { return vec4<f32>(vec3<f32>(m.g), 1.0); }
        case 3u: { return vec4<f32>(vec3<f32>(m.b), 1.0); }
        default: {
            let hx = textureLoad(mat_tex, clamp(p + vec2<i32>(1, 0), vec2<i32>(0), size - 1), 0).b - textureLoad(mat_tex, clamp(p - vec2<i32>(1, 0), vec2<i32>(0), size - 1), 0).b;
            let hy = textureLoad(mat_tex, clamp(p + vec2<i32>(0, 1), vec2<i32>(0), size - 1), 0).b - textureLoad(mat_tex, clamp(p - vec2<i32>(0, 1), vec2<i32>(0), size - 1), 0).b;
            let s = sel.params.x * f32(size.x) / 256.0;
            let n = normalize(vec3<f32>(-hx * s, hy * s, 1.0));
            return vec4<f32>(n * 0.5 + 0.5, 1.0);
        }
    }
}
"#;
