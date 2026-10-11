use crate::shaders;
use fs_core::{dot, BrushSettings, ChannelValues, LayerInfo, Mesh, StrokeMode, Vec3};
use wasm_bindgen::prelude::*;

const LAYER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const UNDO_LIMIT: usize = 24;

struct Set {
    color: wgpu::Texture,
    val: wgpu::Texture,
    cov: wgpu::Texture,
}

struct LayerTex {
    id: u32,
    set: Set,
}

struct Snapshot {
    layer: u32,
    set: Set,
}

struct Pair {
    color: wgpu::Texture,
    mat: wgpu::Texture,
}

pub struct Gpu {
    pub backend: String,
    pub size: u32,
    pub base: [f32; 5],
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
    paint_uniform: wgpu::Buffer,
    layer_uniform: wgpu::Buffer,
    export_uniform: wgpu::Buffer,
    view_pipe: wgpu::RenderPipeline,
    paint_pipe: wgpu::RenderPipeline,
    apply_pipe: wgpu::RenderPipeline,
    composite_pipe: wgpu::RenderPipeline,
    dilate_pipe: wgpu::RenderPipeline,
    coverage_pipe: wgpu::RenderPipeline,
    export_pipe: wgpu::RenderPipeline,
    vertices: Option<wgpu::Buffer>,
    indices: Option<wgpu::Buffer>,
    index_count: u32,
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
    }
}

