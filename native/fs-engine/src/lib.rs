mod gpu;
mod shaders;

use fs_core::{add, cross, dot, normalize, scale, sub, BlendMode, BrushSettings, Bvh, ChannelValues, FillSettings, FilterSettings, Hit, LayerInfo, LayerKind, Mesh, SizeSpace, Symmetry, Vec3};
use fs_core::sets::TextureSet;
use gpu::Gpu;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

struct Parked {
    layers: Vec<LayerInfo>,
    active: u32,
    fills: HashMap<u32, FillSettings>,
    filters: HashMap<u32, FilterSettings>,
}

struct SetEntry {
    info: TextureSet,
    parked: Option<Parked>,
}

#[wasm_bindgen]
pub struct Engine {
    gpu: Gpu,
    sets: Vec<SetEntry>,
    set_index: usize,
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
    pending: Vec<[f32; 16]>,
    seed: u64,
    lazy: Option<(f32, f32)>,
    fills: std::collections::HashMap<u32, FillSettings>,
    filters: std::collections::HashMap<u32, FilterSettings>,
    high: Option<(Mesh, Bvh, Vec<u32>)>,
    mask_target: bool,
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
        sets: Vec::new(),
        set_index: 0,
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
        seed: 0x9e37_79b9_7f4a_7c15,
        lazy: None,
        fills: std::collections::HashMap::new(),
        filters: std::collections::HashMap::new(),
        high: None,
        mask_target: false,
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

    pub fn set_mesh(&mut self, positions: &[f32], normals: &[f32], uvs: &[f32], indices: &[u32], materials: &[u32], names_json: &str) -> Result<String, JsValue> {
        if self.stroking {
            return Err(JsValue::from_str("Finish the stroke before loading a mesh"));
        }
        let names: Vec<String> = serde_json::from_str(names_json).unwrap_or_default();
        let split = fs_core::sets::split(positions, normals, uvs, indices, materials, &names).map_err(|e| JsValue::from_str(&e))?;
        let mesh = Mesh::from_flat(&split.positions, &split.normals, &split.uvs, &split.indices).map_err(|e| JsValue::from_str(&e))?;
        let bvh = Bvh::build(&mesh);
        self.select_texture_set(0);
        let ranges: Vec<(u32, u32)> = split.sets.iter().map(|s| (s.start, s.count)).collect();
        self.gpu.upload_mesh(&mesh, &ranges);
        self.sets = split.sets.into_iter().map(|info| SetEntry { info, parked: None }).collect();
        self.set_index = 0;
        self.target = mesh.center;
        self.distance = mesh.radius * 2.8;
        let count = self.sets.len();
        let info = format!("{} vertices, {} triangles, {} texture set{}", mesh.positions.len(), mesh.triangle_count(), count, if count == 1 { "" } else { "s" });
        self.mesh = Some((mesh, bvh));
        self.each_set(|engine| {
            engine.render_fills();
            Ok(())
        })
        .map_err(|e| JsValue::from_str(&e))?;
        Ok(info)
    }

    pub fn texture_sets_json(&self) -> String {
        let list: Vec<serde_json::Value> = self
            .sets
            .iter()
            .enumerate()
            .map(|(k, s)| {
                let layers = if k == self.set_index { self.layers.len() } else { s.parked.as_ref().map(|p| p.layers.len()).unwrap_or(1) };
                serde_json::json!({ "index": k, "name": s.info.name(), "material": s.info.material, "udim": s.info.udim, "tiled": s.info.tiled, "triangles": s.info.count / 3, "layers": layers })
            })
            .collect();
        serde_json::Value::Array(list).to_string()
    }

    pub fn active_texture_set(&self) -> u32 {
        self.set_index as u32
    }

    pub fn select_texture_set(&mut self, index: u32) -> bool {
        let to = index as usize;
        if self.stroking || to >= self.sets.len() {
            return false;
        }
        if to == self.set_index {
            return true;
        }
        let current = Parked {
            layers: std::mem::take(&mut self.layers),
            active: self.active,
            fills: std::mem::take(&mut self.fills),
            filters: std::mem::take(&mut self.filters),
        };
        self.sets[self.set_index].parked = Some(current);
        self.gpu.select_set(to);
        self.set_index = to;
        match self.sets[to].parked.take() {
            Some(p) => {
                self.layers = p.layers;
                self.active = p.active;
                self.fills = p.fills;
                self.filters = p.filters;
            }
            None => {
                self.add_layer("Layer 1".into());
            }
        }
        self.dirty = true;
        true
    }

