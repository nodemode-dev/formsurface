mod gpu;
mod shaders;

use fs_core::{add, cross, dot, normalize, scale, sub, BlendMode, BrushSettings, Bvh, ChannelValues, Hit, LayerInfo, Mesh, Vec3};
use gpu::Gpu;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Engine {
    gpu: Gpu,
    mesh: Option<(Mesh, Bvh)>,
    yaw: f32,
    pitch: f32,
    distance: f32,
    target: Vec3,
    fov: f32,
    width: u32,
    height: u32,
    brush: BrushSettings,
    channels: ChannelValues,
    layers: Vec<LayerInfo>,
    active: u32,
    next_id: u32,
    last_stamp: Option<Hit>,
    pending: Vec<[f32; 8]>,
    stroking: bool,
    cursor: Option<Hit>,
    dirty: bool,
    exposure: f32,
    normal_strength: f32,
}

#[wasm_bindgen]
pub async fn create_engine(canvas: web_sys::HtmlCanvasElement, texture_size: u32) -> Result<Engine, JsValue> {
    std::panic::set_hook(Box::new(|info| web_sys::console::error_1(&JsValue::from_str(&format!("FormSurface engine panic: {info}")))));
    let width = canvas.width().max(1);
    let height = canvas.height().max(1);
    let gpu = Gpu::new(canvas, width, height, texture_size).await.map_err(|e| JsValue::from_str(&e))?;
    let mut engine = Engine {
        gpu,
        mesh: None,
        yaw: 0.6,
        pitch: 0.25,
        distance: 3.0,
        target: [0.0; 3],
        fov: 0.75,
        width,
        height,
        brush: BrushSettings::default(),
        channels: ChannelValues::default(),
        layers: Vec::new(),
        active: 0,
        next_id: 1,
        last_stamp: None,
        pending: Vec::new(),
        stroking: false,
        cursor: None,
        dirty: true,
        exposure: 1.0,
        normal_strength: 1.0,
    };
    engine.add_layer("Layer 1".into());
    Ok(engine)
}

#[wasm_bindgen]
impl Engine {
    pub fn backend(&self) -> String {
        self.gpu.backend.clone()
    }

    pub fn texture_size(&self) -> u32 {
        self.gpu.size
    }

    pub fn set_mesh(&mut self, positions: &[f32], normals: &[f32], uvs: &[f32], indices: &[u32]) -> Result<String, JsValue> {
        let mesh = Mesh::from_flat(positions, normals, uvs, indices).map_err(|e| JsValue::from_str(&e))?;
        let bvh = Bvh::build(&mesh);
        self.gpu.upload_mesh(&mesh);
        self.target = mesh.center;
        self.distance = mesh.radius * 2.8;
        let info = format!("{} vertices, {} triangles", mesh.positions.len(), mesh.triangle_count());
        self.mesh = Some((mesh, bvh));
        self.dirty = true;
        Ok(info)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.gpu.resize(self.width, self.height);
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.008;
        self.pitch = (self.pitch + dy * 0.008).clamp(-1.5, 1.5);
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        let (_, right, up, _) = self.basis();
        let k = self.distance * 0.0015;
        self.target = add(self.target, add(scale(right, -dx * k), scale(up, dy * k)));
    }

    pub fn zoom(&mut self, delta: f32) {
        let r = self.mesh.as_ref().map(|m| m.0.radius).unwrap_or(1.0);
        self.distance = (self.distance * (1.0 + delta * 0.001)).clamp(r * 0.05, r * 50.0);
    }

    pub fn frame_mesh(&mut self) {
        if let Some((mesh, _)) = &self.mesh {
            self.target = mesh.center;
            self.distance = mesh.radius * 2.8;
        }
    }

    pub fn set_brush(&mut self, json: &str) -> Result<(), JsValue> {
        self.brush = serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(())
    }

    pub fn set_channels(&mut self, json: &str) -> Result<(), JsValue> {
        self.channels = serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        Ok(())
    }

    pub fn set_view(&mut self, exposure: f32, normal_strength: f32) {
        self.exposure = exposure;
        self.normal_strength = normal_strength;
    }

    pub fn set_base(&mut self, r: f32, g: f32, b: f32, roughness: f32, metallic: f32) {
        self.gpu.base = [r, g, b, roughness, metallic];
        self.dirty = true;
    }

    pub fn hover(&mut self, x: f32, y: f32) -> bool {
        self.cursor = self.pick(x, y);
        self.cursor.is_some()
    }

    pub fn clear_hover(&mut self) {
        self.cursor = None;
    }