fn new_pair(device: &wgpu::Device, size: u32) -> Pair {
    Pair { color: texture(device, size, LAYER_FORMAT, "composite color"), mat: texture(device, size, LAYER_FORMAT, "composite material") }
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

fn read_rgba(buffer: wgpu::Buffer, width: u32, height: u32, row: u32, bgra: bool) -> js_sys::Promise {
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
                            if bgra { out.extend_from_slice(&[px[2], px[1], px[0], 255]) } else { out.extend_from_slice(&[px[0], px[1], px[2], 255]) }
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

fn camera_bytes(view_proj: [f32; 16], eye: Vec3, cursor: Option<(Vec3, Vec3, f32)>, exposure: f32, normal_strength: f32) -> Vec<f32> {
    let mut data = view_proj.to_vec();
    data.extend_from_slice(&[eye[0], eye[1], eye[2], 1.0]);
    match cursor {
        Some((p, n, r)) => {
            data.extend_from_slice(&[p[0], p[1], p[2], r]);
            data.extend_from_slice(&[n[0], n[1], n[2], 0.0]);
        }
        None => data.extend_from_slice(&[0.0; 8]),
    }
    data.extend_from_slice(&[exposure, normal_strength, 0.0, 0.0]);
    data
}

impl Gpu {
    pub async fn new(canvas: web_sys::HtmlCanvasElement, width: u32, height: u32, size: u32) -> Result<Gpu, String> {
        let size = size.clamp(256, 4096).next_power_of_two();
        let instance = wgpu::util::new_instance_with_webgpu_detection(wgpu::InstanceDescriptor::new_without_display_handle()).await;
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
            base: [0.5, 0.5, 0.5, 0.6, 0.0],
            view_pipe: pipeline(&device, "view", shaders::VIEW, true, &[format], None, true),
            paint_pipe: pipeline(&device, "paint", shaders::PAINT, true, &[LAYER_FORMAT], Some(union), false),
            apply_pipe: pipeline(&device, "apply", shaders::APPLY, false, &[LAYER_FORMAT; 3], None, false),
            composite_pipe: pipeline(&device, "composite", shaders::COMPOSITE, false, &[LAYER_FORMAT; 2], None, false),
            dilate_pipe: pipeline(&device, "dilate", shaders::DILATE, false, &[LAYER_FORMAT; 2], None, false),
            coverage_pipe: pipeline(&device, "coverage", shaders::COVERAGE, true, &[wgpu::TextureFormat::R8Unorm], None, false),
            export_pipe: pipeline(&device, "export", shaders::EXPORT, false, &[wgpu::TextureFormat::Rgba8Unorm], None, false),
            depth: depth_texture(&device, width, height),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                ..Default::default()
            }),
            camera: uniform("camera", 128),
            stamp_buffer: uniform("stamps", 32 + 32 * 32),
            paint_uniform: uniform("paint", 64),
            layer_uniform: uniform("layer", 64),
            export_uniform: uniform("export", 32),
            vertices: None,
            indices: None,
            index_count: 0,
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
    }

    pub fn upload_mesh(&mut self, mesh: &Mesh) {
        let data = mesh.interleaved();
        let upload = |label: &str, bytes: &[u8], usage: wgpu::BufferUsages| {
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: bytes.len().next_multiple_of(4) as u64, usage: usage | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            self.queue.write_buffer(&buffer, 0, bytes);
            buffer
        };
        self.vertices = Some(upload("vertices", bytemuck::cast_slice(&data), wgpu::BufferUsages::VERTEX));
        self.indices = Some(upload("indices", bytemuck::cast_slice(&mesh.indices), wgpu::BufferUsages::INDEX));
        self.index_count = mesh.indices.len() as u32;
        let view = view_of(&self.coverage);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&view, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))], ..Default::default() });
            pass.set_pipeline(&self.coverage_pipe);
            self.draw_mesh(&mut pass);
        }
        self.queue.submit([encoder.finish()]);
    }

    fn draw_mesh(&self, pass: &mut wgpu::RenderPass) {
        if let (Some(v), Some(i)) = (&self.vertices, &self.indices) {
            pass.set_vertex_buffer(0, v.slice(..));
            pass.set_index_buffer(i.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.index_count, 0, 0..1);
        }
    }

    fn clear_set(&self, set: &Set) {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        for t in [&set.color, &set.val, &set.cov] {
            let view = view_of(t);
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&view, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT))], ..Default::default() });
        }
        self.queue.submit([encoder.finish()]);
    }

    fn take_set(&mut self) -> Set {
        self.pool.pop().unwrap_or_else(|| new_set(&self.device, self.size))
    }

    fn copy_set(&self, encoder: &mut wgpu::CommandEncoder, from: &Set, to: &Set) {
        let extent = wgpu::Extent3d { width: self.size, height: self.size, depth_or_array_layers: 1 };
        for (a, b) in [(&from.color, &to.color), (&from.val, &to.val), (&from.cov, &to.cov)] {
            encoder.copy_texture_to_texture(a.as_image_copy(), b.as_image_copy(), extent);
        }
    }

    pub fn insert_layer(&mut self, at: usize, id: u32) {
        let set = self.take_set();
        self.clear_set(&set);
        self.layers.insert(at, LayerTex { id, set });
    }

    pub fn remove_layer(&mut self, index: usize, id: u32) {
        let layer = self.layers.remove(index);
        self.pool.push(layer.set);
        let (keep_u, drop_u): (Vec<_>, Vec<_>) = std::mem::take(&mut self.undo_stack).into_iter().partition(|s| s.layer != id);
        let (keep_r, drop_r): (Vec<_>, Vec<_>) = std::mem::take(&mut self.redo_stack).into_iter().partition(|s| s.layer != id);
        self.undo_stack = keep_u;
        self.redo_stack = keep_r;
        self.pool.extend(drop_u.into_iter().chain(drop_r).map(|s| s.set));
    }

    pub fn move_layer(&mut self, from: usize, to: usize) {
        let layer = self.layers.remove(from);
        self.layers.insert(to, layer);
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
        self.queue.submit([encoder.finish()]);
        self.stroke = Some((index, Snapshot { layer: self.layers[index].id, set }));
    }

    pub fn fill_mask(&self) {
        let view = view_of(&self.mask);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&view, wgpu::LoadOp::Clear(wgpu::Color::WHITE))], ..Default::default() });
        self.queue.submit([encoder.finish()]);
    }

    pub fn end_stroke(&mut self) {
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

    pub fn paint(&mut self, stamps: &[[f32; 8]], view_proj: [f32; 16], eye: Vec3, brush: &BrushSettings, channels: &ChannelValues, index: usize) {
        let Some((stroke_index, _)) = self.stroke.as_ref() else { return };
        if *stroke_index != index || self.vertices.is_none() {
            return;
        }
        self.queue.write_buffer(&self.camera, 0, bytemuck::cast_slice(&camera_bytes(view_proj, eye, None, 1.0, 1.0)));
        let depth_view = self.depth.create_view(&wgpu::TextureViewDescriptor { aspect: wgpu::TextureAspect::DepthOnly, ..Default::default() });
        let group = bind(&self.device, &self.paint_pipe, &[self.camera.as_entire_binding(), self.stamp_buffer.as_entire_binding(), wgpu::BindingResource::TextureView(&depth_view)]);
        let mask_view = view_of(&self.mask);
        for chunk in stamps.chunks(32) {
            let mut data: Vec<f32> = Vec::with_capacity(8 + 32 * 8);
            data.extend_from_slice(&[f32::from_bits(chunk.len() as u32), 0.0, 0.0, 0.0]);
            data.extend_from_slice(&[brush.hardness, brush.flow, if brush.backface_culling { 1.0 } else { 0.0 }, 1.0]);
            for s in chunk {
                data.extend_from_slice(s);
            }
            data.resize(8 + 32 * 8, 0.0);
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
        let erase = brush.mode == StrokeMode::Erase;
        let flag = |b: bool| if b { 1.0 } else { 0.0 };
        let c = channels.base_color;
        let paint = [
            c[0], c[1], c[2], 1.0,
            channels.roughness, channels.metallic, channels.height, 0.0,
            flag(channels.use_color), flag(channels.use_roughness), flag(channels.use_metallic), flag(channels.use_height),
            flag(erase), brush.opacity, 0.0, 0.0,
        ];
        self.queue.write_buffer(&self.paint_uniform, 0, bytemuck::cast_slice(&paint));
        let (_, snap) = self.stroke.as_ref().unwrap();
        let (sc, sv, sk, mk) = (view_of(&snap.set.color), view_of(&snap.set.val), view_of(&snap.set.cov), view_of(&self.mask));
        let group = bind(&self.device, &self.apply_pipe, &[
            self.paint_uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(&sc),
            wgpu::BindingResource::TextureView(&sv),
            wgpu::BindingResource::TextureView(&sk),
            wgpu::BindingResource::TextureView(&mk),
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

    fn layer_pass(&self, uniform: [f32; 16], prev: &Pair, out: &Pair, layer: &Set) {
        self.queue.write_buffer(&self.layer_uniform, 0, bytemuck::cast_slice(&uniform));
        let (pc, pm, lc, lv, lk) = (view_of(&prev.color), view_of(&prev.mat), view_of(&layer.color), view_of(&layer.val), view_of(&layer.cov));
        let group = bind(&self.device, &self.composite_pipe, &[
            self.layer_uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(&pc),
            wgpu::BindingResource::TextureView(&pm),
            wgpu::BindingResource::TextureView(&lc),
            wgpu::BindingResource::TextureView(&lv),
            wgpu::BindingResource::TextureView(&lk),
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

    pub fn composite(&mut self, infos: &[LayerInfo]) {
        let b = self.base;
        let base_uniform = [0.0, 0.0, 0.0, 0.0, f32::from_bits(0), f32::from_bits(1), 0.0, 0.0, b[0], b[1], b[2], 1.0, b[3], b[4], 0.5, 0.0];
        self.layer_pass(base_uniform, &self.pong, &self.ping, &self.scratch);
        let mut current_is_ping = true;
        for (info, layer) in infos.iter().zip(self.layers.iter()) {
            if !info.visible {
                continue;
            }
            let uniform = [info.opacity, 0.0, 0.0, 0.0, f32::from_bits(info.blend.code()), f32::from_bits(0), 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
            let (prev, out) = if current_is_ping { (&self.ping, &self.pong) } else { (&self.pong, &self.ping) };
            self.layer_pass(uniform, prev, out, &layer.set);
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
        read_rgba(buffer, width, height, row, bgra)
    }

    fn encode_view(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::Texture, view_proj: [f32; 16], eye: Vec3, cursor: Option<(Vec3, Vec3, f32)>, exposure: f32, normal_strength: f32) {
        self.queue.write_buffer(&self.camera, 0, bytemuck::cast_slice(&camera_bytes(view_proj, eye, cursor, exposure, normal_strength)));
        let (fc, fm) = (view_of(&self.final_set.color), view_of(&self.final_set.mat));
        let group = bind(&self.device, &self.view_pipe, &[self.camera.as_entire_binding(), wgpu::BindingResource::TextureView(&fc), wgpu::BindingResource::TextureView(&fm), wgpu::BindingResource::Sampler(&self.sampler)]);
        let view = view_of(target);
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
            pass.set_bind_group(0, &group, &[]);
            self.draw_mesh(&mut pass);
        }
    }

    pub fn export(&mut self, channel: u32, normal_strength: f32) -> js_sys::Promise {
        let size = self.size;
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("export"),
            size: wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let sel = [f32::from_bits(channel), 0.0, 0.0, 0.0, normal_strength, 0.0, 0.0, 0.0];
        self.queue.write_buffer(&self.export_uniform, 0, bytemuck::cast_slice(&sel));
        let (fc, fm) = (view_of(&self.final_set.color), view_of(&self.final_set.mat));
        let group = bind(&self.device, &self.export_pipe, &[self.export_uniform.as_entire_binding(), wgpu::BindingResource::TextureView(&fc), wgpu::BindingResource::TextureView(&fm)]);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("readback"), size: (size * size * 4) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let tv = view_of(&target);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { color_attachments: &[attach(&tv, wgpu::LoadOp::Load)], ..Default::default() });
            pass.set_pipeline(&self.export_pipe);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size * 4), rows_per_image: Some(size) } },
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
