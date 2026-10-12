use crate::shaders;
use fs_core::{dot, BrushSettings, ChannelValues, FilterSettings, LayerInfo, LayerKind, Mesh, StrokeMode, Vec3};
use wasm_bindgen::prelude::*;

const LAYER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const UNDO_LIMIT: usize = 24;

struct Set {
    color: wgpu::Texture,
    val: wgpu::Texture,
    cov: wgpu::Texture,
    mask: wgpu::Texture,
}

struct LayerTex {
    id: u32,
    set: Set,
    effects: Vec<Effect>,
    fx: Option<[wgpu::Texture; 2]>,
}

#[derive(Clone)]
pub struct Effect {
    pub params: [f32; 8],
    pub fill: Option<[f32; 72]>,
    pub source: u32,
    pub spec: fs_core::MaskEffect,
}

struct Snapshot {
    layer: u32,
    set: Set,
}

struct Pair {
    color: wgpu::Texture,
    mat: wgpu::Texture,
}

struct PathScene {
    nodes: wgpu::Buffer,
    tris: wgpu::Buffer,
    attrs: wgpu::Buffer,
    roles: wgpu::Buffer,
    arrays: Option<[wgpu::Texture; 3]>,
    accum: Option<[wgpu::Texture; 6]>,
    filtered: Option<[wgpu::Texture; 2]>,
    cur: usize,
    samples: u32,
    seed: u32,
}

struct SetState {
    size: u32,
    extra_role: f32,
    base: [f32; 5],
    range: (u32, u32),
    layers: Vec<LayerTex>,
    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
    final_set: Pair,
    coverage: wgpu::Texture,
    geometry: Option<(wgpu::Texture, wgpu::Texture)>,
    bake: wgpu::Texture,
    baked: bool,
    baked_normal: wgpu::Texture,
    baked_id: wgpu::Texture,
    high_baked: bool,
    quick: wgpu::Texture,
    mask: wgpu::Texture,
    stroke: Option<(usize, Snapshot)>,
}

pub struct Gpu {
    pub backend: String,
    pub size: u32,
    default_size: u32,
    resample_pipe: wgpu::RenderPipeline,
    resample_one_pipe: wgpu::RenderPipeline,
    pub base: [f32; 5],
    pub extra_role: f32,
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    depth: wgpu::Texture,
    sampler: wgpu::Sampler,
    camera: wgpu::Buffer,
    stamp_buffer: wgpu::Buffer,
    fill_uniform: wgpu::Buffer,
    bake: wgpu::Texture,
    pub baked: bool,
    baked_normal: wgpu::Texture,
    baked_id: wgpu::Texture,
    pub high_baked: bool,
    light_uniform: wgpu::Buffer,
    bake_depth_pipe: wgpu::RenderPipeline,
    bake_accum_pipe: wgpu::RenderPipeline,
    bake_final_pipe: wgpu::RenderPipeline,
    fill_pipe: wgpu::RenderPipeline,
    apply_mask_pipe: wgpu::RenderPipeline,
    mask_mul_pipe: wgpu::RenderPipeline,
    invert_pipe: wgpu::RenderPipeline,
    mask_image_pipe: wgpu::RenderPipeline,
    mask_id_pipe: wgpu::RenderPipeline,
    flatten_pipe: wgpu::RenderPipeline,
    merge_pipe: wgpu::RenderPipeline,
    merge_uniform: wgpu::Buffer,
    layer_export_pipe: wgpu::RenderPipeline,
    mask_export_pipe: wgpu::RenderPipeline,
    group_mask: wgpu::Texture,
    quick: wgpu::Texture,
    pub quick_mode: u32,
    pub clone_offset: Option<[i32; 2]>,
    pub clone_layer: Option<usize>,
    pub flat: bool,
    pub flat_channel: u32,
    geometry_pipe: wgpu::RenderPipeline,
    geometry: Option<(wgpu::Texture, wgpu::Texture)>,
    patterns: std::collections::HashMap<u32, wgpu::Texture>,
    tile_patterns: std::collections::HashMap<(usize, u32), wgpu::Texture>,
    white: wgpu::Texture,
    clamp_sampler: wgpu::Sampler,
    alpha_tex: wgpu::Texture,
    stencil_tex: wgpu::Texture,
    paint_uniform: wgpu::Buffer,
    layer_uniform: wgpu::Buffer,
    export_uniform: wgpu::Buffer,
    view_pipe: wgpu::RenderPipeline,
    background_pipe: wgpu::RenderPipeline,
    env_uniform: wgpu::Buffer,
    env_textures: [wgpu::Texture; 3],
    pub env_params: [f32; 20],
    post_pipe: wgpu::RenderPipeline,
    bloom_pipe: wgpu::RenderPipeline,
    reproject_pipe: wgpu::RenderPipeline,
    trace_pipe: wgpu::RenderPipeline,
    trace_job: Option<(wgpu::BindGroup, u32)>,
    path_pipe: wgpu::RenderPipeline,
    path_show_pipe: wgpu::RenderPipeline,
    path_gather_pipe: wgpu::RenderPipeline,
    path_uniform: wgpu::Buffer,
    path_show_uniform: wgpu::Buffer,
    path: Option<PathScene>,
    pub path_view: bool,
    pub path_denoise: bool,
    pub path_lens: [f32; 2],
    pub path_scatter: f32,
    pub path_size: Option<(u32, u32)>,
    path_filter_pipe: wgpu::RenderPipeline,
    path_filter_uniforms: [wgpu::Buffer; 4],
    env_cdf: wgpu::Buffer,
    path_inflight: std::sync::Arc<std::sync::atomic::AtomicU32>,
    post_uniform: wgpu::Buffer,
    pub post_params: [f32; 24],
    scene: wgpu::Texture,
    bloom: wgpu::Texture,
    paint_pipe: wgpu::RenderPipeline,
    apply_pipe: wgpu::RenderPipeline,
    composite_pipe: wgpu::RenderPipeline,
    dilate_pipe: wgpu::RenderPipeline,
    coverage_pipe: wgpu::RenderPipeline,
    export_pipe: wgpu::RenderPipeline,
    export16_pipe: wgpu::RenderPipeline,
    region_pipe: wgpu::RenderPipeline,
    fx_pipe: wgpu::RenderPipeline,
    fx_uniform: wgpu::Buffer,
    cover_pipe: wgpu::RenderPipeline,
    vertices: Option<wgpu::Buffer>,
    indices: Option<wgpu::Buffer>,
    range: (u32, u32),
    total: u32,
    parked: Vec<Option<SetState>>,
    current: usize,
    coverage: wgpu::Texture,
    mask: wgpu::Texture,
    ping: Pair,
    pong: Pair,
    final_set: Pair,
    scratch: Set,
    layers: Vec<LayerTex>,
    stroke: Option<(usize, Snapshot)>,
    undo_stack: Vec<Snapshot>,
    redo_stack: Vec<Snapshot>,
    pool: Vec<Set>,
}

fn view_of(t: &wgpu::Texture) -> wgpu::TextureView {
    t.create_view(&wgpu::TextureViewDescriptor::default())
}

fn texture(device: &wgpu::Device, size: u32, format: wgpu::TextureFormat, label: &str) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn new_set(device: &wgpu::Device, size: u32) -> Set {
    Set {
        color: texture(device, size, LAYER_FORMAT, "layer color"),
        val: texture(device, size, LAYER_FORMAT, "layer values"),
        cov: texture(device, size, LAYER_FORMAT, "layer coverage"),
        mask: texture(device, size, LAYER_FORMAT, "layer mask"),
    }
}

fn new_pair(device: &wgpu::Device, size: u32) -> Pair {
    Pair { color: texture(device, size, LAYER_FORMAT, "composite color"), mat: texture(device, size, LAYER_FORMAT, "composite material") }
}

fn rgba_texture(device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32, bytes: &[u8]) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("baked image"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(texture.as_image_copy(), bytes, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) }, wgpu::Extent3d { width, height, depth_or_array_layers: 1 });
    texture
}

fn half_bits(value: f32) -> u16 {
    let x = value.max(0.0).min(65000.0).to_bits();
    let exp = ((x >> 23) & 0xff) as i32 - 127 + 15;
    let mant = x & 0x7f_ffff;
    if exp <= 0 {
        if exp < -10 {
            return 0;
        }
        let m = mant | 0x80_0000;
        return (m >> (14 - exp)) as u16;
    }
    ((exp as u32) << 10 | (mant >> 13)) as u16
}

fn half_texture(device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32, rgb: &[f32]) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("environment"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: LAYER_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut halves = Vec::with_capacity((width * height * 4) as usize);
    for p in rgb.chunks_exact(3) {
        halves.extend_from_slice(&[half_bits(p[0]), half_bits(p[1]), half_bits(p[2]), half_bits(1.0)]);
    }
    queue.write_texture(texture.as_image_copy(), bytemuck::cast_slice(&halves), wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 8), rows_per_image: Some(height) }, wgpu::Extent3d { width, height, depth_or_array_layers: 1 });
    texture
}

fn shrink(rgb: &[f32], width: usize, height: usize, to_w: usize, to_h: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; to_w * to_h * 3];
    for y in 0..to_h {
        let (y0, y1) = (y * height / to_h, ((y + 1) * height / to_h).max(y * height / to_h + 1));
        for x in 0..to_w {
            let (x0, x1) = (x * width / to_w, ((x + 1) * width / to_w).max(x * width / to_w + 1));
            let mut sum = [0.0f32; 3];
            for sy in y0..y1.min(height) {
                for sx in x0..x1.min(width) {
                    let i = (sy * width + sx) * 3;
                    for c in 0..3 {
                        sum[c] += rgb[i + c];
                    }
                }
            }
            let n = ((y1.min(height) - y0) * (x1.min(width) - x0)).max(1) as f32;
            for c in 0..3 {
                out[(y * to_w + x) * 3 + c] = sum[c] / n;
            }
        }
    }
    out
}

fn blur_wrap(rgb: &[f32], width: usize, height: usize, passes: usize) -> Vec<f32> {
    let mut cur = rgb.to_vec();
    for _ in 0..passes {
        let mut next = vec![0.0f32; cur.len()];
        for y in 0..height {
            for x in 0..width {
                let mut sum = [0.0f32; 3];
                let mut n = 0.0;
                for dy in -1i32..=1 {
                    let yy = y as i32 + dy;
                    if yy < 0 || yy >= height as i32 {
                        continue;
                    }
                    for dx in -1i32..=1 {
                        let xx = (x as i32 + dx).rem_euclid(width as i32) as usize;
                        let i = (yy as usize * width + xx) * 3;
                        for c in 0..3 {
                            sum[c] += cur[i + c];
                        }
                        n += 1.0;
                    }
                }
                for c in 0..3 {
                    next[(y * width + x) * 3 + c] = sum[c] / n;
                }
            }
        }
        cur = next;
    }
    cur
}

fn depth_only_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("bake depth"), source: wgpu::ShaderSource::Wgsl(shaders::BAKE_DEPTH.into()) });
    let buffers = [Some(mesh_layout())];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bake depth"),
        layout: None,
        vertex: wgpu::VertexState { module: &module, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &buffers },
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: None,
        multiview_mask: None,
        cache: None,
    })
}

fn white_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("white"),
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(texture.as_image_copy(), &[255, 255, 255, 255], wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) }, wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 });
    texture
}

fn depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("view depth"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn scene_texture(device: &wgpu::Device, format: wgpu::TextureFormat, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("post scene"),
        size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn bloom_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("post bloom"),
        size: wgpu::Extent3d { width: (width / 4).max(1), height: (height / 4).max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: LAYER_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn mesh_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
    wgpu::VertexBufferLayout { array_stride: 32, step_mode: wgpu::VertexStepMode::Vertex, attributes: &ATTRS }
}

fn targets(formats: &[wgpu::TextureFormat], blend: Option<wgpu::BlendState>) -> Vec<Option<wgpu::ColorTargetState>> {
    formats.iter().map(|f| Some(wgpu::ColorTargetState { format: *f, blend, write_mask: wgpu::ColorWrites::ALL })).collect()
}

fn pipeline(device: &wgpu::Device, label: &str, source: &str, mesh: bool, formats: &[wgpu::TextureFormat], blend: Option<wgpu::BlendState>, depth: bool) -> wgpu::RenderPipeline {
    let code = if mesh { source.to_string() } else { format!("{}\n{}", shaders::FULLSCREEN, source) };
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(label), source: wgpu::ShaderSource::Wgsl(code.into()) });
    let buffers = if mesh { vec![Some(mesh_layout())] } else { vec![] };
    let color_targets = targets(formats, blend);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: None,
        vertex: wgpu::VertexState { module: &module, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &buffers },
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: if depth {
            Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            })
        } else {
            None
        },
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs"), compilation_options: Default::default(), targets: &color_targets }),
        multiview_mask: None,
        cache: None,
    })
}