    pub fn stroke_begin(&mut self, x: f32, y: f32, pressure: f32) -> bool {
        if self.mesh.is_none() {
            return false;
        }
        let index = self.layer_index(self.active);
        self.gpu.begin_stroke(index);
        self.stroking = true;
        self.last_stamp = self.pick(x, y);
        if let Some(hit) = self.last_stamp {
            self.push_stamp(&hit, pressure);
        }
        true
    }

    pub fn stroke_to(&mut self, x: f32, y: f32, pressure: f32) {
        if !self.stroking {
            return;
        }
        let hit = self.pick(x, y);
        self.cursor = hit;
        let Some(hit) = hit else {
            self.last_stamp = None;
            return;
        };
        let Some(last) = self.last_stamp else {
            self.last_stamp = Some(hit);
            self.push_stamp(&hit, pressure);
            return;
        };
        let radius = self.world_radius(&hit, pressure);
        let step = (radius * 2.0 * self.brush.spacing).max(1e-6);
        let span = fs_core::length(sub(hit.position, last.position));
        if span < step {
            return;
        }
        let steps = ((span / step).floor() as usize).min(512);
        for k in 1..=steps {
            let t = (k as f32 * step) / span;
            let p = add(last.position, scale(sub(hit.position, last.position), t));
            let n = normalize(add(scale(last.normal, 1.0 - t), scale(hit.normal, t)));
            let stamp = Hit { position: p, normal: n, t: last.t + (hit.t - last.t) * t, ..hit };
            self.push_stamp(&stamp, pressure);
            self.last_stamp = Some(stamp);
        }
    }

    pub fn stroke_end(&mut self) {
        if self.stroking {
            self.flush();
            self.gpu.end_stroke();
        }
        self.stroking = false;
        self.last_stamp = None;
    }

    pub fn fill_layer(&mut self) -> bool {
        if self.mesh.is_none() || self.stroking {
            return false;
        }
        let index = self.layer_index(self.active);
        self.gpu.begin_stroke(index);
        self.gpu.fill_mask();
        let (forward, right, up, eye) = self.basis();
        let view_proj = gpu::view_projection(eye, forward, right, up, self.fov, 1.0, 0.1, 10.0);
        self.gpu.paint(&[], view_proj, eye, &self.brush, &self.channels, index);
        self.gpu.end_stroke();
        self.dirty = true;
        true
    }

    pub fn set_orbit(&mut self, yaw: f32, pitch: f32, zoom: f32) {
        self.yaw = yaw;
        self.pitch = pitch.clamp(-1.5, 1.5);
        if let Some((mesh, _)) = &self.mesh {
            self.target = mesh.center;
            self.distance = mesh.radius * 2.8 * zoom.clamp(0.1, 10.0);
        }
    }

    pub fn camera_json(&self) -> String {
        serde_json::json!({ "yaw": self.yaw, "pitch": self.pitch, "distance": self.distance, "fov": self.fov }).to_string()
    }

    pub fn undo(&mut self) -> bool {
        let ok = self.gpu.undo(&self.layer_ids());
        self.dirty |= ok;
        ok
    }

    pub fn redo(&mut self) -> bool {
        let ok = self.gpu.redo(&self.layer_ids());
        self.dirty |= ok;
        ok
    }

    pub fn add_layer(&mut self, name: String) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let at = if self.layers.is_empty() { 0 } else { self.layer_index(self.active) + 1 };
        self.layers.insert(at, LayerInfo { id, name, visible: true, opacity: 1.0, blend: BlendMode::Normal });
        self.gpu.insert_layer(at, id);
        self.active = id;
        self.dirty = true;
        id
    }

    pub fn remove_layer(&mut self, id: u32) -> bool {
        if self.layers.len() <= 1 {
            return false;
        }
        let index = self.layer_index(id);
        self.layers.remove(index);
        self.gpu.remove_layer(index, id);
        if self.active == id {
            self.active = self.layers[index.min(self.layers.len() - 1)].id;
        }
        self.dirty = true;
        true
    }

    pub fn move_layer(&mut self, id: u32, offset: i32) {
        let index = self.layer_index(id);
        let to = (index as i32 + offset).clamp(0, self.layers.len() as i32 - 1) as usize;
        if to != index {
            let layer = self.layers.remove(index);
            self.layers.insert(to, layer);
            self.gpu.move_layer(index, to);
            self.dirty = true;
        }
    }

    pub fn update_layer(&mut self, json: &str) -> Result<(), JsValue> {
        let info: LayerInfo = serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let index = self.layer_index(info.id);
        self.layers[index] = info;
        self.dirty = true;
        Ok(())
    }

    pub fn select_layer(&mut self, id: u32) {
        self.active = id;
    }

    pub fn active_layer(&self) -> u32 {
        self.active
    }

    pub fn layers_json(&self) -> String {
        serde_json::to_string(&self.layers).unwrap_or_else(|_| "[]".into())
    }

    pub fn frame(&mut self) {
        self.flush();
        if self.dirty {
            self.gpu.composite(&self.layers);
            self.dirty = false;
        }
        let (forward, right, up, eye) = self.basis();
        let aspect = self.width as f32 / self.height as f32;
        let r = self.mesh.as_ref().map(|m| m.0.radius).unwrap_or(1.0);
        let view_proj = gpu::view_projection(eye, forward, right, up, self.fov, aspect, (self.distance - r * 2.0).max(r * 0.01), self.distance + r * 3.0);
        let cursor = self.cursor.map(|h| (h.position, h.normal, self.world_radius(&h, 1.0)));
        self.gpu.render(view_proj, eye, cursor, self.exposure, self.normal_strength);
    }

    pub fn export_channel(&mut self, channel: u32) -> js_sys::Promise {
        self.flush();
        if self.dirty {
            self.gpu.composite(&self.layers);
            self.dirty = false;
        }
        self.gpu.export(channel, self.normal_strength)
    }

    pub fn snapshot(&mut self) -> js_sys::Promise {
        let (forward, right, up, eye) = self.basis();
        let aspect = self.width as f32 / self.height as f32;
        let r = self.mesh.as_ref().map(|m| m.0.radius).unwrap_or(1.0);
        let view_proj = gpu::view_projection(eye, forward, right, up, self.fov, aspect, (self.distance - r * 2.0).max(r * 0.01), self.distance + r * 3.0);
        self.gpu.snapshot(view_proj, eye, self.exposure, self.normal_strength)
    }

    pub fn viewport_size(&self) -> Vec<u32> {
        vec![self.width, self.height]
    }
}