    pub fn base_json(&self) -> String {
        let b = self.gpu.base;
        serde_json::json!({ "base_color": [b[0], b[1], b[2]], "roughness": b[3], "metallic": b[4] }).to_string()
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
        if self.layers[index].kind != LayerKind::Paint && !self.layers[index].mask {
            return false;
        }
        self.gpu.begin_stroke(index);
        self.stroking = true;
        self.lazy = Some((x, y));
        self.last_stamp = self.pick(x, y);
        if let Some(hit) = self.last_stamp {
            self.push_stamp(&hit, pressure, None);
        }
        true
    }

    pub fn stroke_to(&mut self, x: f32, y: f32, pressure: f32) {
        if !self.stroking {
            return;
        }
        let (x, y) = match self.lazy {
            Some((lx, ly)) if self.brush.lazy_px > 0.0 => {
                let (dx, dy) = (x - lx, y - ly);
                let d = (dx * dx + dy * dy).sqrt();
                if d <= self.brush.lazy_px {
                    return;
                }
                let k = (d - self.brush.lazy_px) / d;
                let next = (lx + dx * k, ly + dy * k);
                self.lazy = Some(next);
                next
            }
            _ => {
                self.lazy = Some((x, y));
                (x, y)
            }
        };
        let hit = self.pick(x, y);
        self.cursor = hit;
        let Some(hit) = hit else {
            self.last_stamp = None;
            return;
        };
        let Some(last) = self.last_stamp else {
            self.last_stamp = Some(hit);
            self.push_stamp(&hit, pressure, None);
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
            let uv = [last.uv[0] + (hit.uv[0] - last.uv[0]) * t, last.uv[1] + (hit.uv[1] - last.uv[1]) * t];
            let stamp = Hit { position: p, normal: n, t: last.t + (hit.t - last.t) * t, uv, ..hit };
            self.push_stamp(&stamp, pressure, Some(sub(hit.position, last.position)));
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

    pub fn set_alpha(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<(), JsValue> {
        self.gpu.set_image(false, width, height, rgba).map_err(|e| JsValue::from_str(&e))
    }

    pub fn set_stencil(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<(), JsValue> {
        self.gpu.set_image(true, width, height, rgba).map_err(|e| JsValue::from_str(&e))
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
        let to_mask = self.paints_mask();
        self.gpu.paint(&[], view_proj, eye, &self.brush, &self.channels, index, to_mask);
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
        let (at, parent) = if self.layers.is_empty() {
            (0, 0)
        } else {
            let index = self.layer_index(self.active);
            let active = &self.layers[index];
            if active.kind == LayerKind::Folder { (index, active.id) } else { (index + 1, active.parent) }
        };
        self.layers.insert(at, LayerInfo { id, name, visible: true, opacity: 1.0, blend: BlendMode::Normal, kind: LayerKind::Paint, mask: false, parent });
        self.gpu.insert_layer(at, id);
        self.active = id;
        self.normalize();
        id
    }

    pub fn add_folder(&mut self, name: String) -> u32 {
        let id = self.add_layer(name);
        let index = self.layer_index(id);
        self.layers[index].kind = LayerKind::Folder;
        self.dirty = true;
        id
    }

    pub fn set_parent(&mut self, id: u32, parent: u32) -> bool {
        let Some(index) = self.layers.iter().position(|l| l.id == id) else { return false };
        if parent != 0 {
            let is_folder = self.layers.iter().any(|l| l.id == parent && l.kind == LayerKind::Folder);
            if !is_folder || parent == id || fs_core::descendants(&self.layers, id).contains(&parent) {
                return false;
            }
        }
        let mut layer = self.layers.remove(index);
        layer.parent = parent;
        self.layers.push(layer);
        self.normalize();
        true
    }

    pub fn remove_layer(&mut self, id: u32) -> bool {
        let Some(first) = self.layers.iter().position(|l| l.id == id) else { return false };
        let mut doomed = vec![id];
        doomed.extend(fs_core::descendants(&self.layers, id));
        if self.layers.len() <= doomed.len() {
            return false;
        }
        for d in &doomed {
            let Some(index) = self.layers.iter().position(|l| l.id == *d) else { continue };
            self.layers.remove(index);
            self.fills.remove(d);
            self.filters.remove(d);
            self.gpu.remove_layer(index, *d);
        }
        if doomed.contains(&self.active) {
            self.active = self.layers[first.min(self.layers.len() - 1)].id;
        }
        self.normalize();
        true
    }

    pub fn move_layer(&mut self, id: u32, offset: i32) {
        for _ in 0..offset.unsigned_abs().min(64) {
            let Some(index) = self.layers.iter().position(|l| l.id == id) else { return };
            let parent = self.layers[index].parent;
            let siblings: Vec<usize> = (0..self.layers.len()).filter(|&i| self.layers[i].parent == parent).collect();
            let pos = siblings.iter().position(|&i| i == index).unwrap_or(0) as i32;
            let next = pos + offset.signum();
            if next < 0 || next >= siblings.len() as i32 {
                break;
            }
            self.layers.swap(index, siblings[next as usize]);
            self.normalize();
        }
    }

    pub fn update_layer(&mut self, json: &str) -> Result<(), JsValue> {
        let mut info: LayerInfo = serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let index = self.layers.iter().position(|l| l.id == info.id).ok_or_else(|| JsValue::from_str("No layer with that id"))?;
        info.kind = self.layers[index].kind;
        info.parent = self.layers[index].parent;
        self.layers[index] = info;
        self.dirty = true;
        Ok(())
    }

    pub fn add_fill_layer(&mut self, name: String) -> u32 {
        let id = self.add_layer(name);
        let index = self.layer_index(id);
        self.layers[index].kind = LayerKind::Fill;
        self.fills.insert(id, FillSettings::default());
        self.render_fill(id);
        id
    }

    pub fn fill_settings(&self, id: u32) -> String {
        self.fills.get(&id).map(|f| serde_json::to_string(f).unwrap_or_default()).unwrap_or_default()
    }

    pub fn set_fill(&mut self, id: u32, json: &str) -> Result<(), JsValue> {
        if !self.fills.contains_key(&id) {
            return Err(JsValue::from_str("That layer is not a fill layer"));
        }
        let settings: FillSettings = serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.fills.insert(id, settings);
        self.render_fill(id);
        Ok(())
    }

    pub fn set_fill_image(&mut self, id: u32, width: u32, height: u32, rgba: &[u8]) -> Result<(), JsValue> {
        self.gpu.set_pattern(id, width, height, rgba).map_err(|e| JsValue::from_str(&e))?;
        self.render_fill(id);
        Ok(())
    }

    pub fn set_mask(&mut self, id: u32, enabled: bool) {
        let index = self.layer_index(id);
        self.layers[index].mask = enabled;
        self.dirty = true;
    }

    pub fn fill_mask(&mut self, id: u32, white: bool) {
        let index = self.layer_index(id);
        self.layers[index].mask = true;
        self.gpu.fill_mask_value(index, if white { 1.0 } else { 0.0 });
        self.dirty = true;
    }

    pub fn set_paint_target(&mut self, mask: bool) {
        self.mask_target = mask;
    }

    pub fn paint_target_is_mask(&self) -> bool {
        self.paints_mask()
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
            self.gpu.composite(&self.layers, &self.filters);
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
            self.gpu.composite(&self.layers, &self.filters);
            self.dirty = false;
        }
        let (center, radius) = self.mesh.as_ref().map(|m| (m.0.center, m.0.radius)).unwrap_or(([0.0; 3], 1.0));
        self.gpu.export(channel, self.normal_strength, center, radius)
    }

    pub fn bake_maps(&mut self, directions: u32) -> Result<(), JsValue> {
        let (center, radius) = self.mesh.as_ref().map(|m| (m.0.center, m.0.radius)).ok_or_else(|| JsValue::from_str("Load a mesh before baking"))?;
        self.each_set(|engine| {
            engine.gpu.bake_maps(center, radius, directions)?;
            engine.render_fills();
            Ok(())
        })
        .map_err(|e| JsValue::from_str(&e))
    }

    pub fn set_high_mesh(&mut self, positions: &[f32], normals: &[f32], indices: &[u32], parts: &[u32]) -> Result<String, JsValue> {
        let mesh = Mesh::geometry_only(positions, normals, indices).map_err(|e| JsValue::from_str(&e))?;
        let bvh = Bvh::build(&mesh);
        let tris = mesh.triangle_count();
        let parts = if parts.len() == tris { parts.to_vec() } else { vec![0; tris] };
        let count = parts.iter().copied().collect::<std::collections::BTreeSet<u32>>().len();
        self.high = Some((mesh, bvh, parts));
        Ok(format!("{tris} triangles, {count} parts"))
    }

    pub fn clear_high_mesh(&mut self) {
        self.high = None;
    }

    pub fn bake_high(&mut self, front: f32, rear: f32) -> Result<String, JsValue> {
        if self.mesh.is_none() {
            return Err(JsValue::from_str("Load the low-poly mesh first"));
        }
        if self.high.is_none() {
            return Err(JsValue::from_str("Load a high-poly mesh first"));
        }
        let (mut hits, mut texels) = (0usize, 0usize);
        self.each_set(|engine| {
            let (Some((low, _)), Some((high, bvh, parts))) = (engine.mesh.as_ref(), engine.high.as_ref()) else { return Ok(()) };
            let info = &engine.sets[engine.set_index].info;
            let low = low.subset(info.start as usize, info.count as usize);
            let size = engine.gpu.size.min(2048) as usize;
            let r = low.radius;
            let result = fs_core::bake::bake_high(&low, high, bvh, parts, size, front.max(0.0) * r, rear.max(0.0) * r);
            hits += result.hits;
            texels += result.texels;
            engine.gpu.set_high_bake(size as u32, &result.normal, &result.id);
            Ok(())
        })
        .map_err(|e| JsValue::from_str(&e))?;
        let sets = self.sets.len();
        Ok(format!("{hits} of {texels} texels hit the high-poly mesh across {sets} texture set{}", if sets == 1 { "" } else { "s" }))
    }

    pub fn is_high_baked(&self) -> bool {
        self.gpu.high_baked
    }

    pub fn is_baked(&self) -> bool {
        self.gpu.baked
    }

    pub fn add_filter_layer(&mut self, name: String) -> u32 {
        let id = self.add_layer(name);
        let index = self.layer_index(id);
        self.layers[index].kind = LayerKind::Filter;
        self.filters.insert(id, FilterSettings::default());
        self.dirty = true;
        id
    }

    pub fn filter_settings(&self, id: u32) -> String {
        self.filters.get(&id).map(|f| serde_json::to_string(f).unwrap_or_default()).unwrap_or_default()
    }

    pub fn set_filter(&mut self, id: u32, json: &str) -> Result<(), JsValue> {
        if !self.filters.contains_key(&id) {
            return Err(JsValue::from_str("That layer is not a filter layer"));
        }
        let settings: FilterSettings = serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.filters.insert(id, settings);
        self.dirty = true;
        Ok(())
    }

    pub fn snapshot(&mut self) -> js_sys::Promise {
        self.flush();
        if self.dirty {
            self.gpu.composite(&self.layers, &self.filters);
            self.dirty = false;
        }
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
        if self.brush.size_space == SizeSpace::Object {
            let r = self.mesh.as_ref().map(|m| m.0.radius).unwrap_or(1.0);
            return self.brush.size_px / 1000.0 * r * p;
        }
        let world_per_px = 2.0 * hit.t * (self.fov * 0.5).tan() / self.height as f32;
        self.brush.size_px * 0.5 * world_per_px * p
    }

    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 40) as f32 / (1u64 << 24) as f32
    }

    fn push_stamp(&mut self, hit: &Hit, pressure: f32, direction: Option<Vec3>) {
        let Some((mesh, _)) = &self.mesh else { return };
        let center = mesh.center;
        let uv_scale = mesh.uv_per_world(hit.triangle);
        let b = self.brush;
        let mut r = self.world_radius(hit, pressure) * (1.0 - b.jitter_size.clamp(0.0, 1.0) * self.random());
        r = r.max(1e-6);
        let n = hit.normal;
        let (_, right, up, _) = self.basis();
        let mut t = sub(right, scale(n, dot(right, n)));
        if fs_core::length(t) < 1e-4 {
            t = sub(up, scale(n, dot(up, n)));
        }
        let t = normalize(t);
        let bt = cross(n, t);
        let mut angle = b.angle.to_radians();
        if b.follow_path {
            if let Some(d) = direction {
                angle += dot(d, bt).atan2(dot(d, t));
            }
        }
        angle += (self.random() * 2.0 - 1.0) * b.jitter_angle.clamp(0.0, 1.0) * std::f32::consts::PI;
        let jp = b.jitter_position.clamp(0.0, 4.0) * r;
        let (jx, jy) = ((self.random() * 2.0 - 1.0) * jp, (self.random() * 2.0 - 1.0) * jp);
        let p = add(hit.position, add(scale(t, jx), scale(bt, jy)));
        let flow_scale = (if b.pressure_flow { pressure.clamp(0.0, 1.0) } else { 1.0 }) * (1.0 - b.jitter_flow.clamp(0.0, 1.0) * self.random());
        let uv = [hit.uv[0] + (jx * uv_scale), hit.uv[1] - (jy * uv_scale)];
        let stamp = |p: Vec3, n: Vec3, t: Vec3, mirrored: f32| -> [f32; 16] {
            [p[0], p[1], p[2], r, n[0], n[1], n[2], mirrored, t[0], t[1], t[2], angle, uv[0], uv[1], r * uv_scale, flow_scale]
        };
        self.pending.push(stamp(p, n, t, 0.0));
        let symmetry = if b.mirror_x && b.symmetry == Symmetry::Off { Symmetry::Mirror } else { b.symmetry };
        let axis = (b.symmetry_axis as usize).min(2);
        match symmetry {
            Symmetry::Off => {}
            Symmetry::Mirror => {
                let flip = |mut v: Vec3, about: f32| {
                    v[axis] = 2.0 * about - v[axis];
                    v
                };
                let flipn = |mut v: Vec3| {
                    v[axis] = -v[axis];
                    v
                };
                self.pending.push(stamp(flip(p, center[axis]), flipn(n), flipn(t), 1.0));
            }
            Symmetry::Radial => {
                let count = b.radial_count.clamp(2, 32);
                for k in 1..count {
                    let a = std::f32::consts::TAU * k as f32 / count as f32;
                    let rot = |v: Vec3| -> Vec3 {
                        let (s, c) = a.sin_cos();
                        let (i, j) = match axis {
                            0 => (1, 2),
                            1 => (2, 0),
                            _ => (0, 1),
                        };
                        let mut o = v;
                        o[i] = v[i] * c - v[j] * s;
                        o[j] = v[i] * s + v[j] * c;
                        o
                    };
                    let q = add(rot(sub(p, center)), center);
                    self.pending.push(stamp(q, rot(n), rot(t), 1.0));
                }
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
        let to_mask = self.paints_mask();
        self.gpu.paint(&stamps, view_proj, eye, &self.brush, &self.channels, self.layer_index(self.active), to_mask);
        self.dirty = true;
    }

    fn paints_mask(&self) -> bool {
        let layer = &self.layers[self.layer_index(self.active)];
        layer.kind != LayerKind::Paint || (self.mask_target && layer.mask)
    }

    fn each_set(&mut self, mut work: impl FnMut(&mut Engine) -> Result<(), String>) -> Result<(), String> {
        let original = self.set_index as u32;
        let mut result = Ok(());
        for k in 0..self.sets.len() {
            self.select_texture_set(k as u32);
            if let Err(e) = work(self) {
                result = Err(e);
                break;
            }
            self.gpu.composite(&self.layers, &self.filters);
        }
        self.select_texture_set(original);
        self.dirty = true;
        result
    }

    fn render_fills(&mut self) {
        let ids: Vec<u32> = self.fills.keys().copied().collect();
        for id in ids {
            self.render_fill(id);
        }
    }

    fn render_fill(&mut self, id: u32) {
        let Some(settings) = self.fills.get(&id).copied() else { return };
        let Some((mesh, _)) = &self.mesh else { return };
        let uniform = settings.uniform(mesh.center, mesh.radius);
        let index = self.layer_index(id);
        self.gpu.fill_layer_content(index, &uniform);
        self.dirty = true;
    }

    fn normalize(&mut self) {
        self.layers = fs_core::tree_order(&self.layers);
        let ids = self.layer_ids();
        self.gpu.reorder(&ids);
        self.dirty = true;
    }

    fn layer_index(&self, id: u32) -> usize {
        self.layers.iter().position(|l| l.id == id).unwrap_or(0)
    }

    fn layer_ids(&self) -> Vec<u32> {
        self.layers.iter().map(|l| l.id).collect()
    }
}