fn attach<'a>(view: &'a wgpu::TextureView, load: wgpu::LoadOp<wgpu::Color>) -> Option<wgpu::RenderPassColorAttachment<'a>> {
    Some(wgpu::RenderPassColorAttachment { view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load, store: wgpu::StoreOp::Store } })
}

fn bind(device: &wgpu::Device, pipe: &wgpu::RenderPipeline, entries: &[wgpu::BindingResource]) -> wgpu::BindGroup {
    let entries: Vec<wgpu::BindGroupEntry> = entries.iter().enumerate().map(|(i, r)| wgpu::BindGroupEntry { binding: i as u32, resource: r.clone() }).collect();
    device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &pipe.get_bind_group_layout(0), entries: &entries })
}

pub fn view_projection(eye: Vec3, forward: Vec3, right: Vec3, up: Vec3, fov: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
    let f = 1.0 / (fov * 0.5).tan();
    let a = far / (near - far);
    let b = near * far / (near - far);
    let view_row = |axis: Vec3, sign: f32| [axis[0] * sign, axis[1] * sign, axis[2] * sign, -dot(axis, eye) * sign];
    let r = view_row(right, 1.0);
    let u = view_row(up, 1.0);
    let z = view_row(forward, -1.0);
    let rows = [
        [r[0] * f / aspect, r[1] * f / aspect, r[2] * f / aspect, r[3] * f / aspect],
        [u[0] * f, u[1] * f, u[2] * f, u[3] * f],
        [z[0] * a, z[1] * a, z[2] * a, z[3] * a + b],
        [-z[0], -z[1], -z[2], -z[3]],
    ];
    let mut m = [0.0f32; 16];
    for c in 0..4 {
        for r in 0..4 {
            m[c * 4 + r] = rows[r][c];
        }
    }
    m
}

fn read_rgba(buffer: wgpu::Buffer, width: u32, height: u32, row: u32, bgra: bool, keep_alpha: bool) -> js_sys::Promise {
    js_sys::Promise::new(&mut |resolve, reject| {
        let readback = buffer.clone();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            let fail = |e: String| {
                let _ = reject.call1(&JsValue::NULL, &JsValue::from_str(&format!("Viewport readback failed: {e}")));
            };
            if let Err(e) = result {
                return fail(e.to_string());
            }
            let pixels = match readback.slice(..).get_mapped_range() {
                Ok(view) => {
                    let mut out = Vec::with_capacity((width * height * 4) as usize);
                    for y in 0..height as usize {
                        let line = &view[y * row as usize..y * row as usize + width as usize * 4];
                        for px in line.chunks_exact(4) {
                            let a = if keep_alpha { px[3] } else { 255 };
                            if bgra { out.extend_from_slice(&[px[2], px[1], px[0], a]) } else { out.extend_from_slice(&[px[0], px[1], px[2], a]) }
                        }
                    }
                    out
                }
                Err(e) => return fail(e.to_string()),
            };
            readback.unmap();
            let _ = resolve.call1(&JsValue::NULL, &js_sys::Uint8Array::from(&pixels[..]));
        });
    })
}

fn camera_bytes(view_proj: [f32; 16], eye: Vec3, cursor: Option<(Vec3, Vec3, f32)>, exposure: f32, normal_strength: f32, overlay: f32) -> Vec<f32> {
    let mut data = view_proj.to_vec();
    data.extend_from_slice(&[eye[0], eye[1], eye[2], 1.0]);
    match cursor {
        Some((p, n, r)) => {
            data.extend_from_slice(&[p[0], p[1], p[2], r]);
            data.extend_from_slice(&[n[0], n[1], n[2], 0.0]);
        }
        None => data.extend_from_slice(&[0.0; 8]),
    }
    data.extend_from_slice(&[exposure, normal_strength, overlay, 0.0]);
    data
}