impl Engine {
    fn basis(&self) -> (Vec3, Vec3, Vec3, Vec3) {
        let forward = normalize([-self.yaw.sin() * self.pitch.cos(), -self.pitch.sin(), -self.yaw.cos() * self.pitch.cos()]);
        let right = normalize(cross(forward, [0.0, 1.0, 0.0]));
        let up = cross(right, forward);
        let eye = sub(self.target, scale(forward, self.distance));
        (forward, right, up, eye)
    }

    fn pick(&self, x: f32, y: f32) -> Option<Hit> {
        let (mesh, bvh) = self.mesh.as_ref()?;
        let (forward, right, up, eye) = self.basis();
        let aspect = self.width as f32 / self.height as f32;
        let t = (self.fov * 0.5).tan();
        let nx = (x / self.width as f32) * 2.0 - 1.0;
        let ny = 1.0 - (y / self.height as f32) * 2.0;
        let dir = normalize(add(forward, add(scale(right, nx * t * aspect), scale(up, ny * t))));
        let hit = bvh.raycast(mesh, eye, dir)?;
        let n = if dot(hit.normal, dir) > 0.0 { scale(hit.normal, -1.0) } else { hit.normal };
        Some(Hit { normal: n, ..hit })
    }

    fn world_radius(&self, hit: &Hit, pressure: f32) -> f32 {
        let p = if self.brush.pressure_size { pressure.clamp(0.05, 1.0) } else { 1.0 };
        let world_per_px = 2.0 * hit.t * (self.fov * 0.5).tan() / self.height as f32;
        self.brush.size_px * 0.5 * world_per_px * p
    }

    fn push_stamp(&mut self, hit: &Hit, pressure: f32) {
        let r = self.world_radius(hit, pressure);
        let p = hit.position;
        let n = hit.normal;
        self.pending.push([p[0], p[1], p[2], r, n[0], n[1], n[2], 0.0]);
        if self.brush.mirror_x {
            if let Some((mesh, _)) = &self.mesh {
                let cx = mesh.center[0];
                self.pending.push([2.0 * cx - p[0], p[1], p[2], r, -n[0], n[1], n[2], 1.0]);
            }
        }
        if self.pending.len() >= 256 {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.pending.is_empty() || !self.stroking {
            self.pending.clear();
            return;
        }
        let (forward, right, up, eye) = self.basis();
        let aspect = self.width as f32 / self.height as f32;
        let r = self.mesh.as_ref().map(|m| m.0.radius).unwrap_or(1.0);
        let view_proj = gpu::view_projection(eye, forward, right, up, self.fov, aspect, (self.distance - r * 2.0).max(r * 0.01), self.distance + r * 3.0);
        let stamps = std::mem::take(&mut self.pending);
        self.gpu.paint(&stamps, view_proj, eye, &self.brush, &self.channels, self.layer_index(self.active));
        self.dirty = true;
    }

    fn layer_index(&self, id: u32) -> usize {
        self.layers.iter().position(|l| l.id == id).unwrap_or(0)
    }

    fn layer_ids(&self) -> Vec<u32> {
        self.layers.iter().map(|l| l.id).collect()
    }
}