impl Gpu {
    pub async fn new(canvas: web_sys::HtmlCanvasElement, width: u32, height: u32, size: u32) -> Result<Gpu, String> {
        let size = size.clamp(256, 4096).next_power_of_two();
        let navigator = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("navigator")).unwrap_or(JsValue::UNDEFINED);
        let has_webgpu = !navigator.is_undefined() && js_sys::Reflect::get(&navigator, &JsValue::from_str("gpu")).map(|g| !g.is_undefined() && !g.is_null()).unwrap_or(false);
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = if has_webgpu { wgpu::Backends::BROWSER_WEBGPU } else { wgpu::Backends::GL };
        let instance = wgpu::Instance::new(descriptor);
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas)).map_err(|e| format!("Could not create a drawing surface: {e}"))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::HighPerformance, compatible_surface: Some(&surface), force_fallback_adapter: false, apply_limit_buckets: false })
            .await
            .map_err(|e| format!("No GPU adapter is available: {e}"))?;
        let backend = format!("{:?}", adapter.get_info().backend);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor { label: Some("formsurface"), required_limits: adapter.limits(), ..Default::default() })
            .await
            .map_err(|e| format!("The GPU device could not be opened: {e}"))?;
        device.on_uncaptured_error(std::sync::Arc::new(|error| web_sys::console::error_1(&JsValue::from_str(&format!("FormSurface GPU error: {error}")))));
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);
        let uniform = |label: &str, bytes: u64| device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: bytes, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let union = wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::OneMinusDst, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::OneMinusDstAlpha, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
        };
        let gpu = Gpu {
            backend,
            size,
            default_size: size,
            resample_pipe: pipeline(&device, "resample layer", shaders::RESAMPLE, false, &[LAYER_FORMAT; 4], None, false),
            resample_one_pipe: pipeline(&device, "resample texture", shaders::RESAMPLE_ONE, false, &[LAYER_FORMAT], None, false),
            base: [0.5, 0.5, 0.5, 0.6, 0.0],
            view_pipe: pipeline(&device, "view", &format!("{}{}", shaders::ENV_COMMON, shaders::VIEW), true, &[format], None, true),
            background_pipe: pipeline(&device, "background", &format!("{}{}", shaders::ENV_COMMON, shaders::BACKGROUND), false, &[format], None, true),
            env_uniform: uniform("environment", 80),
            env_textures: [half_texture(&device, &queue, 1, 1, &[0.0; 3]), half_texture(&device, &queue, 1, 1, &[0.0; 3]), half_texture(&device, &queue, 1, 1, &[0.0; 3])],
            env_params: [0.0; 20],
            post_pipe: pipeline(&device, "post", &format!("{}{}", shaders::POST_COMMON, shaders::POST), false, &[format], None, false),
            bloom_pipe: pipeline(&device, "bloom", &format!("{}{}", shaders::POST_COMMON, shaders::BLOOM), false, &[LAYER_FORMAT], None, false),
            reproject_pipe: pipeline(&device, "reproject", shaders::REPROJECT, false, &[LAYER_FORMAT; 4], None, false),
            trace_pipe: pipeline(&device, "ray traced bake", shaders::TRACE, false, &[LAYER_FORMAT], None, false),
            trace_job: None,
            path_pipe: pipeline(&device, "path trace", &format!("{}{}", shaders::ENV_COMMON, shaders::PATH), false, &[wgpu::TextureFormat::Rgba32Float, LAYER_FORMAT, LAYER_FORMAT], None, false),
            path_filter_pipe: pipeline(&device, "path denoise", shaders::PATH_FILTER, false, &[wgpu::TextureFormat::Rgba32Float], None, false),
            path_filter_uniforms: [uniform("path denoise", 16), uniform("path denoise", 16), uniform("path denoise", 16), uniform("path denoise", 16)],
            path_denoise: false,
            path_lens: [0.0, 1.0],
            path_scatter: 0.05,
            path_size: None,
            env_cdf: {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("environment distribution"), size: 32, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
                queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&[1.0f32, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0]));
                buffer
            },
            path_show_pipe: pipeline(&device, "path show", &format!("{}{}", shaders::ENV_COMMON, shaders::PATH_SHOW), false, &[format], None, false),
            path_gather_pipe: pipeline(&device, "path gather", shaders::PATH_GATHER, false, &[LAYER_FORMAT; 3], None, false),
            path_uniform: uniform("path trace", 48),
            path_show_uniform: uniform("path show", 32),
            path: None,
            path_view: false,
            path_inflight: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            post_uniform: uniform("post", 96),
            post_params: [0.0; 24],
            scene: scene_texture(&device, format, width, height),
            bloom: bloom_texture(&device, width, height),
            paint_pipe: pipeline(&device, "paint", shaders::PAINT, true, &[LAYER_FORMAT], Some(union), false),
            apply_pipe: pipeline(&device, "apply", shaders::APPLY, false, &[LAYER_FORMAT; 3], None, false),
            composite_pipe: pipeline(&device, "composite", &format!("{}{}", shaders::BLEND, shaders::COMPOSITE), false, &[LAYER_FORMAT; 2], None, false),
            dilate_pipe: pipeline(&device, "dilate", shaders::DILATE, false, &[LAYER_FORMAT; 2], None, false),
            coverage_pipe: pipeline(&device, "coverage", shaders::COVERAGE, true, &[wgpu::TextureFormat::R8Unorm], None, false),
            export_pipe: pipeline(&device, "export", shaders::EXPORT, false, &[wgpu::TextureFormat::Rgba8Unorm], None, false),
            export16_pipe: pipeline(&device, "export 16-bit", shaders::EXPORT, false, &[LAYER_FORMAT], None, false),
            region_pipe: pipeline(&device, "polygon fill", shaders::COVERAGE, true, &[LAYER_FORMAT], None, false),
            fx_pipe: pipeline(&device, "mask effect", shaders::MASK_FX, false, &[LAYER_FORMAT], None, false),
            fx_uniform: uniform("mask effect", 32),
            cover_pipe: pipeline(&device, "path cover", shaders::PATH_COVER, true, &[LAYER_FORMAT], None, false),
            depth: depth_texture(&device, width, height),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                ..Default::default()
            }),
            camera: uniform("camera", 128),
            stamp_buffer: uniform("stamps", 64 + 32 * 64),
            fill_uniform: uniform("fill", 288),
            bake: texture(&device, size, LAYER_FORMAT, "baked maps"),
            baked: false,
            baked_normal: rgba_texture(&device, &queue, 1, 1, &[128, 128, 255, 255]),
            baked_id: rgba_texture(&device, &queue, 1, 1, &[128, 128, 128, 255]),
            high_baked: false,
            light_uniform: uniform("bake light", 96),
            bake_depth_pipe: depth_only_pipeline(&device),
            bake_accum_pipe: pipeline(&device, "bake accumulate", shaders::BAKE_ACCUM, false, &[LAYER_FORMAT], Some(wgpu::BlendState {
                color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
                alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            }), false),
            bake_final_pipe: pipeline(&device, "bake final", shaders::BAKE_FINAL, false, &[LAYER_FORMAT], None, false),
            fill_pipe: pipeline(&device, "fill", shaders::FILL, false, &[LAYER_FORMAT; 3], None, false),
            apply_mask_pipe: pipeline(&device, "apply mask", shaders::APPLY_MASK, false, &[LAYER_FORMAT], None, false),
            mask_mul_pipe: pipeline(&device, "folder mask", shaders::MASK_MUL, false, &[LAYER_FORMAT], Some(wgpu::BlendState {
                color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Dst, dst_factor: wgpu::BlendFactor::Zero, operation: wgpu::BlendOperation::Add },
                alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::Zero, operation: wgpu::BlendOperation::Add },
            }), false),
            invert_pipe: pipeline(&device, "invert mask", shaders::WHITE, false, &[LAYER_FORMAT], Some(wgpu::BlendState {
                color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::OneMinusDst, dst_factor: wgpu::BlendFactor::Zero, operation: wgpu::BlendOperation::Add },
                alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::Zero, operation: wgpu::BlendOperation::Add },
            }), false),
            mask_image_pipe: pipeline(&device, "mask from image", shaders::MASK_IMAGE, false, &[LAYER_FORMAT], None, false),
            mask_id_pipe: pipeline(&device, "mask from id", shaders::MASK_ID, false, &[LAYER_FORMAT], None, false),
            flatten_pipe: pipeline(&device, "flatten", shaders::PAIR_TO_SET, false, &[LAYER_FORMAT; 3], None, false),
            merge_pipe: pipeline(&device, "merge", &format!("{}{}", shaders::BLEND, shaders::MERGE), false, &[LAYER_FORMAT; 3], None, false),
            merge_uniform: uniform("merge", 48),
            layer_export_pipe: pipeline(&device, "layer export", shaders::LAYER_EXPORT, false, &[wgpu::TextureFormat::Rgba8Unorm], None, false),
            mask_export_pipe: pipeline(&device, "mask export", shaders::MASK_MUL, false, &[wgpu::TextureFormat::Rgba8Unorm], None, false),
            group_mask: texture(&device, size, LAYER_FORMAT, "folder mask"),
            quick: texture(&device, size, LAYER_FORMAT, "quick mask"),
            quick_mode: 0,
            clone_offset: None,
            clone_layer: None,
            flat: false,
            flat_channel: 0,
            geometry_pipe: pipeline(&device, "geometry", shaders::GEOMETRY, true, &[wgpu::TextureFormat::Rgba32Float, LAYER_FORMAT], None, false),
            geometry: None,
            patterns: std::collections::HashMap::new(),
            tile_patterns: std::collections::HashMap::new(),
            white: white_texture(&device, &queue),
            clamp_sampler: device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() }),
            alpha_tex: white_texture(&device, &queue),
            stencil_tex: white_texture(&device, &queue),
            paint_uniform: uniform("paint", 80),
            extra_role: 0.0,
            layer_uniform: uniform("layer", 96),
            export_uniform: uniform("export", 32),
            vertices: None,
            indices: None,
            range: (0, 0),
            total: 0,
            parked: vec![None],
            current: 0,
            coverage: texture(&device, size, wgpu::TextureFormat::R8Unorm, "uv coverage"),
            mask: texture(&device, size, LAYER_FORMAT, "stroke mask"),
            ping: new_pair(&device, size),
            pong: new_pair(&device, size),
            final_set: new_pair(&device, size),
            scratch: new_set(&device, size),
            layers: Vec::new(),
            stroke: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pool: Vec::new(),
            surface,
            config,
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
        };
        Ok(gpu)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth = depth_texture(&self.device, width, height);
        self.scene = scene_texture(&self.device, self.config.format, width, height);
        self.bloom = bloom_texture(&self.device, width, height);
    }

    pub fn upload_mesh(&mut self, mesh: &Mesh, ranges: &[(u32, u32)], carry: &[Option<usize>]) {
        self.tile_patterns.clear();
        self.path = None;
        let data = mesh.interleaved();
        let upload = |label: &str, bytes: &[u8], usage: wgpu::BufferUsages| {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: bytes.len().next_multiple_of(4) as u64, usage: usage | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            self.queue.write_buffer(&buffer, 0, bytes);
            buffer
        };
        self.vertices = Some(upload("vertices", bytemuck::cast_slice(&data), wgpu::BufferUsages::VERTEX));
        self.indices = Some(upload("indices", bytemuck::cast_slice(&mesh.indices), wgpu::BufferUsages::INDEX));
        self.total = mesh.indices.len() as u32;
        let ranges: Vec<(u32, u32)> = if ranges.is_empty() { vec![(0, self.total)] } else { ranges.to_vec() };
        self.select_set(0);
        let mut old: Vec<Option<SetState>> = std::mem::take(&mut self.parked);
        let mut live = self.new_state(self.range);
        self.exchange(&mut live);
        if old.is_empty() {
            old.push(None);
        }
        old[0] = Some(live);
        let mut fresh: Vec<Option<SetState>> = Vec::with_capacity(ranges.len());
        for (k, range) in ranges.iter().enumerate() {
            let kept = carry.get(k).copied().flatten().and_then(|j| old.get_mut(j).and_then(|s| s.take()));
            let state = match kept {
                Some(mut s) => {
                    s.range = *range;
                    s.geometry = None;
                    s.baked = false;
                    s.high_baked = false;
                    s
                }
                None => self.new_state(*range),
            };
            fresh.push(Some(state));
        }
        self.parked = fresh;
        let mut first = self.parked[0].take().unwrap_or_else(|| self.new_state(ranges[0]));
        self.exchange(&mut first);
        self.current = 0;
        self.fit_shared();
        self.range = ranges[0];
        for k in 0..ranges.len() {
            self.select_set(k);
            self.prepare_set();
        }
        self.select_set(0);
    }

    fn new_state(&self, range: (u32, u32)) -> SetState {
        let size = self.default_size;
        SetState {
            size,
            extra_role: 0.0,
            base: self.base,
            range,
            layers: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            final_set: new_pair(&self.device, size),
            coverage: texture(&self.device, size, wgpu::TextureFormat::R8Unorm, "uv coverage"),
            geometry: None,
            bake: texture(&self.device, size, LAYER_FORMAT, "baked maps"),
            baked: false,
            baked_normal: rgba_texture(&self.device, &self.queue, 1, 1, &[128, 128, 255, 255]),
            baked_id: rgba_texture(&self.device, &self.queue, 1, 1, &[128, 128, 128, 255]),
            high_baked: false,
            quick: texture(&self.device, size, LAYER_FORMAT, "quick mask"),
            mask: texture(&self.device, size, LAYER_FORMAT, "stroke mask"),
            stroke: None,
        }
    }

    fn fit_shared(&mut self) {
        if self.group_mask.width() == self.size {
            return;
        }
        self.group_mask = texture(&self.device, self.size, LAYER_FORMAT, "folder mask");
        self.ping = new_pair(&self.device, self.size);
        self.pong = new_pair(&self.device, self.size);
        self.scratch = new_set(&self.device, self.size);
    }

    pub fn set_extra_role(&self, index: usize) -> f32 {
        if index == self.current {
            return self.extra_role;
        }
        self.parked.get(index).and_then(|s| s.as_ref()).map(|s| s.extra_role).unwrap_or(0.0)
    }

    pub fn set_size(&self, index: usize) -> u32 {
        if index == self.current {
            return self.size;
        }
        self.parked.get(index).and_then(|s| s.as_ref()).map(|s| s.size).unwrap_or(self.default_size)
    }

    pub fn resize_current(&mut self, size: u32) {
        if size == self.size {
            return;
        }
        self.size = size;
        self.fit_shared();
        let layers = std::mem::take(&mut self.layers);
        let mut resized = Vec::with_capacity(layers.len());
        for layer in layers {
            let next = new_set(&self.device, size);
            self.resample_set(&layer.set, &next);
            let fx = layer.fx.as_ref().map(|_| [texture(&self.device, size, LAYER_FORMAT, "mask effects"), texture(&self.device, size, LAYER_FORMAT, "mask effects")]);
            resized.push(LayerTex { id: layer.id, set: next, effects: layer.effects.clone(), fx });
        }
        self.layers = resized;
        let quick = texture(&self.device, size, LAYER_FORMAT, "quick mask");
        self.resample_one(&self.quick, &quick);
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.pool.clear();
        self.final_set = new_pair(&self.device, size);
        self.coverage = texture(&self.device, size, wgpu::TextureFormat::R8Unorm, "uv coverage");
        self.bake = texture(&self.device, size, LAYER_FORMAT, "baked maps");
        self.mask = texture(&self.device, size, LAYER_FORMAT, "stroke mask");
        self.prepare_set();
        self.quick = quick;
    }

    fn resample_set(&self, from: &Set, to: &Set) {
        let views = [view_of(&from.color), view_of(&from.val), view_of(&from.cov), view_of(&from.mask)];
        let group = bind(&self.device, &self.resample_pipe, &[
            wgpu::BindingResource::TextureView(&views[0]),
            wgpu::BindingResource::TextureView(&views[1]),
            wgpu::BindingResource::TextureView(&views[2]),
            wgpu::BindingResource::TextureView(&views[3]),
            wgpu::BindingResource::Sampler(&self.clamp_sampler),
        ]);
        let targets = [view_of(&to.color), view_of(&to.val), view_of(&to.cov), view_of(&to.mask)];
        let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[attach(&targets[0], clear), attach(&targets[1], clear), attach(&targets[2], clear), attach(&targets[3], clear)],
                ..Default::default()
            });
            pass.set_pipeline(&self.resample_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
    }

    fn resample_one(&self, from: &wgpu::Texture, to: &wgpu::Texture) {
        let view = view_of(from);
        let group = bind(&self.device, &self.resample_one_pipe, &[wgpu::BindingResource::TextureView(&view), wgpu::BindingResource::Sampler(&self.clamp_sampler)]);
        let target = view_of(to);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&target, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))], ..Default::default() });
            pass.set_pipeline(&self.resample_one_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
    }

    fn exchange(&mut self, s: &mut SetState) {
        std::mem::swap(&mut self.size, &mut s.size);
        std::mem::swap(&mut self.extra_role, &mut s.extra_role);
        std::mem::swap(&mut self.base, &mut s.base);
        std::mem::swap(&mut self.range, &mut s.range);
        std::mem::swap(&mut self.layers, &mut s.layers);
        std::mem::swap(&mut self.undo_stack, &mut s.undo_stack);
        std::mem::swap(&mut self.redo_stack, &mut s.redo_stack);
        std::mem::swap(&mut self.final_set, &mut s.final_set);
        std::mem::swap(&mut self.coverage, &mut s.coverage);
        std::mem::swap(&mut self.geometry, &mut s.geometry);
        std::mem::swap(&mut self.bake, &mut s.bake);
        std::mem::swap(&mut self.baked, &mut s.baked);
        std::mem::swap(&mut self.baked_normal, &mut s.baked_normal);
        std::mem::swap(&mut self.baked_id, &mut s.baked_id);
        std::mem::swap(&mut self.high_baked, &mut s.high_baked);
        std::mem::swap(&mut self.quick, &mut s.quick);
        std::mem::swap(&mut self.mask, &mut s.mask);
        std::mem::swap(&mut self.stroke, &mut s.stroke);
    }

    pub fn quick_fill(&self, value: f64) {
        let view = view_of(&self.quick);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&view, wgpu::LoadOp::Clear(wgpu::Color { r: value, g: value, b: value, a: 1.0 }))], ..Default::default() });
        self.queue.submit([encoder.finish()]);
    }

    pub fn quick_invert(&self) {
        let view = view_of(&self.quick);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&view, wgpu::LoadOp::Load)], ..Default::default() });
            pass.set_pipeline(&self.invert_pipe);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
    }

    pub fn select_set(&mut self, to: usize) {
        if to == self.current || to >= self.parked.len() {
            return;
        }
        let Some(mut state) = self.parked[to].take() else { return };
        self.exchange(&mut state);
        self.parked[self.current] = Some(state);
        self.current = to;
        self.fit_shared();
    }

    fn prepare_set(&mut self) {
        let view = view_of(&self.coverage);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&view, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))], ..Default::default() });
            pass.set_pipeline(&self.coverage_pipe);
            self.draw_mesh(&mut pass);
        }
        let pos = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("uv position map"),
            size: wgpu::Extent3d { width: self.size, height: self.size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let normal = texture(&self.device, self.size, LAYER_FORMAT, "uv normal map");
        {
            let (pv, nv) = (view_of(&pos), view_of(&normal));
            let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&pv, clear), attach(&nv, clear)], ..Default::default() });
            pass.set_pipeline(&self.geometry_pipe);
            self.draw_mesh(&mut pass);
        }
        {
            let bv = view_of(&self.bake);
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&bv, wgpu::LoadOp::Clear(wgpu::Color { r: 1.0, g: 0.5, b: 0.5, a: 1.0 }))], ..Default::default() });
        }
        self.queue.submit([encoder.finish()]);
        self.geometry = Some((pos, normal));
        self.baked = false;
        self.baked_normal = rgba_texture(&self.device, &self.queue, 1, 1, &[128, 128, 255, 255]);
        self.baked_id = rgba_texture(&self.device, &self.queue, 1, 1, &[128, 128, 128, 255]);
        self.high_baked = false;
        self.quick_fill(0.0);
    }

    pub fn set_high_bake_for(&mut self, index: usize, size: u32, normal: &[u8], id: &[u8]) {
        if index == self.current {
            return self.set_high_bake(size, normal, id);
        }
        let (n, i) = (rgba_texture(&self.device, &self.queue, size, size, normal), rgba_texture(&self.device, &self.queue, size, size, id));
        if let Some(Some(state)) = self.parked.get_mut(index) {
            state.baked_normal = n;
            state.baked_id = i;
            state.high_baked = true;
        }
    }

    pub fn set_high_bake(&mut self, size: u32, normal: &[u8], id: &[u8]) {
        self.baked_normal = rgba_texture(&self.device, &self.queue, size, size, normal);
        self.baked_id = rgba_texture(&self.device, &self.queue, size, size, id);
        self.high_baked = true;
    }

    fn draw_mesh(&self, pass: &mut wgpu::RenderPass) {
        if let (Some(v), Some(i)) = (&self.vertices, &self.indices) {
            pass.set_vertex_buffer(0, v.slice(..));
            pass.set_index_buffer(i.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(self.range.0..self.range.0 + self.range.1, 0, 0..1);
        }
    }

    fn draw_all(&self, pass: &mut wgpu::RenderPass) {
        if let (Some(v), Some(i)) = (&self.vertices, &self.indices) {
            pass.set_vertex_buffer(0, v.slice(..));
            pass.set_index_buffer(i.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.total, 0, 0..1);
        }
    }

    fn clear_set(&self, set: &Set) {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        for t in [&set.color, &set.val, &set.cov] {
            let view = view_of(t);
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&view, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))], ..Default::default() });
        }
        let mask = view_of(&set.mask);
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&mask, wgpu::LoadOp::Clear(wgpu::Color::WHITE))], ..Default::default() });
        self.queue.submit([encoder.finish()]);
    }

    pub fn fill_mask_value(&mut self, index: usize, value: f64) {
        let view = view_of(&self.layers[index].set.mask);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&view, wgpu::LoadOp::Clear(wgpu::Color { r: value, g: value, b: value, a: 1.0 }))], ..Default::default() });
        self.queue.submit([encoder.finish()]);
    }

    pub fn fill_layer_content(&mut self, index: usize, uniform: &[f32; 72]) {
        self.fill_target(self.layers[index].id, &self.layers[index].set, uniform);
    }

    pub fn mask_from_fill(&mut self, index: usize, uniform: &[f32; 72]) {
        self.fill_target(self.layers[index].id, &self.scratch, uniform);
        let extent = wgpu::Extent3d { width: self.size, height: self.size, depth_or_array_layers: 1 };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_texture(self.scratch.cov.as_image_copy(), self.layers[index].set.mask.as_image_copy(), extent);
        self.queue.submit([encoder.finish()]);
    }

    fn mask_pass(&self, index: usize, pipe: &wgpu::RenderPipeline, group: &wgpu::BindGroup) {
        let target = view_of(&self.layers[index].set.mask);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&target, wgpu::LoadOp::Clear(wgpu::Color::BLACK))], ..Default::default() });
            pass.set_pipeline(pipe);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
    }

    pub fn mask_from_image(&mut self, index: usize, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
        if width == 0 || height == 0 || width > 8192 || height > 8192 || rgba.len() != (width * height * 4) as usize {
            return Err("Mask images must be RGBA, 1 to 8192 pixels per side".into());
        }
        let image = rgba_texture(&self.device, &self.queue, width, height, rgba);
        let view = view_of(&image);
        let group = bind(&self.device, &self.mask_image_pipe, &[wgpu::BindingResource::TextureView(&view), wgpu::BindingResource::Sampler(&self.clamp_sampler)]);
        self.mask_pass(index, &self.mask_image_pipe, &group);
        Ok(())
    }

    pub fn mask_from_id(&mut self, index: usize, color: [f32; 3], tolerance: f32) -> Result<(), String> {
        if !self.high_baked {
            return Err("Bake normal and ID from a high-poly mesh first; color selection reads the ID map".into());
        }
        self.queue.write_buffer(&self.export_uniform, 0, bytemuck::cast_slice(&[color[0], color[1], color[2], tolerance, 0.0, 0.0, 0.0, 0.0]));
        let view = view_of(&self.baked_id);
        let group = bind(&self.device, &self.mask_id_pipe, &[self.export_uniform.as_entire_binding(), wgpu::BindingResource::TextureView(&view)]);
        self.mask_pass(index, &self.mask_id_pipe, &group);
        Ok(())
    }

    pub fn flatten_into(&mut self, index: usize) {
        let (fc, fm) = (view_of(&self.final_set.color), view_of(&self.final_set.mat));
        let group = bind(&self.device, &self.flatten_pipe, &[wgpu::BindingResource::TextureView(&fc), wgpu::BindingResource::TextureView(&fm)]);
        let set = &self.layers[index].set;
        let (c, v, k) = (view_of(&set.color), view_of(&set.val), view_of(&set.cov));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&c, clear), attach(&v, clear), attach(&k, clear)], ..Default::default() });
            pass.set_pipeline(&self.flatten_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
    }

    pub fn merge_layers(&mut self, top: usize, bottom: usize, target: usize, top_params: [f32; 2], bottom_params: [f32; 2], blend: u32) {
        let data: [f32; 12] = [top_params[0], top_params[1], 0.0, 0.0, bottom_params[0], bottom_params[1], 0.0, 0.0, f32::from_bits(blend), 0.0, 0.0, 0.0];
        self.queue.write_buffer(&self.merge_uniform, 0, bytemuck::cast_slice(&data));
        let out = self.take_set();
        {
            let (t, b) = (&self.layers[top].set, &self.layers[bottom].set);
            let views = [&t.color, &t.val, &t.cov, &t.mask, &b.color, &b.val, &b.cov, &b.mask].map(view_of);
            let mut entries = vec![self.merge_uniform.as_entire_binding()];
            entries.extend(views.iter().map(wgpu::BindingResource::TextureView));
            let group = bind(&self.device, &self.merge_pipe, &entries);
            let (c, v, k) = (view_of(&out.color), view_of(&out.val), view_of(&out.cov));
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&c, clear), attach(&v, clear), attach(&k, clear)], ..Default::default() });
                pass.set_pipeline(&self.merge_pipe);
                pass.set_bind_group(0, &group, &[]);
                pass.draw(0..3, 0..1);
            }
            let extent = wgpu::Extent3d { width: self.size, height: self.size, depth_or_array_layers: 1 };
            let dest = &self.layers[target].set;
            for (a, b) in [(&out.color, &dest.color), (&out.val, &dest.val), (&out.cov, &dest.cov)] {
                encoder.copy_texture_to_texture(a.as_image_copy(), b.as_image_copy(), extent);
            }
            self.queue.submit([encoder.finish()]);
        }
        self.pool.push(out);
    }

    pub fn reproject_layers(&mut self, map: &[f32]) -> Result<(), String> {
        let size = self.size;
        if map.len() != (size * size * 4) as usize {
            return Err("Reprojection map does not match the texture size".into());
        }
        let map_tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("reprojection map"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(map_tex.as_image_copy(), bytemuck::cast_slice(map), wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size * 16), rows_per_image: Some(size) }, wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 });
        let map_view = view_of(&map_tex);
        let extent = wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 };
        for index in 0..self.layers.len() {
            let out = self.take_set();
            {
                let s = &self.layers[index].set;
                let views = [&s.color, &s.val, &s.cov, &s.mask].map(view_of);
                let mut entries = vec![wgpu::BindingResource::TextureView(&map_view)];
                entries.extend(views.iter().map(wgpu::BindingResource::TextureView));
                entries.push(wgpu::BindingResource::Sampler(&self.clamp_sampler));
                let group = bind(&self.device, &self.reproject_pipe, &entries);
                let targets = [&out.color, &out.val, &out.cov, &out.mask].map(view_of);
                let mut encoder = self.device.create_command_encoder(&Default::default());
                {
                    let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&targets[0], clear), attach(&targets[1], clear), attach(&targets[2], clear), attach(&targets[3], clear)], ..Default::default() });
                    pass.set_pipeline(&self.reproject_pipe);
                    pass.set_bind_group(0, &group, &[]);
                    pass.draw(0..3, 0..1);
                }
                let dest = &self.layers[index].set;
                for (a, b) in [(&out.color, &dest.color), (&out.val, &dest.val), (&out.cov, &dest.cov), (&out.mask, &dest.mask)] {
                    encoder.copy_texture_to_texture(a.as_image_copy(), b.as_image_copy(), extent);
                }
                self.queue.submit([encoder.finish()]);
            }
            self.pool.push(out);
        }
        self.undo_stack.clear();
        self.redo_stack.clear();
        Ok(())
    }

    pub fn copy_mask(&mut self, from: usize, to: usize) {
        let extent = wgpu::Extent3d { width: self.size, height: self.size, depth_or_array_layers: 1 };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_texture(self.layers[from].set.mask.as_image_copy(), self.layers[to].set.mask.as_image_copy(), extent);
        self.queue.submit([encoder.finish()]);
    }

    pub fn export_layer(&mut self, index: usize, channel: u32) -> js_sys::Promise {
        let size = self.size;
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("layer export"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let sel = [channel, 0, 0, 0];
        self.queue.write_buffer(&self.export_uniform, 0, bytemuck::cast_slice(&sel));
        let set = &self.layers[index].set;
        let (c, v, k) = (view_of(&set.color), view_of(&set.val), view_of(&set.cov));
        let group = bind(&self.device, &self.layer_export_pipe, &[
            self.export_uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(&c),
            wgpu::BindingResource::TextureView(&v),
            wgpu::BindingResource::TextureView(&k),
        ]);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("layer readback"), size: (size * size * 4) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let tv = view_of(&target);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&tv, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))], ..Default::default() });
            pass.set_pipeline(&self.layer_export_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size * 4), rows_per_image: Some(size) } },
            wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        read_rgba(buffer, size, size, size * 4, false, true)
    }

    pub fn export_mask(&mut self, index: usize) -> js_sys::Promise {
        let size = self.size;
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mask export"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let source = view_of(&self.layers[index].set.mask);
        let group = bind(&self.device, &self.mask_export_pipe, &[wgpu::BindingResource::TextureView(&source)]);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("mask readback"), size: (size * size * 4) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let tv = view_of(&target);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&tv, wgpu::LoadOp::Clear(wgpu::Color::BLACK))], ..Default::default() });
            pass.set_pipeline(&self.mask_export_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size * 4), rows_per_image: Some(size) } },
            wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        read_rgba(buffer, size, size, size * 4, false, false)
    }

    fn fill_target(&self, id: u32, set: &Set, uniform: &[f32; 72]) {
        let Some(geometry) = &self.geometry else { return };
        self.queue.write_buffer(&self.fill_uniform, 0, bytemuck::cast_slice(uniform));
        let (pv, nv) = (view_of(&geometry.0), view_of(&geometry.1));
        let tile = if uniform[12] > 5.5 { self.tile_patterns.get(&(self.current, id)) } else { None };
        let pattern = view_of(tile.or(self.patterns.get(&id)).unwrap_or(&self.white));
        let bake = view_of(&self.bake);
        let group = bind(&self.device, &self.fill_pipe, &[
            self.fill_uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(&pv),
            wgpu::BindingResource::TextureView(&nv),
            wgpu::BindingResource::TextureView(&pattern),
            wgpu::BindingResource::Sampler(&self.sampler),
            wgpu::BindingResource::TextureView(&bake),
        ]);
        let (c, v, k) = (view_of(&set.color), view_of(&set.val), view_of(&set.cov));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&c, clear), attach(&v, clear), attach(&k, clear)], ..Default::default() });
            pass.set_pipeline(&self.fill_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
    }

    pub fn bake_maps(&mut self, center: Vec3, radius: f32, directions: u32) -> Result<(), String> {
        let Some((pos, normal)) = &self.geometry else { return Err("Load a mesh before baking".into()) };
        let res = 1024;
        let depth = depth_texture(&self.device, res, res);
        let depth_view = view_of(&depth);
        let depth_sample = depth.create_view(&wgpu::TextureViewDescriptor { aspect: wgpu::TextureAspect::DepthOnly, ..Default::default() });
        let accum = texture(&self.device, self.size, LAYER_FORMAT, "bake accumulation");
        let accum_view = view_of(&accum);
        let (pv, nv) = (view_of(pos), view_of(normal));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&accum_view, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))], ..Default::default() });
        self.queue.submit([encoder.finish()]);
        let r = radius.max(1e-6);
        let golden = std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
        let count = directions.clamp(8, 256);
        for k in 0..count {
            let y = 1.0 - 2.0 * (k as f32 + 0.5) / count as f32;
            let ring = (1.0 - y * y).max(0.0).sqrt();
            let theta = golden * k as f32;
            let dir = [theta.cos() * ring, y, theta.sin() * ring];
            let up = if dir[1].abs() > 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
            let right = fs_core::normalize(fs_core::cross(up, dir));
            let up = fs_core::cross(dir, right);
            let hs = r * 1.05;
            let range = 4.0 * r;
            let eye = fs_core::add(center, fs_core::scale(dir, 2.0 * r));
            let rows = [
                [right[0] / hs, right[1] / hs, right[2] / hs, -dot(right, center) / hs],
                [up[0] / hs, up[1] / hs, up[2] / hs, -dot(up, center) / hs],
                [-dir[0] / range, -dir[1] / range, -dir[2] / range, dot(dir, eye) / range],
                [0.0, 0.0, 0.0, 1.0],
            ];
            let mut data = [0.0f32; 24];
            for c in 0..4 {
                for row in 0..4 {
                    data[c * 4 + row] = rows[row][c];
                }
            }
            data[16..20].copy_from_slice(&[dir[0], dir[1], dir[2], 0.0]);
            data[20..24].copy_from_slice(&[0.003, range, r, 0.05 * r]);
            self.queue.write_buffer(&self.light_uniform, 0, bytemuck::cast_slice(&data));
            let depth_group = bind(&self.device, &self.bake_depth_pipe, &[self.light_uniform.as_entire_binding()]);
            let accum_group = bind(&self.device, &self.bake_accum_pipe, &[
                self.light_uniform.as_entire_binding(),
                wgpu::BindingResource::TextureView(&pv),
                wgpu::BindingResource::TextureView(&nv),
                wgpu::BindingResource::TextureView(&depth_sample),
            ]);
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &depth_view, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }), stencil_ops: None }),
                    ..Default::default()
                });
                pass.set_pipeline(&self.bake_depth_pipe);
                pass.set_bind_group(0, &depth_group, &[]);
                self.draw_all(&mut pass);
            }
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&accum_view, wgpu::LoadOp::Load)], ..Default::default() });
                pass.set_pipeline(&self.bake_accum_pipe);
                pass.set_bind_group(0, &accum_group, &[]);
                pass.draw(0..3, 0..1);
            }
            self.queue.submit([encoder.finish()]);
        }
        let group = bind(&self.device, &self.bake_final_pipe, &[
            self.light_uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(&accum_view),
            wgpu::BindingResource::TextureView(&pv),
            wgpu::BindingResource::TextureView(&nv),
        ]);
        let bake_view = view_of(&self.bake);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&bake_view, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))], ..Default::default() });
            pass.set_pipeline(&self.bake_final_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        self.baked = true;
        Ok(())
    }

    pub fn trace_begin(&mut self, nodes: &[f32], tris: &[f32], radius: f32, rays: u32) -> Result<(), String> {
        let Some((pos, normal)) = &self.geometry else { return Err("Load a mesh before baking".into()) };
        if !self.baked {
            return Err("Bake mesh maps before tracing".into());
        }
        let storage = |label: &str, data: &[f32]| {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: (data.len() * 4) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            self.queue.write_buffer(&buffer, 0, bytemuck::cast_slice(data));
            buffer
        };
        let node_buffer = storage("bvh nodes", nodes);
        let tri_buffer = storage("bvh triangles", tris);
        let params = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("trace"), size: 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let r = radius.max(1e-6);
        self.queue.write_buffer(&params, 0, bytemuck::cast_slice(&[rays.clamp(4, 256) as f32, 2.0 * r, r, 0.0]));
        let previous = texture(&self.device, self.size, LAYER_FORMAT, "bake before tracing");
        let extent = wgpu::Extent3d { width: self.size, height: self.size, depth_or_array_layers: 1 };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_texture(self.bake.as_image_copy(), previous.as_image_copy(), extent);
        self.queue.submit([encoder.finish()]);
        let (pv, nv, prev_view) = (view_of(pos), view_of(normal), view_of(&previous));
        let group = bind(&self.device, &self.trace_pipe, &[
            params.as_entire_binding(),
            wgpu::BindingResource::TextureView(&pv),
            wgpu::BindingResource::TextureView(&nv),
            wgpu::BindingResource::TextureView(&prev_view),
            node_buffer.as_entire_binding(),
            tri_buffer.as_entire_binding(),
        ]);
        self.trace_job = Some((group, 0));
        Ok(())
    }

    pub fn trace_step(&mut self, rows: u32) -> bool {
        let Some((group, y)) = self.trace_job.take() else { return true };
        let rows = rows.max(1).min(self.size - y);
        let bake_view = view_of(&self.bake);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&bake_view, wgpu::LoadOp::Load)], ..Default::default() });
            pass.set_pipeline(&self.trace_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.set_scissor_rect(0, y, self.size, rows);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        if y + rows < self.size {
            self.trace_job = Some((group, y + rows));
            return false;
        }
        true
    }

    pub fn path_scene(&mut self, nodes: &[f32], tris: &[f32], attrs: &[f32]) {
        let storage = |label: &str, data: &[f32]| {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: (data.len().max(4) * 4) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            self.queue.write_buffer(&buffer, 0, bytemuck::cast_slice(data));
            buffer
        };
        let roles = storage("path roles", &vec![0.0f32; self.parked.len().max(4)]);
        self.path = Some(PathScene { nodes: storage("path nodes", nodes), tris: storage("path triangles", tris), attrs: storage("path attributes", attrs), roles, arrays: None, accum: None, filtered: None, cur: 0, samples: 0, seed: 0 });
    }

    pub fn idle(&self) -> js_sys::Promise {
        js_sys::Promise::new(&mut |resolve, _| {
            self.queue.on_submitted_work_done(move || {
                let _ = resolve.call0(&JsValue::NULL);
            });
        })
    }

    pub fn has_path_scene(&self) -> bool {
        self.path.is_some()
    }

    pub fn drop_path(&mut self) {
        self.path = None;
    }

    pub fn path_samples(&self) -> u32 {
        self.path.as_ref().map(|p| p.samples).unwrap_or(0)
    }

    pub fn path_side(&self) -> u32 {
        self.path.as_ref().and_then(|p| p.arrays.as_ref()).map(|a| a[0].width()).unwrap_or(0)
    }

    pub fn path_reset(&mut self, normal_strength: f32) {
        let Some(mut scene) = self.path.take() else { return };
        let count = self.parked.len().max(1);
        let biggest = (0..count).map(|k| self.set_size(k)).max().unwrap_or(self.size);
        let side = biggest.min(if count <= 2 { 2048 } else { 1024 });
        let fits = scene.arrays.as_ref().map(|a| a[0].width() == side && a[0].depth_or_array_layers() == count as u32).unwrap_or(false);
        if !fits {
            let array = |label: &str| self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: side, height: side, depth_or_array_layers: count as u32 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: LAYER_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            scene.arrays = Some([array("path color"), array("path material"), array("path normal")]);
        }
        let roles: Vec<f32> = (0..count).map(|k| self.set_extra_role(k)).collect();
        self.queue.write_buffer(&scene.roles, 0, bytemuck::cast_slice(&roles));
        let gather = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("path gather"), size: 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        self.queue.write_buffer(&gather, 0, bytemuck::cast_slice(&[normal_strength, 0.0f32, 0.0, 0.0]));
        let (width, height) = self.path_size.unwrap_or((self.config.width.max(1), self.config.height.max(1)));
        let sized = scene.accum.as_ref().map(|a| a[0].width() == width && a[0].height() == height).unwrap_or(false);
        if !sized {
            let accum = |label: &str, format: wgpu::TextureFormat| self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let wide = wgpu::TextureFormat::Rgba32Float;
            scene.accum = Some([accum("path color", wide), accum("path albedo", LAYER_FORMAT), accum("path normal", LAYER_FORMAT), accum("path color", wide), accum("path albedo", LAYER_FORMAT), accum("path normal", LAYER_FORMAT)]);
            scene.filtered = Some([accum("path filtered", wide), accum("path filtered", wide)]);
        }
        let mut encoder = self.device.create_command_encoder(&Default::default());
        if let Some(arrays) = scene.arrays.as_ref() {
            for k in 0..count {
                let source = if k == self.current { Some((&self.final_set, &self.baked_normal)) } else { self.parked.get(k).and_then(|s| s.as_ref()).map(|s| (&s.final_set, &s.baked_normal)) };
                let Some((pair, normal)) = source else { continue };
                let (fc, fm, bn) = (view_of(&pair.color), view_of(&pair.mat), view_of(normal));
                let group = bind(&self.device, &self.path_gather_pipe, &[
                    gather.as_entire_binding(),
                    wgpu::BindingResource::TextureView(&fc),
                    wgpu::BindingResource::TextureView(&fm),
                    wgpu::BindingResource::TextureView(&bn),
                    wgpu::BindingResource::Sampler(&self.sampler),
                ]);
                let layer = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2), base_array_layer: k as u32, array_layer_count: Some(1), ..Default::default() });
                let (lc, lm, ln) = (layer(&arrays[0]), layer(&arrays[1]), layer(&arrays[2]));
                let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&lc, clear), attach(&lm, clear), attach(&ln, clear)], ..Default::default() });
                pass.set_pipeline(&self.path_gather_pipe);
                pass.set_bind_group(0, &group, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        if let Some(accum) = scene.accum.as_ref() {
            let (c, a, n) = (view_of(&accum[scene.cur * 3]), view_of(&accum[scene.cur * 3 + 1]), view_of(&accum[scene.cur * 3 + 2]));
            let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&c, clear), attach(&a, clear), attach(&n, clear)], ..Default::default() });
        }
        self.queue.submit([encoder.finish()]);
        scene.samples = 0;
        self.path = Some(scene);
    }

    pub fn path_sample(&mut self, eye: Vec3, bounces: u32, radius: f32) {
        let Some(scene) = self.path.as_mut() else { return };
        let (Some(arrays), Some(accum)) = (scene.arrays.as_ref(), scene.accum.as_ref()) else { return };
        scene.seed = scene.seed.wrapping_add(1) % 16_000_000;
        self.queue.write_buffer(&self.path_uniform, 0, bytemuck::cast_slice(&[eye[0], eye[1], eye[2], scene.seed as f32, bounces as f32, radius.max(1e-6) * 2e-4, self.path_lens[0], self.path_lens[1], self.path_scatter.max(1e-6), 0.0, 0.0, 0.0]));
        self.queue.write_buffer(&self.env_uniform, 0, bytemuck::cast_slice(&self.env_params));
        let (e0, e1, e2) = (view_of(&self.env_textures[0]), view_of(&self.env_textures[1]), view_of(&self.env_textures[2]));
        let array_view = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        let (ca, ma, na) = (array_view(&arrays[0]), array_view(&arrays[1]), array_view(&arrays[2]));
        let (from, to) = (scene.cur * 3, (1 - scene.cur) * 3);
        let (prev, prev_albedo, prev_normal) = (view_of(&accum[from]), view_of(&accum[from + 1]), view_of(&accum[from + 2]));
        let (next, next_albedo, next_normal) = (view_of(&accum[to]), view_of(&accum[to + 1]), view_of(&accum[to + 2]));
        let group = bind(&self.device, &self.path_pipe, &[
            self.path_uniform.as_entire_binding(),
            self.env_uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(&e0),
            wgpu::BindingResource::TextureView(&e1),
            wgpu::BindingResource::TextureView(&e2),
            wgpu::BindingResource::Sampler(&self.sampler),
            scene.nodes.as_entire_binding(),
            scene.tris.as_entire_binding(),
            scene.attrs.as_entire_binding(),
            scene.roles.as_entire_binding(),
            wgpu::BindingResource::TextureView(&ca),
            wgpu::BindingResource::TextureView(&ma),
            wgpu::BindingResource::TextureView(&na),
            wgpu::BindingResource::TextureView(&prev),
            wgpu::BindingResource::TextureView(&prev_albedo),
            wgpu::BindingResource::TextureView(&prev_normal),
            self.env_cdf.as_entire_binding(),
        ]);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let clear = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&next, clear), attach(&next_albedo, clear), attach(&next_normal, clear)], ..Default::default() });
            pass.set_pipeline(&self.path_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        scene.cur = 1 - scene.cur;
        scene.samples += 1;
        let inflight = self.path_inflight.clone();
        inflight.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.queue.on_submitted_work_done(move || {
            inflight.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        });
    }

    pub fn path_read(&mut self) -> Result<js_sys::Promise, String> {
        let (width, height) = self.path.as_ref().and_then(|p| p.accum.as_ref()).map(|a| (a[0].width(), a[0].height())).ok_or("Nothing has been traced yet")?;
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("path render"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let row = (width * 4).next_multiple_of(256);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("path render readback"), size: (row * height) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        if !self.encode_path(&mut encoder, &target) {
            return Err("Nothing has been traced yet".into());
        }
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(height) } },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        let bgra = matches!(self.config.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
        Ok(read_rgba(buffer, width, height, row, bgra, false))
    }

    pub fn path_ready(&self) -> bool {
        self.path_inflight.load(std::sync::atomic::Ordering::Relaxed) < 2
    }

    fn encode_path(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::Texture) -> bool {
        let Some(scene) = self.path.as_ref() else { return false };
        let Some(accum) = scene.accum.as_ref() else { return false };
        if accum[0].width() != target.width() || accum[0].height() != target.height() {
            return false;
        }
        let (albedo, normal) = (view_of(&accum[scene.cur * 3 + 1]), view_of(&accum[scene.cur * 3 + 2]));
        let mut shown = view_of(&accum[scene.cur * 3]);
        let mut divisor = scene.samples as f32;
        if let (true, Some(filtered)) = (self.path_denoise && scene.samples > 0, scene.filtered.as_ref()) {
            let sigma = (2.0 / (scene.samples as f32).sqrt()).clamp(0.08, 1.5);
            for (pass_index, uniform) in self.path_filter_uniforms.iter().enumerate() {
                let input = if pass_index == 0 { view_of(&accum[scene.cur * 3]) } else { view_of(&filtered[(pass_index + 1) % 2]) };
                let output = view_of(&filtered[pass_index % 2]);
                let first = if pass_index == 0 { scene.samples as f32 } else { 1.0 };
                self.queue.write_buffer(uniform, 0, bytemuck::cast_slice(&[(1u32 << pass_index) as f32, first, 0.0, sigma * 0.7f32.powi(pass_index as i32)]));
                let group = bind(&self.device, &self.path_filter_pipe, &[
                    uniform.as_entire_binding(),
                    wgpu::BindingResource::TextureView(&input),
                    wgpu::BindingResource::TextureView(&albedo),
                    wgpu::BindingResource::TextureView(&normal),
                ]);
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&output, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))], ..Default::default() });
                pass.set_pipeline(&self.path_filter_pipe);
                pass.set_bind_group(0, &group, &[]);
                pass.draw(0..3, 0..1);
            }
            shown = view_of(&filtered[(self.path_filter_uniforms.len() - 1) % 2]);
            divisor = 1.0;
        }
        self.queue.write_buffer(&self.path_show_uniform, 0, bytemuck::cast_slice(&[divisor, 0.0f32, 0.0, 0.0, 0.075, 0.078, 0.086, 1.0]));
        self.queue.write_buffer(&self.env_uniform, 0, bytemuck::cast_slice(&self.env_params));
        let group = bind(&self.device, &self.path_show_pipe, &[
            self.path_show_uniform.as_entire_binding(),
            self.env_uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(&shown),
        ]);
        let post = self.post_params[19] > 0.5 && self.scene.width() == target.width() && self.scene.height() == target.height();
        let out = if post { view_of(&self.scene) } else { view_of(target) };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&out, wgpu::LoadOp::Clear(wgpu::Color::BLACK))], ..Default::default() });
            pass.set_pipeline(&self.path_show_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        if post {
            let mut params = self.post_params;
            params[4] = 0.0;
            self.encode_post(encoder, target, params);
        }
        true
    }

    pub fn trace_progress(&self) -> f32 {
        self.trace_job.as_ref().map(|(_, y)| *y as f32 / self.size as f32).unwrap_or(1.0)
    }

    pub fn set_pattern(&mut self, id: u32, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
        if width == 0 || height == 0 || width > 4096 || height > 4096 || rgba.len() != (width * height * 4) as usize {
            return Err("Fill images must be RGBA, 1 to 4096 pixels per side".into());
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fill pattern"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(texture.as_image_copy(), rgba, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) }, wgpu::Extent3d { width, height, depth_or_array_layers: 1 });
        self.patterns.insert(id, texture);
        Ok(())
    }

    pub fn set_tile_pattern(&mut self, id: u32, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
        let shared = self.patterns.remove(&id);
        let result = self.set_pattern(id, width, height, rgba);
        if let Some(texture) = self.patterns.remove(&id) {
            self.tile_patterns.insert((self.current, id), texture);
        }
        if let Some(texture) = shared {
            self.patterns.insert(id, texture);
        }
        result
    }

    fn take_set(&mut self) -> Set {
        while let Some(set) = self.pool.pop() {
            if set.color.width() == self.size {
                return set;
            }
        }
        new_set(&self.device, self.size)
    }

    fn copy_set(&self, encoder: &mut wgpu::CommandEncoder, from: &Set, to: &Set) {
        let extent = wgpu::Extent3d { width: self.size, height: self.size, depth_or_array_layers: 1 };
        for (a, b) in [(&from.color, &to.color), (&from.val, &to.val), (&from.cov, &to.cov), (&from.mask, &to.mask)] {
            encoder.copy_texture_to_texture(a.as_image_copy(), b.as_image_copy(), extent);
        }
    }

    pub fn insert_layer(&mut self, at: usize, id: u32) {
        let set = self.take_set();
        self.clear_set(&set);
        self.layers.insert(at, LayerTex { id, set, effects: Vec::new(), fx: None });
    }

    pub fn remove_layer(&mut self, index: usize, id: u32) {
        let layer = self.layers.remove(index);
        self.patterns.remove(&id);
        self.tile_patterns.remove(&(self.current, id));
        self.pool.push(layer.set);
        let (keep_u, drop_u): (Vec<_>, Vec<_>) = std::mem::take(&mut self.undo_stack).into_iter().partition(|s| s.layer != id);
        let (keep_r, drop_r): (Vec<_>, Vec<_>) = std::mem::take(&mut self.redo_stack).into_iter().partition(|s| s.layer != id);
        self.undo_stack = keep_u;
        self.redo_stack = keep_r;
        self.pool.extend(drop_u.into_iter().chain(drop_r).map(|s| s.set));
    }

    pub fn reorder(&mut self, ids: &[u32]) {
        self.layers.sort_by_key(|l| ids.iter().position(|id| *id == l.id).unwrap_or(usize::MAX));
    }

    fn build_group_mask(&self, masks: &[&wgpu::Texture]) {
        let target = view_of(&self.group_mask);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&target, wgpu::LoadOp::Clear(wgpu::Color::WHITE))], ..Default::default() });
        for mask in masks {
            let source = view_of(mask);
            let group = bind(&self.device, &self.mask_mul_pipe, &[wgpu::BindingResource::TextureView(&source)]);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&target, wgpu::LoadOp::Load)], ..Default::default() });
            pass.set_pipeline(&self.mask_mul_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
    }

    pub fn begin_stroke(&mut self, index: usize) {
        if let Some((_, snap)) = self.stroke.take() {
            self.pool.push(snap.set);
        }
        let set = self.take_set();
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.copy_set(&mut encoder, &self.layers[index].set, &set);
        {
            let view = view_of(&self.mask);
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&view, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))], ..Default::default() });
        }
        if self.quick_mode == 1 {
            let extent = wgpu::Extent3d { width: self.size, height: self.size, depth_or_array_layers: 1 };
            encoder.copy_texture_to_texture(self.quick.as_image_copy(), set.mask.as_image_copy(), extent);
        }
        self.queue.submit([encoder.finish()]);
        self.stroke = Some((index, Snapshot { layer: self.layers[index].id, set }));
    }

    pub fn mask_triangles(&self, indices: &[u32], triangles: &[u32]) {
        let view = view_of(&self.mask);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let selected: Vec<u32> = triangles.iter().flat_map(|&t| indices[t as usize * 3..t as usize * 3 + 3].iter().copied()).collect();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&view, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))], ..Default::default() });
        if let (Some(v), false) = (&self.vertices, selected.is_empty()) {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("polygon fill indices"), size: (selected.len() * 4) as u64, usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            self.queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&selected));
            pass.set_pipeline(&self.region_pipe);
            pass.set_vertex_buffer(0, v.slice(..));
            pass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..selected.len() as u32, 0, 0..1);
        }
        drop(pass);
        self.queue.submit([encoder.finish()]);
    }

    pub fn mask_cover(&self, width: u32, height: u32, rgba: &[u8], bounds: [f32; 4], uv: bool, cull: bool, view_proj: [f32; 16], eye: Vec3) -> Result<(), String> {
        if width == 0 || height == 0 || width > 8192 || height > 8192 || rgba.len() != (width * height * 4) as usize {
            return Err("A path coverage image must be RGBA, 1 to 8192 pixels per side".into());
        }
        let extent = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("path coverage"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(texture.as_image_copy(), rgba, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) }, extent);
        self.queue.write_buffer(&self.camera, 0, bytemuck::cast_slice(&camera_bytes(view_proj, eye, None, 1.0, 1.0, 0.0)));
        let params = [bounds[0], bounds[1], bounds[2], bounds[3], if uv { 1.0 } else { 0.0 }, if cull { 1.0 } else { 0.0 }, 0.0, 0.0];
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("path cover"), size: 32, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        self.queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&params));
        let depth_view = self.depth.create_view(&wgpu::TextureViewDescriptor { aspect: wgpu::TextureAspect::DepthOnly, ..Default::default() });
        let cover_view = view_of(&texture);
        let group = bind(&self.device, &self.cover_pipe, &[
            self.camera.as_entire_binding(),
            buffer.as_entire_binding(),
            wgpu::BindingResource::TextureView(&depth_view),
            wgpu::BindingResource::TextureView(&cover_view),
            wgpu::BindingResource::Sampler(&self.clamp_sampler),
        ]);
        let view = view_of(&self.mask);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&view, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))], ..Default::default() });
            pass.set_pipeline(&self.cover_pipe);
            pass.set_bind_group(0, &group, &[]);
            self.draw_mesh(&mut pass);
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    pub fn fill_mask(&self) {
        let view = view_of(&self.mask);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&view, wgpu::LoadOp::Clear(wgpu::Color::WHITE))], ..Default::default() });
        self.queue.submit([encoder.finish()]);
    }

    pub fn end_stroke(&mut self) {
        if self.quick_mode == 1 {
            if let Some((_, snap)) = self.stroke.take() {
                self.pool.push(snap.set);
            }
            return;
        }
        if let Some((_, snap)) = self.stroke.take() {
            self.undo_stack.push(snap);
            if self.undo_stack.len() > UNDO_LIMIT {
                let old = self.undo_stack.remove(0);
                self.pool.push(old.set);
            }
            let redo = std::mem::take(&mut self.redo_stack);
            self.pool.extend(redo.into_iter().map(|s| s.set));
        }
    }

    fn swap(&mut self, entry: &Snapshot, layer_ids: &[u32]) -> bool {
        let Some(index) = layer_ids.iter().position(|id| *id == entry.layer) else { return false };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.copy_set(&mut encoder, &self.layers[index].set, &self.scratch);
        self.copy_set(&mut encoder, &entry.set, &self.layers[index].set);
        self.copy_set(&mut encoder, &self.scratch, &entry.set);
        self.queue.submit([encoder.finish()]);
        true
    }

    pub fn undo(&mut self, layer_ids: &[u32]) -> bool {
        let Some(entry) = self.undo_stack.pop() else { return false };
        let ok = self.swap(&entry, layer_ids);
        self.redo_stack.push(entry);
        ok
    }

    pub fn redo(&mut self, layer_ids: &[u32]) -> bool {
        let Some(entry) = self.redo_stack.pop() else { return false };
        let ok = self.swap(&entry, layer_ids);
        self.undo_stack.push(entry);
        ok
    }

    pub fn set_environment(&mut self, width: u32, height: u32, rgb: &[f32]) -> Result<(), String> {
        let (w, h) = (width as usize, height as usize);
        if w < 8 || h < 4 || w > 16384 || h > 8192 || rgb.len() != w * h * 3 {
            return Err("An environment is an equirectangular RGB float image, 8 to 16384 pixels wide".into());
        }
        let sw = w.min(2048);
        let sh = (sw / 2).max(1);
        let sharp = shrink(rgb, w, h, sw, sh);
        let mid = blur_wrap(&shrink(&sharp, sw, sh, 128, 64), 128, 64, 2);
        let small = blur_wrap(&shrink(&mid, 128, 64, 32, 16), 32, 16, 4);
        let (cw, ch) = (256usize, 128usize);
        let grid = shrink(&sharp, sw, sh, cw, ch);
        let mut rows = vec![0.0f32; ch];
        let mut cond = vec![0.0f32; cw * ch];
        for y in 0..ch {
            let sin = (((y as f32) + 0.5) / ch as f32 * std::f32::consts::PI).sin();
            let mut run = 0.0f32;
            for x in 0..cw {
                let k = (y * cw + x) * 3;
                run += (0.2126 * grid[k] + 0.7152 * grid[k + 1] + 0.0722 * grid[k + 2]).max(0.0) * sin + 1e-6;
                cond[y * cw + x] = run;
            }
            rows[y] = run;
            for x in 0..cw {
                cond[y * cw + x] /= run;
            }
        }
        let total: f32 = rows.iter().sum();
        let mut acc = 0.0f32;
        for r in rows.iter_mut() {
            acc += *r;
            *r = acc / total;
        }
        let mut table = vec![cw as f32, ch as f32, total, 0.0];
        table.extend_from_slice(&rows);
        table.extend_from_slice(&cond);
        self.env_cdf = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("environment distribution"), size: (table.len() * 4) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        self.queue.write_buffer(&self.env_cdf, 0, bytemuck::cast_slice(&table));
        self.env_textures = [
            half_texture(&self.device, &self.queue, sw as u32, sh as u32, &sharp),
            half_texture(&self.device, &self.queue, 128, 64, &mid),
            half_texture(&self.device, &self.queue, 32, 16, &small),
        ];
        Ok(())
    }

    pub fn set_image(&mut self, stencil: bool, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
        if width == 0 || height == 0 || width > 4096 || height > 4096 || rgba.len() != (width * height * 4) as usize {
            return Err("Brush images must be RGBA, 1 to 4096 pixels per side".into());
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(if stencil { "stencil" } else { "brush alpha" }),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            texture.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: Some(height) },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        if stencil { self.stencil_tex = texture } else { self.alpha_tex = texture }
        Ok(())
    }

    pub fn paint(&mut self, stamps: &[[f32; 16]], view_proj: [f32; 16], eye: Vec3, brush: &BrushSettings, channels: &ChannelValues, index: usize, to_mask: bool) {
        let Some((stroke_index, _)) = self.stroke.as_ref() else { return };
        if *stroke_index != index || self.vertices.is_none() {
            return;
        }
        self.queue.write_buffer(&self.camera, 0, bytemuck::cast_slice(&camera_bytes(view_proj, eye, None, 1.0, 1.0, 0.0)));
        let depth_view = self.depth.create_view(&wgpu::TextureViewDescriptor { aspect: wgpu::TextureAspect::DepthOnly, ..Default::default() });
        let (alpha_view, stencil_view) = (view_of(&self.alpha_tex), view_of(&self.stencil_tex));
        let group = bind(&self.device, &self.paint_pipe, &[
            self.camera.as_entire_binding(),
            self.stamp_buffer.as_entire_binding(),
            wgpu::BindingResource::TextureView(&depth_view),
            wgpu::BindingResource::TextureView(&alpha_view),
            wgpu::BindingResource::TextureView(&stencil_view),
            wgpu::BindingResource::Sampler(&self.clamp_sampler),
        ]);
        let mask_view = view_of(&self.mask);
        for chunk in stamps.chunks(32) {
            let mut data: Vec<f32> = Vec::with_capacity(16 + 32 * 16);
            data.extend_from_slice(&[f32::from_bits(chunk.len() as u32), 0.0, 0.0, 0.0]);
            data.extend_from_slice(&[brush.hardness, brush.flow, if brush.backface_culling { 1.0 } else { 0.0 }, brush.alignment.code()]);
            data.extend_from_slice(&[brush.tip.code(), brush.spray_density, if brush.stencil { 1.0 } else { 0.0 }, 0.0]);
            data.extend_from_slice(&[brush.stencil_scale, brush.stencil_angle.to_radians(), brush.stencil_x, brush.stencil_y]);
            for s in chunk {
                data.extend_from_slice(s);
            }
            data.resize(16 + 32 * 16, 0.0);
            self.queue.write_buffer(&self.stamp_buffer, 0, bytemuck::cast_slice(&data));
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&mask_view, wgpu::LoadOp::Load)], ..Default::default() });
                pass.set_pipeline(&self.paint_pipe);
                pass.set_bind_group(0, &group, &[]);
                self.draw_mesh(&mut pass);
            }
            self.queue.submit([encoder.finish()]);
        }
        let quick_edit = self.quick_mode == 1;
        let to_mask = to_mask || quick_edit;
        let restrict = self.quick_mode == 2 && !quick_edit;
        if restrict {
            self.build_group_mask(&[&self.mask, &self.quick]);
        }
        let stroke_tex = if restrict { &self.group_mask } else { &self.mask };
        let erase = brush.mode == StrokeMode::Erase;
        let flag = |b: bool| if b { 1.0 } else { 0.0 };
        let c = channels.base_color;
        let mut paint = [
            c[0], c[1], c[2], 1.0,
            channels.roughness, channels.metallic, channels.height, 0.0,
            flag(channels.use_color), flag(channels.use_roughness), flag(channels.use_metallic), flag(channels.use_height),
            flag(erase), brush.opacity, 0.0, 0.0,
            channels.extra.clamp(0.0, 1.0), flag(channels.use_extra && self.extra_role > 0.5), 0.0, 0.0,
        ];
        if to_mask {
            paint[0] = if quick_edit { 1.0 } else { (0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]).max(0.0).powf(1.0 / 2.2) };
        }
        if let (Some([dx, dy]), StrokeMode::Clone, false) = (self.clone_offset, brush.mode, to_mask) {
            paint[3] = dx as f32;
            paint[7] = dy as f32;
            paint[14] = 1.0;
        }
        self.queue.write_buffer(&self.paint_uniform, 0, bytemuck::cast_slice(&paint));
        let (_, snap) = self.stroke.as_ref().unwrap();
        if to_mask {
            let (sm, mk) = (view_of(&snap.set.mask), view_of(stroke_tex));
            let group = bind(&self.device, &self.apply_mask_pipe, &[self.paint_uniform.as_entire_binding(), wgpu::BindingResource::TextureView(&sm), wgpu::BindingResource::TextureView(&mk)]);
            let target = view_of(if quick_edit { &self.quick } else { &self.layers[index].set.mask });
            let mut encoder = self.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&target, wgpu::LoadOp::Load)], ..Default::default() });
                pass.set_pipeline(&self.apply_mask_pipe);
                pass.set_bind_group(0, &group, &[]);
                pass.draw(0..3, 0..1);
            }
            self.queue.submit([encoder.finish()]);
            return;
        }
        let (sc, sv, sk, mk) = (view_of(&snap.set.color), view_of(&snap.set.val), view_of(&snap.set.cov), view_of(stroke_tex));
        let layer_source = self.clone_layer.filter(|&i| i < self.layers.len());
        let (cc, cm, ck) = match layer_source {
            Some(i) if i == index => (view_of(&snap.set.color), view_of(&snap.set.val), view_of(&snap.set.cov)),
            Some(i) => (view_of(&self.layers[i].set.color), view_of(&self.layers[i].set.val), view_of(&self.layers[i].set.cov)),
            None => (view_of(&self.final_set.color), view_of(&self.final_set.mat), view_of(&self.white)),
        };
        if layer_source.is_some() && paint[14] > 0.5 {
            paint[15] = 1.0;
            self.queue.write_buffer(&self.paint_uniform, 0, bytemuck::cast_slice(&paint));
        }
        let group = bind(&self.device, &self.apply_pipe, &[
            self.paint_uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(&sc),
            wgpu::BindingResource::TextureView(&sv),
            wgpu::BindingResource::TextureView(&sk),
            wgpu::BindingResource::TextureView(&mk),
            wgpu::BindingResource::TextureView(&cc),
            wgpu::BindingResource::TextureView(&cm),
            wgpu::BindingResource::TextureView(&ck),
        ]);
        let target = &self.layers[index].set;
        let (tc, tv, tk) = (view_of(&target.color), view_of(&target.val), view_of(&target.cov));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[attach(&tc, wgpu::LoadOp::Load), attach(&tv, wgpu::LoadOp::Load), attach(&tk, wgpu::LoadOp::Load)],
                ..Default::default()
            });
            pass.set_pipeline(&self.apply_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
    }

    fn layer_pass(&self, uniform: [f32; 24], prev: &Pair, out: &Pair, layer: &Set, mask: &wgpu::Texture) {
        self.queue.write_buffer(&self.layer_uniform, 0, bytemuck::cast_slice(&uniform));
        let (pc, pm, lc, lv, lk, lm) = (view_of(&prev.color), view_of(&prev.mat), view_of(&layer.color), view_of(&layer.val), view_of(&layer.cov), view_of(mask));
        let cover = view_of(&self.coverage);
        let group = bind(&self.device, &self.composite_pipe, &[
            self.layer_uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(&pc),
            wgpu::BindingResource::TextureView(&pm),
            wgpu::BindingResource::TextureView(&lc),
            wgpu::BindingResource::TextureView(&lv),
            wgpu::BindingResource::TextureView(&lk),
            wgpu::BindingResource::TextureView(&lm),
            wgpu::BindingResource::TextureView(&cover),
        ]);
        let (oc, om) = (view_of(&out.color), view_of(&out.mat));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&oc, wgpu::LoadOp::Load), attach(&om, wgpu::LoadOp::Load)], ..Default::default() });
            pass.set_pipeline(&self.composite_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
    }

    pub fn set_layer_effects(&mut self, index: usize, effects: Vec<Effect>) {
        let size = self.size;
        let Some(layer) = self.layers.get_mut(index) else { return };
        layer.fx = if effects.is_empty() { None } else { Some([texture(&self.device, size, LAYER_FORMAT, "mask effects"), texture(&self.device, size, LAYER_FORMAT, "mask effects")]) };
        layer.effects = effects;
    }

    pub fn layer_effects(&self, index: usize) -> Vec<fs_core::MaskEffect> {
        self.layers.get(index).map(|l| l.effects.iter().map(|e| e.spec).collect()).unwrap_or_default()
    }

    fn run_effects(&self, index: usize) -> Option<usize> {
        let layer = &self.layers[index];
        let fx = layer.fx.as_ref()?;
        if layer.effects.is_empty() {
            return None;
        }
        let extent = wgpu::Extent3d { width: self.size, height: self.size, depth_or_array_layers: 1 };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_texture(layer.set.mask.as_image_copy(), fx[0].as_image_copy(), extent);
        self.queue.submit([encoder.finish()]);
        let mut cur = 0;
        for effect in &layer.effects {
            if effect.params[0] > 8.0 {
                continue;
            }
            let passes: &[f32] = if effect.params[0] == 4.0 { &[0.0, 1.0] } else { &[0.0] };
            for &pass in passes {
                if let Some(fill) = effect.fill.as_ref() {
                    self.fill_target(layer.id, &self.scratch, fill);
                }
                let source = match effect.params[0] as u32 {
                    1 => Some(&self.scratch.cov),
                    5 => self.layers.iter().find(|l| l.id == effect.source && l.id != layer.id).map(|l| &l.set.mask),
                    _ => None,
                };
                let mut params = effect.params;
                if params[0] == 4.0 {
                    params[4] = pass;
                }
                if params[0] == 5.0 && source.is_none() {
                    continue;
                }
                self.queue.write_buffer(&self.fx_uniform, 0, bytemuck::cast_slice(&params));
                let (acc, src) = (view_of(&fx[cur]), view_of(source.unwrap_or(&fx[cur])));
                let group = bind(&self.device, &self.fx_pipe, &[self.fx_uniform.as_entire_binding(), wgpu::BindingResource::TextureView(&acc), wgpu::BindingResource::TextureView(&src)]);
                let target = view_of(&fx[1 - cur]);
                let mut encoder = self.device.create_command_encoder(&Default::default());
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&target, wgpu::LoadOp::Clear(wgpu::Color::BLACK))], ..Default::default() });
                    pass.set_pipeline(&self.fx_pipe);
                    pass.set_bind_group(0, &group, &[]);
                    pass.draw(0..3, 0..1);
                }
                self.queue.submit([encoder.finish()]);
                cur = 1 - cur;
            }
        }
        Some(cur)
    }

    fn effective_mask(&self, index: usize, results: &[Option<usize>]) -> &wgpu::Texture {
        match (results.get(index).copied().flatten(), self.layers[index].fx.as_ref()) {
            (Some(k), Some(fx)) => &fx[k],
            _ => &self.layers[index].set.mask,
        }
    }

    pub fn composite(&mut self, infos: &[LayerInfo], filters: &std::collections::HashMap<u32, FilterSettings>) {
        let results: Vec<Option<usize>> = (0..self.layers.len()).map(|i| if infos.get(i).map(|l| l.mask).unwrap_or(false) { self.run_effects(i) } else { None }).collect();
        let b = self.base;
        let extra_base = if self.extra_role > 1.5 && self.extra_role < 3.5 { 1.0 } else { 0.0 };
        let base_uniform = [0.0, 0.0, 0.0, 0.0, f32::from_bits(0), f32::from_bits(1), 0.0, 0.0, b[0], b[1], b[2], 1.0, b[3], b[4], 0.5, extra_base, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        self.layer_pass(base_uniform, &self.pong, &self.ping, &self.scratch, &self.scratch.mask);
        let mut current_is_ping = true;
        for (info, layer) in infos.iter().zip(self.layers.iter()) {
            if !info.visible || info.kind == LayerKind::Folder {
                continue;
            }
            let chain = fs_core::ancestors(infos, info.id);
            if chain.iter().any(|&a| !infos[a].visible) {
                continue;
            }
            let opacity = chain.iter().fold(info.opacity, |o, &a| o * infos[a].opacity);
            let index = self.layers.iter().position(|l| l.id == layer.id).unwrap_or(0);
            let mut masks: Vec<&wgpu::Texture> = chain.iter().filter(|&&a| infos[a].mask).map(|&a| self.effective_mask(a, &results)).collect();
            let grouped = !masks.is_empty();
            if grouped && info.mask {
                masks.push(self.effective_mask(index, &results));
            }
            if grouped {
                self.build_group_mask(&masks);
            }
            let mask_texture = if grouped { &self.group_mask } else { self.effective_mask(index, &results) };
            let f = filters.get(&info.id).copied().unwrap_or_default();
            let filter = if info.kind == LayerKind::Filter { 1 } else { 0 };
            let uniform = [
                opacity, if info.mask || grouped { 1.0 } else { 0.0 }, (f.blur.clamp(0.0, 32.0) * self.size as f32 / 1024.0), f.sharpen.clamp(0.0, 4.0),
                f32::from_bits(info.blend.code()), f32::from_bits(0), f32::from_bits(filter), 0.0,
                f.brightness, f.contrast, f.hue.to_radians(), f.saturation,
                f.gamma.max(0.05), f.roughness_offset, f.metallic_offset, 0.0,
                f.levels_in_low.clamp(0.0, 1.0), f.levels_in_high.clamp(0.0, 1.0).max(f.levels_in_low.clamp(0.0, 1.0) + 0.001), f.levels_out_low.clamp(0.0, 1.0), f.levels_out_high.clamp(0.0, 1.0),
                if f.posterize >= 2.0 { f.posterize.min(64.0).floor() } else { 0.0 }, (f.pixelate.clamp(0.0, 128.0) * self.size as f32 / 1024.0).floor(), if f.invert { 1.0 } else { 0.0 }, 0.0,
            ];
            let (prev, out) = if current_is_ping { (&self.ping, &self.pong) } else { (&self.pong, &self.ping) };
            self.layer_pass(uniform, prev, out, &layer.set, mask_texture);
            current_is_ping = !current_is_ping;
        }
        let src = if current_is_ping { &self.ping } else { &self.pong };
        let (sc, sm, cv) = (view_of(&src.color), view_of(&src.mat), view_of(&self.coverage));
        let group = bind(&self.device, &self.dilate_pipe, &[wgpu::BindingResource::TextureView(&sc), wgpu::BindingResource::TextureView(&sm), wgpu::BindingResource::TextureView(&cv)]);
        let (fc, fm) = (view_of(&self.final_set.color), view_of(&self.final_set.mat));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&fc, wgpu::LoadOp::Load), attach(&fm, wgpu::LoadOp::Load)], ..Default::default() });
            pass.set_pipeline(&self.dilate_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([encoder.finish()]);
    }

    pub fn render(&mut self, view_proj: [f32; 16], eye: Vec3, cursor: Option<(Vec3, Vec3, f32)>, exposure: f32, normal_strength: f32) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            _ => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.encode_view(&mut encoder, &frame.texture, view_proj, eye, cursor, exposure, normal_strength);
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
    }

    pub fn snapshot(&mut self, view_proj: [f32; 16], eye: Vec3, exposure: f32, normal_strength: f32) -> js_sys::Promise {
        let (width, height) = (self.config.width, self.config.height);
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("viewport snapshot"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let row = (width * 4).next_multiple_of(256);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("snapshot readback"), size: (row * height) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        self.encode_view(&mut encoder, &target, view_proj, eye, None, exposure, normal_strength);
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(height) } },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        let bgra = matches!(self.config.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
        read_rgba(buffer, width, height, row, bgra, false)
    }

    fn encode_view(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::Texture, view_proj: [f32; 16], eye: Vec3, cursor: Option<(Vec3, Vec3, f32)>, exposure: f32, normal_strength: f32) {
        if self.path_view && !self.flat && self.encode_path(encoder, target) {
            return;
        }
        let overlay = if self.quick_mode == 0 { 0.0 } else { 1.0 };
        let mut bytes = camera_bytes(view_proj, eye, cursor, exposure, normal_strength, overlay);
        if let Some(last) = bytes.last_mut() {
            *last = if self.flat { 1.0 } else { 0.0 };
        }
        if bytes.len() > 27 {
            bytes[27] = self.flat_channel as f32;
        }
        self.queue.write_buffer(&self.camera, 0, bytemuck::cast_slice(&bytes));
        self.queue.write_buffer(&self.env_uniform, 0, bytemuck::cast_slice(&self.env_params));
        let (e0, e1, e2) = (view_of(&self.env_textures[0]), view_of(&self.env_textures[1]), view_of(&self.env_textures[2]));
        let set_group = |final_set: &Pair, baked_normal: &wgpu::Texture, quick: &wgpu::Texture, role: f32| {
            let (fc, fm, bn, qm) = (view_of(&final_set.color), view_of(&final_set.mat), view_of(baked_normal), view_of(quick));
            let look = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("set look"), size: 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            self.queue.write_buffer(&look, 0, bytemuck::cast_slice(&[role, 0.0f32, 0.0, 0.0]));
            bind(&self.device, &self.view_pipe, &[
                self.camera.as_entire_binding(),
                wgpu::BindingResource::TextureView(&fc),
                wgpu::BindingResource::TextureView(&fm),
                wgpu::BindingResource::Sampler(&self.sampler),
                wgpu::BindingResource::TextureView(&bn),
                wgpu::BindingResource::TextureView(&qm),
                self.env_uniform.as_entire_binding(),
                wgpu::BindingResource::TextureView(&e0),
                wgpu::BindingResource::TextureView(&e1),
                wgpu::BindingResource::TextureView(&e2),
                look.as_entire_binding(),
            ])
        };
        let background = bind(&self.device, &self.background_pipe, &[
            self.env_uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(&e0),
            wgpu::BindingResource::TextureView(&e1),
            wgpu::BindingResource::TextureView(&e2),
            wgpu::BindingResource::Sampler(&self.sampler),
        ]);
        let mut draws = vec![(set_group(&self.final_set, &self.baked_normal, &self.quick, self.extra_role), self.range)];
        for state in self.parked.iter().flatten().filter(|_| !self.flat) {
            draws.push((set_group(&state.final_set, &state.baked_normal, &self.white, state.extra_role), state.range));
        }
        let post = self.post_params[19] > 0.5 && !self.flat;
        let view = if post { view_of(&self.scene) } else { view_of(target) };
        let depth = view_of(&self.depth);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[attach(&view, wgpu::LoadOp::Clear(wgpu::Color { r: 0.075, g: 0.078, b: 0.086, a: 1.0 }))],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.view_pipe);
            if let (Some(v), Some(i)) = (&self.vertices, &self.indices) {
                pass.set_vertex_buffer(0, v.slice(..));
                pass.set_index_buffer(i.slice(..), wgpu::IndexFormat::Uint32);
                for (group, (start, count)) in &draws {
                    pass.set_bind_group(0, group, &[]);
                    pass.draw_indexed(*start..*start + *count, 0, 0..1);
                }
            }
            if !self.flat && self.env_params[0] > 0.5 && self.env_params[3] > 0.5 {
                pass.set_pipeline(&self.background_pipe);
                pass.set_bind_group(0, &background, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        if !post {
            return;
        }
        self.encode_post(encoder, target, self.post_params);
    }

    fn encode_post(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::Texture, params: [f32; 24]) {
        self.queue.write_buffer(&self.post_uniform, 0, bytemuck::cast_slice(&params));
        let scene = view_of(&self.scene);
        let bloom = view_of(&self.bloom);
        let glow = params[0] > 0.0 || params[2] > 0.0 || params[3] > 0.0;
        {
            let group = bind(&self.device, &self.bloom_pipe, &[
                self.post_uniform.as_entire_binding(),
                wgpu::BindingResource::TextureView(&scene),
                wgpu::BindingResource::Sampler(&self.clamp_sampler),
            ]);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&bloom, wgpu::LoadOp::Clear(wgpu::Color::BLACK))], ..Default::default() });
            if glow {
                pass.set_pipeline(&self.bloom_pipe);
                pass.set_bind_group(0, &group, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        let depth_only = self.depth.create_view(&wgpu::TextureViewDescriptor { aspect: wgpu::TextureAspect::DepthOnly, ..Default::default() });
        let group = bind(&self.device, &self.post_pipe, &[
            self.post_uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(&scene),
            wgpu::BindingResource::TextureView(&bloom),
            wgpu::BindingResource::TextureView(&depth_only),
            wgpu::BindingResource::Sampler(&self.clamp_sampler),
        ]);
        let out = view_of(target);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&out, wgpu::LoadOp::Clear(wgpu::Color::BLACK))], ..Default::default() });
        pass.set_pipeline(&self.post_pipe);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }

    pub fn export(&mut self, channel: u32, normal_strength: f32, center: Vec3, radius: f32, wide: bool) -> js_sys::Promise {
        let size = self.size;
        let texel = if wide { 8 } else { 4 };
        let pipe = if wide { &self.export16_pipe } else { &self.export_pipe };
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("export"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if wide { LAYER_FORMAT } else { wgpu::TextureFormat::Rgba8Unorm },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let sel = [f32::from_bits(channel), radius, 0.0, 0.0, normal_strength, center[0], center[1], center[2]];
        self.queue.write_buffer(&self.export_uniform, 0, bytemuck::cast_slice(&sel));
        let (fc, fm) = (view_of(&self.final_set.color), view_of(&self.final_set.mat));
        let bake = view_of(&self.bake);
        let (pv, nv) = match &self.geometry {
            Some((p, n)) => (view_of(p), view_of(n)),
            None => (view_of(&self.bake), view_of(&self.bake)),
        };
        let group = bind(&self.device, pipe, &[
            self.export_uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(&fc),
            wgpu::BindingResource::TextureView(&fm),
            wgpu::BindingResource::TextureView(&bake),
            wgpu::BindingResource::TextureView(&pv),
            wgpu::BindingResource::TextureView(&nv),
            wgpu::BindingResource::TextureView(&view_of(&self.baked_normal)),
            wgpu::BindingResource::TextureView(&view_of(&self.baked_id)),
            wgpu::BindingResource::Sampler(&self.clamp_sampler),
        ]);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("readback"), size: (size * size * texel) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let tv = view_of(&target);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&tv, wgpu::LoadOp::Load)], ..Default::default() });
            pass.set_pipeline(pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size * texel), rows_per_image: Some(size) } },
            wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);
        js_sys::Promise::new(&mut |resolve, reject| {
            let readback = buffer.clone();
            buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
                if let Err(e) = result {
                    let _ = reject.call1(&JsValue::NULL, &JsValue::from_str(&format!("Texture readback failed: {e}")));
                    return;
                }
                let bytes = match readback.slice(..).get_mapped_range() {
                    Ok(view) => js_sys::Uint8Array::from(&view[..]),
                    Err(e) => {
                        let _ = reject.call1(&JsValue::NULL, &JsValue::from_str(&format!("Texture readback failed: {e}")));
                        return;
                    }
                };
                readback.unmap();
                let _ = resolve.call1(&JsValue::NULL, &bytes);
            });
        })
    }
}
