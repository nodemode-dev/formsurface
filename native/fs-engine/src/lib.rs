mod gpu;
mod shaders;

use fs_core::{add, cross, dot, normalize, scale, sub, BlendMode, BrushSettings, Bvh, ChannelValues, FillSettings, FilterSettings, Hit, LayerInfo, LayerKind, Mesh, SizeSpace, StrokeMode, Symmetry, Vec3};
use fs_core::sets::TextureSet;
use gpu::Gpu;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

struct Parked {
    layers: Vec<LayerInfo>,
    active: u32,
    fills: HashMap<u32, FillSettings>,
    filters: HashMap<u32, FilterSettings>,
    mask_gens: HashMap<u32, FillSettings>,
}

struct SetEntry {
    info: TextureSet,
    parked: Option<Parked>,
}

const POST_KEYS: [&str; 16] = ["bloom", "bloom_threshold", "glare", "lens_flare", "vignette", "vignette_softness", "grain", "sharpen", "aberration", "dof_aperture", "dof_focus", "contrast", "saturation", "temperature", "tint", "gain"];
const POST_DEFAULTS: [f32; 16] = [0.0, 0.8, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
const POST_RANGES: [(f32, f32); 16] = [(0.0, 2.0), (0.0, 0.99), (0.0, 2.0), (0.0, 2.0), (0.0, 1.0), (0.0, 1.0), (0.0, 1.0), (0.0, 2.0), (0.0, 1.0), (0.0, 24.0), (-1.0, 1.0), (-1.0, 1.0), (0.0, 2.0), (-1.0, 1.0), (-1.0, 1.0), (0.0, 2.0)];

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
    post: [f32; 16],
    post_seed: f32,
    cage: Option<Vec<f32>>,
    normal_strength: f32,
    uv_tiles: bool,
    relaying: bool,
    clone_source: Option<[f32; 2]>,
    tri_order: Vec<u32>,
    match_keys: Option<(Vec<u32>, Vec<u32>)>,
    instance_groups: Vec<Vec<(usize, u32)>>,
    instancing: bool,
    mask_gens: HashMap<u32, FillSettings>,
    id_maps: HashMap<usize, (u32, Vec<u8>)>,
    flat: Option<[f32; 3]>,
    flat_cursor: Option<[f32; 2]>,
    last_uv: Option<[f32; 2]>,
    environment: Option<String>,
    env_view: [f32; 3],
    clone_layer: Option<u32>,
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
        post: POST_DEFAULTS,
        post_seed: 0.0,
        cage: None,
        normal_strength: 1.0,
        uv_tiles: false,
        relaying: false,
        clone_source: None,
        tri_order: Vec::new(),
        match_keys: None,
        instance_groups: Vec::new(),
        instancing: false,
        mask_gens: HashMap::new(),
        id_maps: HashMap::new(),
        flat: None,
        flat_cursor: None,
        last_uv: None,
        environment: None,
        env_view: [0.0, 0.0, 1.0],
        clone_layer: None,
    };
    engine.add_layer("Layer 1".into());
    Ok(engine)
}

#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn bake_high_detached(low_positions: &[f32], low_normals: &[f32], low_uvs: &[f32], low_indices: &[u32], high_positions: &[f32], high_normals: &[f32], high_indices: &[u32], parts: &[u32], size: u32, front: f32, rear: f32, low_keys: &[u32], high_keys: &[u32], ao_rays: u32, cage: &[f32]) -> Result<js_sys::Array, JsValue> {
    let low = Mesh::from_flat(low_positions, low_normals, low_uvs, low_indices).map_err(|e| JsValue::from_str(&e))?;
    let high = Mesh::geometry_only(high_positions, high_normals, high_indices).map_err(|e| JsValue::from_str(&e))?;
    let bvh = Bvh::build(&high);
    let tris = high.triangle_count();
    let parts = if parts.len() == tris { parts.to_vec() } else { vec![0; tris] };
    let r = low.radius;
    let size = size.clamp(64, 2048) as usize;
    let keys = if low_keys.len() * 3 == low.indices.len() && high_keys.len() == tris { Some((low_keys, high_keys)) } else { None };
    let cage = if !cage.is_empty() && cage.len() == low.indices.len() * 3 { Some(cage) } else { None };
    let result = fs_core::bake::bake_high_matched(&low, &high, &bvh, &parts, size, front.max(0.0) * r, rear.max(0.0) * r, keys, ao_rays.min(64) as usize, cage);
    let out = js_sys::Array::new();
    out.push(&js_sys::Uint8Array::from(&result.normal[..]));
    out.push(&js_sys::Uint8Array::from(&result.id[..]));
    out.push(&JsValue::from_f64(result.hits as f64));
    out.push(&JsValue::from_f64(result.texels as f64));
    for map in [&result.height, &result.opacity, &result.ao, &result.bent] {
        out.push(&js_sys::Uint8Array::from(&map[..]));
    }
    Ok(out)
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
        self.tri_order = split.order;
        self.match_keys = None;
        self.cage = None;
        self.instance_groups.clear();
        self.set_index = 0;
        self.id_maps.clear();
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
        if self.uv_tiles {
            self.set_uv_tiles(true);
        }
        Ok(info)
    }

    pub fn set_match_names(&mut self, low_parts: &[u32], low_names_json: &str, high_names_json: &str) -> Result<String, JsValue> {
        let low_names: Vec<String> = serde_json::from_str(low_names_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let high_names: Vec<String> = serde_json::from_str(high_names_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let (_, _, high_parts) = self.high.as_ref().ok_or_else(|| JsValue::from_str("Load a high-poly mesh first"))?;
        if low_parts.len() != self.tri_order.len() {
            return Err(JsValue::from_str("Low-poly part list does not match the loaded mesh"));
        }
        let low_keys: Vec<String> = low_names.iter().map(|n| fs_core::bake::match_key(n)).collect();
        let high_keys: Vec<String> = high_names.iter().map(|n| fs_core::bake::match_key(n)).collect();
        let shared: Vec<String> = low_keys.iter().filter(|k| !k.is_empty() && high_keys.contains(k)).cloned().collect();
        let id_of = |k: &String| shared.iter().position(|s| s == k).map(|p| p as u32).unwrap_or(u32::MAX);
        let low_ids: Vec<u32> = low_keys.iter().map(id_of).collect();
        let high_ids: Vec<u32> = high_keys.iter().map(id_of).collect();
        let low_tri: Vec<u32> = self.tri_order.iter().map(|&t| low_ids.get(low_parts[t as usize] as usize).copied().unwrap_or(u32::MAX)).collect();
        let high_tri: Vec<u32> = high_parts.iter().map(|&p| high_ids.get(p as usize).copied().unwrap_or(u32::MAX)).collect();
        let mut unique_low = low_keys.clone();
        unique_low.sort();
        unique_low.dedup();
        let matched = unique_low.iter().filter(|k| shared.contains(k)).count();
        self.match_keys = Some((low_tri, high_tri));
        Ok(format!("{matched} of {} low-poly parts matched a high-poly part by name; unmatched parts bake only against unmatched high-poly parts", unique_low.len()))
    }

    pub fn clear_match_names(&mut self) {
        self.match_keys = None;
    }

    pub fn texture_set_match_keys(&self, index: u32) -> js_sys::Array {
        let out = js_sys::Array::new();
        if let (Some((low, high)), Some(set)) = (&self.match_keys, self.sets.get(index as usize)) {
            let first = (set.info.start / 3) as usize;
            let count = (set.info.count / 3) as usize;
            out.push(&js_sys::Uint32Array::from(&low[first..first + count]));
            out.push(&js_sys::Uint32Array::from(&high[..]));
        }
        out
    }

    pub fn set_clone_source(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
        if let Some(uv) = self.flat_uv(x, y) {
            if !(0.0..=1.0).contains(&uv[0]) || !(0.0..=1.0).contains(&uv[1]) {
                return Err(JsValue::from_str("Click inside the UV square to set the clone source"));
            }
            self.clone_source = Some(uv);
            self.gpu.clone_offset = None;
            return Ok(());
        }
        let hit =self.pick(x, y).ok_or_else(|| JsValue::from_str("Click on the mesh to set the clone source"))?;
        let info = &self.sets[self.set_index].info;
        if hit.triangle < info.start / 3 || hit.triangle >= (info.start + info.count) / 3 {
            return Err(JsValue::from_str("The clone source must be on the active texture set"));
        }
        self.clone_source = Some(hit.uv);
        self.gpu.clone_offset = None;
        Ok(())
    }

    pub fn set_clone_layer(&mut self, id: u32) -> Result<(), JsValue> {
        if id == 0 {
            self.clone_layer = None;
            return Ok(());
        }
        let layer = self.layers.iter().find(|l| l.id == id).ok_or_else(|| JsValue::from_str("No layer with that id in the active texture set"))?;
        if layer.kind == LayerKind::Folder || layer.kind == LayerKind::Filter {
            return Err(JsValue::from_str("Clone from a paint or fill layer; folders and filters hold no pixels of their own"));
        }
        self.clone_layer = Some(id);
        Ok(())
    }

    pub fn clone_layer(&self) -> u32 {
        self.clone_layer.unwrap_or(0)
    }

    pub fn has_clone_source(&self) -> bool {
        self.clone_source.is_some()
    }

    pub fn flatten_stack(&mut self, name: String) -> u32 {
        if self.stroking {
            return 0;
        }
        if let Some(r) = self.relay_new(|e| e.flatten_stack(name.clone())) {
            return r;
        }
        self.flush();
        self.gpu.composite(&self.layers, &self.filters);
        let id = self.next_id;
        self.next_id += 1;
        let at = self.layers.len();
        self.layers.push(LayerInfo { id, name, visible: true, opacity: 1.0, blend: BlendMode::Normal, kind: LayerKind::Paint, mask: false, parent: 0, instance: false });
        self.gpu.insert_layer(at, id);
        self.gpu.flatten_into(at);
        while self.layers.len() > 1 {
            let old = self.layers.remove(0);
            self.gpu.remove_layer(0, old.id);
        }
        self.fills.clear();
        self.filters.clear();
        self.mask_gens.clear();
        self.active = id;
        self.normalize();
        id
    }

    fn merge_partner(&self, id: u32) -> Result<(usize, usize), String> {
        let i = self.layers.iter().position(|l| l.id == id).ok_or("No layer with that id in the active texture set")?;
        let top = &self.layers[i];
        if top.kind != LayerKind::Paint && top.kind != LayerKind::Fill {
            return Err("Merge down works on paint and fill layers; use flatten for a folder".into());
        }
        let j = (0..i).rev().find(|&j| self.layers[j].parent == top.parent).ok_or("There is no layer below it in the same folder")?;
        let bottom = &self.layers[j];
        if bottom.kind != LayerKind::Paint && bottom.kind != LayerKind::Fill {
            return Err(format!("The layer below is a {}; merge down needs a paint or fill layer below", if bottom.kind == LayerKind::Folder { "folder" } else { "filter" }));
        }
        Ok((i, j))
    }

    pub fn merge_down(&mut self, id: u32) -> Result<u32, JsValue> {
        if self.stroking {
            return Err(JsValue::from_str("Finish the stroke first"));
        }
        self.merge_partner(id).map_err(|e| JsValue::from_str(&e))?;
        if let Some(r) = self.relay_new(|e| e.merge_down_here(id)) {
            return Ok(r);
        }
        Ok(self.merge_down_here(id))
    }

    fn merge_down_here(&mut self, id: u32) -> u32 {
        let Ok((i, j)) = self.merge_partner(id) else { return 0 };
        self.flush();
        let (top, bottom) = (self.layers[i].clone(), self.layers[j].clone());
        let nid = self.next_id;
        self.next_id += 1;
        self.layers.insert(j, LayerInfo { id: nid, name: bottom.name.clone(), visible: true, opacity: 1.0, blend: bottom.blend, kind: LayerKind::Paint, mask: false, parent: bottom.parent, instance: false });
        self.gpu.insert_layer(j, nid);
        let vis = |l: &LayerInfo| if l.visible { l.opacity } else { 0.0 };
        let mask = |l: &LayerInfo| if l.mask { 1.0 } else { 0.0 };
        self.gpu.merge_layers(i + 1, j + 1, j, [vis(&top), mask(&top)], [vis(&bottom), mask(&bottom)], top.blend.code());
        for old in [top.id, bottom.id] {
            if let Some(index) = self.layers.iter().position(|l| l.id == old) {
                self.layers.remove(index);
                self.gpu.remove_layer(index, old);
            }
            self.fills.remove(&old);
            self.mask_gens.remove(&old);
        }
        self.active = nid;
        self.normalize();
        nid
    }

    pub fn flatten_folder(&mut self, id: u32) -> Result<u32, JsValue> {
        if self.stroking {
            return Err(JsValue::from_str("Finish the stroke first"));
        }
        self.folder_children(id).map_err(|e| JsValue::from_str(&e))?;
        if let Some(r) = self.relay_new(|e| e.flatten_folder_here(id)) {
            return Ok(r);
        }
        Ok(self.flatten_folder_here(id))
    }

    fn folder_children(&self, id: u32) -> Result<Vec<u32>, String> {
        let folder = self.layers.iter().find(|l| l.id == id).ok_or("No layer with that id in the active texture set")?;
        if folder.kind != LayerKind::Folder {
            return Err("That layer is not a folder; use merge down for layers".into());
        }
        let children: Vec<&LayerInfo> = self.layers.iter().filter(|l| l.parent == id).collect();
        if children.is_empty() {
            return Err("The folder is empty".into());
        }
        if let Some(bad) = children.iter().find(|l| l.kind != LayerKind::Paint && l.kind != LayerKind::Fill) {
            return Err(format!("{} is a {}; flatten nested folders first, and move filter layers out of the folder", bad.name, if bad.kind == LayerKind::Folder { "folder" } else { "filter" }));
        }
        Ok(children.iter().map(|l| l.id).collect())
    }

    fn flatten_folder_here(&mut self, id: u32) -> u32 {
        let Ok(children) = self.folder_children(id) else { return 0 };
        self.flush();
        let Some(fi) = self.layers.iter().position(|l| l.id == id) else { return 0 };
        let folder = self.layers[fi].clone();
        let nid = self.next_id;
        self.next_id += 1;
        self.layers.insert(fi, LayerInfo { id: nid, name: folder.name.clone(), visible: folder.visible, opacity: folder.opacity, blend: BlendMode::Normal, kind: LayerKind::Paint, mask: folder.mask, parent: folder.parent, instance: false });
        self.gpu.insert_layer(fi, nid);
        for child in &children {
            let (Some(ci), Some(ni)) = (self.layers.iter().position(|l| l.id == *child), self.layers.iter().position(|l| l.id == nid)) else { continue };
            let c = self.layers[ci].clone();
            let k = if c.visible { c.opacity } else { 0.0 };
            self.gpu.merge_layers(ci, ni, ni, [k, if c.mask { 1.0 } else { 0.0 }], [1.0, 0.0], c.blend.code());
        }
        if folder.mask {
            if let (Some(from), Some(to)) = (self.layers.iter().position(|l| l.id == id), self.layers.iter().position(|l| l.id == nid)) {
                self.gpu.copy_mask(from, to);
            }
        }
        for old in children.iter().copied().chain([id]) {
            if let Some(index) = self.layers.iter().position(|l| l.id == old) {
                self.layers.remove(index);
                self.gpu.remove_layer(index, old);
            }
            self.fills.remove(&old);
            self.mask_gens.remove(&old);
        }
        self.active = nid;
        self.normalize();
        nid
    }

    pub fn export_layer(&mut self, id: u32, channel: u32) -> Result<js_sys::Promise, JsValue> {
        let index = self.layers.iter().position(|l| l.id == id).ok_or_else(|| JsValue::from_str("No layer with that id"))?;
        let kind = self.layers[index].kind;
        if kind == LayerKind::Folder || kind == LayerKind::Filter {
            return Err(JsValue::from_str("Folders and filter layers hold no pixels of their own; flatten the folder first"));
        }
        self.flush();
        Ok(self.gpu.export_layer(index, channel.min(3)))
    }

    pub fn export_layer_mask(&mut self, id: u32) -> Result<js_sys::Promise, JsValue> {
        let index = self.layers.iter().position(|l| l.id == id).ok_or_else(|| JsValue::from_str("No layer with that id"))?;
        if !self.layers[index].mask {
            return Err(JsValue::from_str("That layer has no mask"));
        }
        Ok(self.gpu.export_mask(index))
    }

    pub fn mesh_obj(&self) -> Result<String, JsValue> {
        use std::fmt::Write;
        let (mesh, _) = self.mesh.as_ref().ok_or_else(|| JsValue::from_str("Load a mesh first"))?;
        let mut uvs = mesh.uvs.clone();
        let mut done = vec![false; uvs.len()];
        for set in &self.sets {
            let (col, row) = ((set.info.udim - 1001) % 10, (set.info.udim - 1001) / 10);
            for &v in &mesh.indices[set.info.start as usize..(set.info.start + set.info.count) as usize] {
                if !done[v as usize] {
                    done[v as usize] = true;
                    let uv = mesh.uvs[v as usize];
                    uvs[v as usize] = [uv[0] + col as f32, 1.0 - (uv[1] - row as f32)];
                }
            }
        }
        let mut out = String::with_capacity(mesh.positions.len() * 96);
        let _ = writeln!(out, "# FormSurface {} OBJ: {} vertices, {} triangles, {} texture sets", env!("CARGO_PKG_VERSION"), mesh.positions.len(), mesh.triangle_count(), self.sets.len());
        for p in &mesh.positions {
            let _ = writeln!(out, "v {} {} {}", p[0], p[1], p[2]);
        }
        for uv in &uvs {
            let _ = writeln!(out, "vt {} {}", uv[0], uv[1]);
        }
        for n in &mesh.normals {
            let _ = writeln!(out, "vn {} {} {}", n[0], n[1], n[2]);
        }
        for set in &self.sets {
            let _ = writeln!(out, "g {}\nusemtl {}", set.info.name(), set.info.material);
            for tri in mesh.indices[set.info.start as usize..(set.info.start + set.info.count) as usize].chunks_exact(3) {
                let (a, b, c) = (tri[0] + 1, tri[1] + 1, tri[2] + 1);
                let _ = writeln!(out, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}");
            }
        }
        Ok(out)
    }

    pub fn texture_sets_json(&self) -> String {
        let list: Vec<serde_json::Value> = self
            .sets
            .iter()
            .enumerate()
            .map(|(k, s)| {
                let layers = if k == self.set_index { self.layers.len() } else { s.parked.as_ref().map(|p| p.layers.len()).unwrap_or(1) };
                serde_json::json!({ "index": k, "name": s.info.name(), "material": s.info.material, "udim": s.info.udim, "tiled": s.info.tiled, "triangles": s.info.count / 3, "layers": layers, "linked": self.uv_tiles && s.info.tiled })
            })
            .collect();
        serde_json::Value::Array(list).to_string()
    }

    pub fn active_texture_set(&self) -> u32 {
        self.set_index as u32
    }

    pub fn select_texture_set(&mut self, index: u32) -> bool {
        if self.stroking || index as usize >= self.sets.len() {
            return false;
        }
        self.switch_set(index as usize);
        true
    }

    pub fn uv_tiles(&self) -> bool {
        self.uv_tiles
    }

    pub fn set_uv_tiles(&mut self, on: bool) -> bool {
        if self.stroking {
            return false;
        }
        self.uv_tiles = on;
        if !on {
            return true;
        }
        let original = self.set_index;
        let mut materials: Vec<String> = self.sets.iter().filter(|s| s.info.tiled).map(|s| s.info.material.clone()).collect();
        materials.dedup();
        for material in materials {
            let group: Vec<usize> = (0..self.sets.len()).filter(|&k| self.sets[k].info.tiled && self.sets[k].info.material == material).collect();
            let leader = if group.contains(&original) { original } else { group[0] };
            self.switch_set(leader);
            let (layers, fills, filters, gens, base, active) = (self.layers.clone(), self.fills.clone(), self.filters.clone(), self.mask_gens.clone(), self.gpu.base, self.active);
            for &k in group.iter().filter(|&&k| k != leader) {
                self.switch_set(k);
                while let Some(layer) = self.layers.pop() {
                    let index = self.layers.len();
                    self.gpu.remove_layer(index, layer.id);
                }
                for (i, layer) in layers.iter().enumerate() {
                    self.layers.push(layer.clone());
                    self.gpu.insert_layer(i, layer.id);
                }
                self.fills = fills.clone();
                self.filters = filters.clone();
                self.mask_gens = gens.clone();
                self.gpu.base = base;
                self.active = active;
                self.render_fills();
                self.gpu.composite(&self.layers, &self.filters);
            }
        }
        self.switch_set(original);
        self.dirty = true;
        true
    }

    pub fn mask_generator(&mut self, id: u32, json: &str) -> Result<(), JsValue> {
        if let Some(r) = self.relay(|e| e.mask_generator(id, json)) {
            return r;
        }
        let settings: FillSettings = serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let index = self.layers.iter().position(|l| l.id == id).ok_or_else(|| JsValue::from_str("No layer with that id"))?;
        self.layers[index].mask = true;
        self.mask_gens.insert(id, settings);
        self.render_mask_gen(id);
        self.propagate(id, |e, other| {
            let _ = e.mask_generator(other, json);
        });
        Ok(())
    }

    pub fn mask_generator_settings(&self, id: u32) -> String {
        self.mask_gens.get(&id).map(|f| serde_json::to_string(f).unwrap_or_default()).unwrap_or_default()
    }

    pub fn mask_from_image(&mut self, id: u32, width: u32, height: u32, rgba: &[u8]) -> Result<(), JsValue> {
        if let Some(r) = self.relay(|e| e.mask_from_image(id, width, height, rgba)) {
            return r;
        }
        let index = self.layers.iter().position(|l| l.id == id).ok_or_else(|| JsValue::from_str("No layer with that id"))?;
        self.mask_gens.remove(&id);
        self.gpu.mask_from_image(index, width, height, rgba).map_err(|e| JsValue::from_str(&e))?;
        self.layers[index].mask = true;
        self.dirty = true;
        Ok(())
    }

    pub fn mask_from_id(&mut self, id: u32, r: f32, g: f32, b: f32, tolerance: f32) -> Result<(), JsValue> {
        if let Some(out) = self.relay(|e| e.mask_from_id(id, r, g, b, tolerance)) {
            return out;
        }
        let index = self.layers.iter().position(|l| l.id == id).ok_or_else(|| JsValue::from_str("No layer with that id"))?;
        self.mask_gens.remove(&id);
        self.gpu.mask_from_id(index, [r, g, b], tolerance.clamp(0.0, 1.0)).map_err(|e| JsValue::from_str(&e))?;
        self.layers[index].mask = true;
        self.dirty = true;
        Ok(())
    }

    pub fn id_color_at(&self, x: f32, y: f32) -> Result<String, JsValue> {
        if self.flat.is_some() {
            return Err(JsValue::from_str("Pick an ID color in the 3D view"));
        }
        let hit =self.pick(x, y).ok_or_else(|| JsValue::from_str("Click on the mesh to pick an ID color"))?;
        let set = self.sets.iter().position(|s| hit.triangle >= s.info.start / 3 && hit.triangle < (s.info.start + s.info.count) / 3).unwrap_or(self.set_index);
        let (size, bytes) = self.id_maps.get(&set).ok_or_else(|| JsValue::from_str("Bake normal and ID from a high-poly mesh first"))?;
        let s = *size as f32;
        let px = ((hit.uv[0] * s) as i64).clamp(0, *size as i64 - 1) as usize;
        let py = ((hit.uv[1] * s) as i64).clamp(0, *size as i64 - 1) as usize;
        let o = (py * *size as usize + px) * 4;
        Ok(format!("#{:02x}{:02x}{:02x}", bytes[o], bytes[o + 1], bytes[o + 2]))
    }
}

impl Engine {
    fn siblings(&self) -> Vec<usize> {
        let Some(entry) = self.sets.get(self.set_index) else { return Vec::new() };
        if !self.uv_tiles || !entry.info.tiled {
            return Vec::new();
        }
        (0..self.sets.len()).filter(|&k| k != self.set_index && self.sets[k].info.tiled && self.sets[k].info.material == entry.info.material).collect()
    }

    fn relay<R>(&mut self, mut work: impl FnMut(&mut Engine) -> R) -> Option<R> {
        if self.relaying {
            return None;
        }
        let siblings = self.siblings();
        if siblings.is_empty() {
            return None;
        }
        self.relaying = true;
        let first = work(self);
        let original = self.set_index;
        for k in siblings {
            self.switch_set(k);
            work(self);
            self.flush_set();
        }
        self.switch_set(original);
        self.relaying = false;
        self.dirty = true;
        Some(first)
    }

    fn propagate(&mut self, id: u32, mut work: impl FnMut(&mut Engine, u32)) {
        if self.instancing || self.relaying {
            return;
        }
        let here = (self.set_index, id);
        let Some(group) = self.instance_groups.iter().find(|g| g.contains(&here)).cloned() else { return };
        self.instancing = true;
        let original = self.set_index;
        for (set, other) in group.into_iter().filter(|m| *m != here) {
            self.switch_set(set);
            if self.layers.iter().any(|l| l.id == other) {
                work(self, other);
                self.gpu.composite(&self.layers, &self.filters);
            }
        }
        self.switch_set(original);
        self.instancing = false;
        self.dirty = true;
    }

    fn each_sibling(&mut self, mut work: impl FnMut(&mut Engine)) {
        let siblings = self.siblings();
        if siblings.is_empty() {
            return;
        }
        let original = self.set_index;
        for k in siblings {
            self.switch_set(k);
            work(self);
        }
        self.switch_set(original);
    }

    fn relay_new(&mut self, mut work: impl FnMut(&mut Engine) -> u32) -> Option<u32> {
        let id = self.next_id;
        self.relay(|e| {
            let saved = e.next_id;
            e.next_id = id;
            let r = work(e);
            e.next_id = saved.max(id + 1);
            r
        })
    }

    fn flush_set(&mut self) {
        self.gpu.composite(&self.layers, &self.filters);
    }

    fn render_mask_gen(&mut self, id: u32) {
        let Some(mut settings) = self.mask_gens.get(&id).copied() else { return };
        let Some((mesh, _)) = &self.mesh else { return };
        settings.use_roughness = true;
        settings.opacity = 1.0;
        let uniform = settings.uniform(mesh.center, mesh.radius);
        let Some(index) = self.layers.iter().position(|l| l.id == id) else { return };
        self.gpu.mask_from_fill(index, &uniform);
        self.dirty = true;
    }

    fn switch_set(&mut self, to: usize) {
        if to >= self.sets.len() || to == self.set_index {
            return;
        }
        let current = Parked {
            layers: std::mem::take(&mut self.layers),
            active: self.active,
            fills: std::mem::take(&mut self.fills),
            filters: std::mem::take(&mut self.filters),
            mask_gens: std::mem::take(&mut self.mask_gens),
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
                self.mask_gens = p.mask_gens;
            }
            None => {
                let saved = self.relaying;
                self.relaying = true;
                self.add_layer("Layer 1".into());
                self.relaying = saved;
            }
        }
        self.dirty = true;
    }
}

#[wasm_bindgen]
impl Engine {
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
        if let Some(f) = &mut self.flat {
            f[0] -= dx / f[2];
            f[1] -= dy / f[2];
            return;
        }
        let (_, right, up, _) = self.basis();
        let k = self.distance * 0.0015;
        self.target = add(self.target, add(scale(right, -dx * k), scale(up, dy * k)));
    }

    pub fn zoom(&mut self, delta: f32) {
        if let Some(f) = &mut self.flat {
            f[2] = (f[2] * (1.0 - delta * 0.001)).clamp(32.0, 32768.0);
            return;
        }
        let r =self.mesh.as_ref().map(|m| m.0.radius).unwrap_or(1.0);
        self.distance = (self.distance * (1.0 + delta * 0.001)).clamp(r * 0.05, r * 50.0);
    }

    pub fn set_environment(&mut self, name: String, width: u32, height: u32, rgb: &[f32]) -> Result<(), JsValue> {
        self.gpu.set_environment(width, height, rgb).map_err(|e| JsValue::from_str(&e))?;
        self.environment = Some(name);
        Ok(())
    }

    pub fn clear_environment(&mut self) {
        self.environment = None;
    }

    pub fn set_environment_view(&mut self, rotation_degrees: f32, blur: f32, background: bool) {
        self.env_view = [rotation_degrees.to_radians(), blur.clamp(0.0, 1.0), if background { 1.0 } else { 0.0 }];
    }

    pub fn environment_json(&self) -> String {
        serde_json::json!({
            "environment": self.environment.clone().unwrap_or_else(|| "studio".into()),
            "rotation_degrees": self.env_view[0].to_degrees(),
            "blur": self.env_view[1],
            "background": self.env_view[2] > 0.5,
            "exposure": self.exposure,
        })
        .to_string()
    }

    pub fn set_post(&mut self, json: &str) -> Result<String, JsValue> {
        let value: serde_json::Value = serde_json::from_str(json).map_err(|e| JsValue::from_str(&format!("Post effect settings are not JSON: {e}")))?;
        let object = value.as_object().ok_or_else(|| JsValue::from_str("Post effect settings must be an object"))?;
        for (key, raw) in object {
            let k = POST_KEYS.iter().position(|n| n == key).ok_or_else(|| JsValue::from_str(&format!("Unknown post effect setting {key}")))?;
            let v = raw.as_f64().ok_or_else(|| JsValue::from_str(&format!("{key} must be a number")))? as f32;
            self.post[k] = v.clamp(POST_RANGES[k].0, POST_RANGES[k].1);
        }
        Ok(self.post_json())
    }

    pub fn reset_post(&mut self) -> String {
        self.post = POST_DEFAULTS;
        self.post_json()
    }

    pub fn post_json(&self) -> String {
        let map: serde_json::Map<String, serde_json::Value> = POST_KEYS.iter().zip(self.post.iter()).map(|(k, v)| (k.to_string(), serde_json::json!(((*v as f64) * 1000.0).round() / 1000.0))).collect();
        serde_json::Value::Object(map).to_string()
    }

    fn push_env_params(&mut self) {
        let (forward, right, up, _) = self.basis();
        let t = (self.fov * 0.5).tan();
        let aspect = self.width as f32 / self.height as f32;
        let on = if self.environment.is_some() { 1.0 } else { 0.0 };
        self.gpu.env_params = [
            on, self.env_view[0], self.env_view[1], self.env_view[2],
            right[0], right[1], right[2], t * aspect,
            up[0], up[1], up[2], t,
            forward[0], forward[1], forward[2], self.exposure,
        ];
        let r = self.mesh.as_ref().map(|m| m.0.radius).unwrap_or(1.0);
        let active = self.post.iter().zip(POST_DEFAULTS.iter()).enumerate().any(|(k, (v, d))| k != 1 && k != 5 && k != 10 && (v - d).abs() > 1e-4);
        self.post_seed = (self.post_seed + 1.0) % 997.0;
        let mut p = [0.0f32; 24];
        p[..16].copy_from_slice(&self.post);
        p[10] = self.distance + self.post[10] * r;
        p[16] = (self.distance - r * 2.0).max(r * 0.01);
        p[17] = self.distance + r * 3.0;
        p[18] = self.post_seed;
        p[19] = if active { 1.0 } else { 0.0 };
        p[20] = self.width as f32;
        p[21] = self.height as f32;
        self.gpu.post_params = p;
    }

    pub fn set_view_2d(&mut self, on: bool) -> bool {
        if self.stroking {
            return false;
        }
        self.flat = if on { Some([0.5, 0.5, self.width.min(self.height) as f32 * 0.9]) } else { None };
        self.flat_cursor = None;
        self.gpu.flat = on;
        true
    }

    pub fn view_2d(&self) -> bool {
        self.flat.is_some()
    }

    pub fn uv_to_screen(&self, u: f32, v: f32) -> Vec<f32> {
        match self.flat {
            Some(f) => vec![(u - f[0]) * f[2] + self.width as f32 * 0.5, (v - f[1]) * f[2] + self.height as f32 * 0.5],
            None => Vec::new(),
        }
    }

    pub fn set_view_2d_channel(&mut self, channel: u32) {
        self.gpu.flat_channel = channel.min(3);
    }

    pub fn uv_at(&self, x: f32, y: f32) -> Vec<f32> {
        self.flat_uv(x, y).map(|uv| uv.to_vec()).unwrap_or_default()
    }

    pub fn surface_point(&self, x: f32, y: f32) -> Vec<f32> {
        if self.flat.is_some() {
            return Vec::new();
        }
        self.pick(x, y).map(|h| h.position.to_vec()).unwrap_or_default()
    }

    pub fn screen_point(&self, px: f32, py: f32, pz: f32) -> Vec<f32> {
        let (forward, right, up, eye) = self.basis();
        let d = sub([px, py, pz], eye);
        let z = dot(d, forward);
        if z <= 1e-6 {
            return Vec::new();
        }
        let t = (self.fov * 0.5).tan();
        let aspect = self.width as f32 / self.height as f32;
        let nx = dot(d, right) / (z * t * aspect);
        let ny = dot(d, up) / (z * t);
        vec![(nx + 1.0) * 0.5 * self.width as f32, (1.0 - ny) * 0.5 * self.height as f32]
    }

    pub fn frame_mesh(&mut self) {
        if self.flat.is_some() {
            self.flat = Some([0.5, 0.5, self.width.min(self.height) as f32 * 0.9]);
            return;
        }
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
        if self.relay(|e| e.set_base(r, g, b, roughness, metallic)).is_some() {
            return;
        }
        self.gpu.base = [r, g, b, roughness, metallic];
        self.dirty = true;
    }

    pub fn hover(&mut self, x: f32, y: f32) -> bool {
        if self.flat.is_some() {
            self.flat_cursor = self.flat_uv(x, y);
            return self.flat_cursor.is_some_and(|uv| (0.0..=1.0).contains(&uv[0]) && (0.0..=1.0).contains(&uv[1]));
        }
        self.cursor = self.pick(x, y);
        self.cursor.is_some()
    }

    pub fn clear_hover(&mut self) {
        self.cursor = None;
        self.flat_cursor = None;
    }

    pub fn stroke_begin(&mut self, x: f32, y: f32, pressure: f32) -> bool {
        if self.mesh.is_none() || (self.brush.mode == StrokeMode::Clone && self.clone_source.is_none()) {
            return false;
        }
        let index = self.layer_index(self.active);
        if self.layers[index].kind != LayerKind::Paint && !self.layers[index].mask && self.gpu.quick_mode != 1 {
            return false;
        }
        self.gpu.begin_stroke(index);
        let active = self.active;
        self.each_sibling(|e| {
            let i = e.layer_index(active);
            e.gpu.begin_stroke(i);
        });
        self.stroking = true;
        self.lazy = Some((x, y));
        if self.flat.is_some() {
            self.last_uv = self.flat_uv(x, y);
            self.flat_cursor = self.last_uv;
            if let Some(uv) = self.last_uv {
                self.push_uv_stamp(uv, pressure, None);
            }
            return true;
        }
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
        if self.flat.is_some() {
            self.flat_stroke_to(x, y, pressure);
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
            self.each_sibling(|e| e.gpu.end_stroke());
        }
        self.stroking = false;
        self.last_stamp = None;
        self.last_uv = None;
    }

    pub fn set_alpha(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<(), JsValue> {
        self.gpu.set_image(false, width, height, rgba).map_err(|e| JsValue::from_str(&e))
    }

    pub fn set_stencil(&mut self, width: u32, height: u32, rgba: &[u8]) -> Result<(), JsValue> {
        self.gpu.set_image(true, width, height, rgba).map_err(|e| JsValue::from_str(&e))
    }

    pub fn fill_layer(&mut self) -> bool {
        if let Some(r) = self.relay(|e| e.fill_layer()) {
            return r;
        }
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
        if let Some(r) = self.relay(|e| e.undo()) {
            return r;
        }
        let ok = self.gpu.undo(&self.layer_ids());
        self.dirty |= ok;
        ok
    }

    pub fn redo(&mut self) -> bool {
        if let Some(r) = self.relay(|e| e.redo()) {
            return r;
        }
        let ok = self.gpu.redo(&self.layer_ids());
        self.dirty |= ok;
        ok
    }

    pub fn add_layer(&mut self, name: String) -> u32 {
        if let Some(r) = self.relay_new(|e| e.add_layer(name.clone())) {
            return r;
        }
        let id = self.next_id;
        self.next_id += 1;
        let (at, parent) = if self.layers.is_empty() {
            (0, 0)
        } else {
            let index = self.layer_index(self.active);
            let active = &self.layers[index];
            if active.kind == LayerKind::Folder { (index, active.id) } else { (index + 1, active.parent) }
        };
        self.layers.insert(at, LayerInfo { id, name, visible: true, opacity: 1.0, blend: BlendMode::Normal, kind: LayerKind::Paint, mask: false, parent, instance: false });
        self.gpu.insert_layer(at, id);
        self.active = id;
        self.normalize();
        id
    }

    pub fn add_folder(&mut self, name: String) -> u32 {
        if let Some(r) = self.relay_new(|e| e.add_folder(name.clone())) {
            return r;
        }
        let id = self.add_layer(name);
        let index = self.layer_index(id);
        self.layers[index].kind = LayerKind::Folder;
        self.dirty = true;
        id
    }

    pub fn set_parent(&mut self, id: u32, parent: u32) -> bool {
        if let Some(r) = self.relay(|e| e.set_parent(id, parent)) {
            return r;
        }
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
        if let Some(r) = self.relay(|e| e.remove_layer(id)) {
            return r;
        }
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
            self.mask_gens.remove(d);
            self.gpu.remove_layer(index, *d);
        }
        if doomed.contains(&self.active) {
            self.active = self.layers[first.min(self.layers.len() - 1)].id;
        }
        self.normalize();
        true
    }

    pub fn move_layer(&mut self, id: u32, offset: i32) {
        if self.relay(|e| e.move_layer(id, offset)).is_some() {
            return;
        }
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
        if let Some(r) = self.relay(|e| e.update_layer(json)) {
            return r;
        }
        let mut info: LayerInfo = serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let index = self.layers.iter().position(|l| l.id == info.id).ok_or_else(|| JsValue::from_str("No layer with that id"))?;
        info.kind = self.layers[index].kind;
        info.parent = self.layers[index].parent;
        info.instance = self.layers[index].instance;
        let shared = (info.name.clone(), info.visible, info.opacity, info.blend);
        self.propagate(info.id, |e, other| {
            if let Some(layer) = e.layers.iter().find(|l| l.id == other) {
                let mut next = layer.clone();
                (next.name, next.visible, next.opacity, next.blend) = shared.clone();
                if let Ok(text) = serde_json::to_string(&next) {
                    let _ = e.update_layer(&text);
                }
            }
        });
        self.layers[index] = info;
        self.dirty = true;
        Ok(())
    }

    pub fn add_fill_layer(&mut self, name: String) -> u32 {
        if let Some(r) = self.relay_new(|e| e.add_fill_layer(name.clone())) {
            return r;
        }
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
        if let Some(r) = self.relay(|e| e.set_fill(id, json)) {
            return r;
        }
        if !self.fills.contains_key(&id) {
            return Err(JsValue::from_str("That layer is not a fill layer"));
        }
        let settings: FillSettings = serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.fills.insert(id, settings);
        self.render_fill(id);
        self.propagate(id, |e, other| {
            let _ = e.set_fill(other, json);
        });
        Ok(())
    }

    pub fn set_fill_image(&mut self, id: u32, width: u32, height: u32, rgba: &[u8]) -> Result<(), JsValue> {
        if let Some(r) = self.relay(|e| e.set_fill_image(id, width, height, rgba)) {
            return r;
        }
        self.gpu.set_pattern(id, width, height, rgba).map_err(|e| JsValue::from_str(&e))?;
        self.render_fill(id);
        Ok(())
    }

    pub fn set_mask(&mut self, id: u32, enabled: bool) {
        if self.relay(|e| e.set_mask(id, enabled)).is_some() {
            return;
        }
        let index = self.layer_index(id);
        self.layers[index].mask = enabled;
        if !enabled {
            self.mask_gens.remove(&id);
        }
        self.dirty = true;
    }

    pub fn fill_mask(&mut self, id: u32, white: bool) {
        if self.relay(|e| e.fill_mask(id, white)).is_some() {
            return;
        }
        self.mask_gens.remove(&id);
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
        if self.relay(|e| e.select_layer(id)).is_some() {
            return;
        }
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
        self.push_env_params();
        if self.flat.is_some() {
            let cursor = self.flat_cursor.map(|uv| ([uv[0], uv[1], 0.0], [0.0, 0.0, 1.0], self.flat_radius(1.0)));
            self.gpu.render(self.flat_matrix(), eye, cursor, self.exposure, self.normal_strength);
            return;
        }
        let cursor = self.cursor.map(|h| (h.position, h.normal, self.world_radius(&h, 1.0)));
        self.gpu.render(view_proj, eye, cursor, self.exposure, self.normal_strength);
    }

    pub fn export_channel(&mut self, channel: u32) -> js_sys::Promise {
        self.export_any(channel, false)
    }

    pub fn export_channel_wide(&mut self, channel: u32) -> js_sys::Promise {
        self.export_any(channel, true)
    }

    pub fn polygon_fill(&mut self, x: f32, y: f32, mode: u32) -> Result<u32, JsValue> {
        if self.stroking {
            return Err(JsValue::from_str("Finish the stroke first"));
        }
        let info = &self.sets[self.set_index].info;
        let (first, last) = (info.start / 3, (info.start + info.count) / 3);
        let triangle = if let Some(uv) = self.flat_uv(x, y) {
            self.uv_triangle(uv, first, last).ok_or_else(|| JsValue::from_str("Click inside a UV island to fill"))?
        } else {
            let hit = self.pick(x, y).ok_or_else(|| JsValue::from_str("Click on the mesh to fill"))?;
            if hit.triangle < first || hit.triangle >= last {
                return Err(JsValue::from_str("That part belongs to another texture set; select it first"));
            }
            hit.triangle
        };
        let index = self.layer_index(self.active);
        if self.layers[index].kind != LayerKind::Paint && !self.layers[index].mask && self.gpu.quick_mode != 1 {
            return Err(JsValue::from_str("Fill, filter and folder layers are filled through their mask: choose a mask first"));
        }
        let (mesh, _) = self.mesh.as_ref().unwrap();
        let region = fs_core::regions::select(mesh, triangle, first, last, fs_core::regions::Mode::from_code(mode));
        self.gpu.begin_stroke(index);
        self.gpu.mask_triangles(&mesh.indices, &region);
        let (forward, right, up, eye) = self.basis();
        let view_proj = gpu::view_projection(eye, forward, right, up, self.fov, 1.0, 0.1, 10.0);
        let to_mask = self.paints_mask();
        self.gpu.paint(&[], view_proj, eye, &self.brush, &self.channels, index, to_mask);
        self.gpu.end_stroke();
        let active = self.active;
        self.each_sibling(|e| {
            let i = e.layer_index(active);
            e.gpu.begin_stroke(i);
            e.gpu.end_stroke();
        });
        self.dirty = true;
        Ok(region.len() as u32)
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

    pub fn set_cage(&mut self, positions: &[f32], indices: &[u32]) -> Result<String, JsValue> {
        if self.mesh.is_none() {
            return Err(JsValue::from_str("Load the low-poly mesh first"));
        }
        let triangles = self.tri_order.len();
        if indices.len() != triangles * 3 {
            return Err(JsValue::from_str(&format!("The cage has {} triangles and the low-poly mesh {triangles}; a custom cage must be the low-poly mesh with its vertices moved", indices.len() / 3)));
        }
        let vertex = |k: u32| -> Result<[f32; 3], JsValue> {
            let o = k as usize * 3;
            positions.get(o..o + 3).map(|p| [p[0], p[1], p[2]]).ok_or_else(|| JsValue::from_str("Cage index out of range"))
        };
        let mut corners = vec![0.0f32; triangles * 9];
        for (k, original) in self.tri_order.iter().enumerate() {
            for c in 0..3 {
                let p = vertex(indices[*original as usize * 3 + c])?;
                corners[k * 9 + c * 3..k * 9 + c * 3 + 3].copy_from_slice(&p);
            }
        }
        let mut offset = 0.0f32;
        let (low, _) = self.mesh.as_ref().ok_or_else(|| JsValue::from_str("Load the low-poly mesh first"))?;
        for (k, tri) in low.indices.chunks_exact(3).enumerate() {
            for c in 0..3 {
                let a = low.positions[tri[c] as usize];
                let d = [corners[k * 9 + c * 3] - a[0], corners[k * 9 + c * 3 + 1] - a[1], corners[k * 9 + c * 3 + 2] - a[2]];
                offset = offset.max(dot(d, d).sqrt());
            }
        }
        self.cage = Some(corners);
        Ok(format!("Custom cage set: {triangles} triangles, at most {:.3} of the mesh radius from the low-poly surface", offset / low.radius.max(1e-6)))
    }

    pub fn clear_cage(&mut self) {
        self.cage = None;
    }

    pub fn has_cage(&self) -> bool {
        self.cage.is_some()
    }

    pub fn texture_set_cage(&self, index: u32) -> js_sys::Float32Array {
        match (&self.cage, self.sets.get(index as usize)) {
            (Some(c), Some(set)) => js_sys::Float32Array::from(&c[(set.info.start * 3) as usize..((set.info.start + set.info.count) * 3) as usize]),
            _ => js_sys::Float32Array::new_with_length(0),
        }
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
            let keys = engine.match_keys.as_ref().map(|(l, h)| (&l[(info.start / 3) as usize..((info.start + info.count) / 3) as usize], h.as_slice()));
            let cage = engine.cage.as_ref().map(|c| &c[(info.start * 3) as usize..((info.start + info.count) * 3) as usize]);
            let result = fs_core::bake::bake_high_matched(&low, high, bvh, parts, size, front.max(0.0) * r, rear.max(0.0) * r, keys, 0, cage);
            hits += result.hits;
            texels += result.texels;
            engine.gpu.set_high_bake(size as u32, &result.normal, &result.id);
            engine.id_maps.insert(engine.set_index, (size as u32, result.id.clone()));
            Ok(())
        })
        .map_err(|e| JsValue::from_str(&e))?;
        let sets = self.sets.len();
        Ok(format!("{hits} of {texels} texels hit the high-poly mesh across {sets} texture set{}", if sets == 1 { "" } else { "s" }))
    }

    pub fn set_quick_mask(&mut self, mode: u32) -> bool {
        if self.stroking {
            return false;
        }
        self.gpu.quick_mode = mode.min(2);
        true
    }

    pub fn quick_mask_mode(&self) -> u32 {
        self.gpu.quick_mode
    }

    pub fn quick_mask_fill(&mut self, white: bool) {
        self.gpu.quick_fill(if white { 1.0 } else { 0.0 });
    }

    pub fn quick_mask_invert(&mut self) {
        self.gpu.quick_invert();
    }

    pub fn texture_set_mesh(&self, index: u32) -> Result<js_sys::Array, JsValue> {
        let (mesh, _) = self.mesh.as_ref().ok_or_else(|| JsValue::from_str("Load the low-poly mesh first"))?;
        let info = &self.sets.get(index as usize).ok_or_else(|| JsValue::from_str("No such texture set"))?.info;
        let flat = |v: &[[f32; 3]]| v.iter().flat_map(|p| p.iter().copied()).collect::<Vec<f32>>();
        let uvs: Vec<f32> = mesh.uvs.iter().flat_map(|p| p.iter().copied()).collect();
        let indices = &mesh.indices[info.start as usize..(info.start + info.count) as usize];
        let out = js_sys::Array::new();
        out.push(&js_sys::Float32Array::from(&flat(&mesh.positions)[..]));
        out.push(&js_sys::Float32Array::from(&flat(&mesh.normals)[..]));
        out.push(&js_sys::Float32Array::from(&uvs[..]));
        out.push(&js_sys::Uint32Array::from(indices));
        out.push(&JsValue::from_f64(self.gpu.size.min(2048) as f64));
        Ok(out)
    }

    pub fn apply_high_bake(&mut self, index: u32, size: u32, normal: &[u8], id: &[u8]) -> Result<(), JsValue> {
        if size == 0 || normal.len() != (size * size * 4) as usize || id.len() != normal.len() {
            return Err(JsValue::from_str("Baked maps do not match their size"));
        }
        if index as usize >= self.sets.len() {
            return Err(JsValue::from_str("No such texture set"));
        }
        self.gpu.set_high_bake_for(index as usize, size, normal, id);
        self.id_maps.insert(index as usize, (size, id.to_vec()));
        self.dirty = true;
        Ok(())
    }

    pub fn is_high_baked(&self) -> bool {
        self.gpu.high_baked
    }

    pub fn is_baked(&self) -> bool {
        self.gpu.baked
    }

    pub fn add_filter_layer(&mut self, name: String) -> u32 {
        if let Some(r) = self.relay_new(|e| e.add_filter_layer(name.clone())) {
            return r;
        }
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
        if let Some(r) = self.relay(|e| e.set_filter(id, json)) {
            return r;
        }
        if !self.filters.contains_key(&id) {
            return Err(JsValue::from_str("That layer is not a filter layer"));
        }
        let settings: FilterSettings = serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        self.filters.insert(id, settings);
        self.dirty = true;
        self.propagate(id, |e, other| {
            let _ = e.set_filter(other, json);
        });
        Ok(())
    }

    pub fn instance_layer(&mut self, id: u32, target: u32) -> Result<u32, JsValue> {
        let target = target as usize;
        if self.stroking || target >= self.sets.len() || target == self.set_index {
            return Err(JsValue::from_str("Choose another texture set to instance into"));
        }
        let index = self.layers.iter().position(|l| l.id == id).ok_or_else(|| JsValue::from_str("No layer with that id"))?;
        let info = self.layers[index].clone();
        let (fill, filter, generator) = (self.fills.get(&id).copied(), self.filters.get(&id).copied(), self.mask_gens.get(&id).copied());
        if fill.is_none() && filter.is_none() {
            return Err(JsValue::from_str("Only fill and filter layers can be instanced: paint lives in each texture set's own UV space"));
        }
        let origin = self.set_index;
        let group = match self.instance_groups.iter().position(|g| g.contains(&(origin, id))) {
            Some(g) => g,
            None => {
                self.instance_groups.push(vec![(origin, id)]);
                self.instance_groups.len() - 1
            }
        };
        self.layers[index].instance = true;
        self.instancing = true;
        self.switch_set(target);
        let new_id = if fill.is_some() { self.add_fill_layer(info.name.clone()) } else { self.add_filter_layer(info.name.clone()) };
        let text = |v: serde_json::Result<String>| v.unwrap_or_default();
        if let Some(f) = fill {
            let _ = self.set_fill(new_id, &text(serde_json::to_string(&f)));
        }
        if let Some(f) = filter {
            let _ = self.set_filter(new_id, &text(serde_json::to_string(&f)));
        }
        if let Some(g) = generator {
            let _ = self.mask_generator(new_id, &text(serde_json::to_string(&g)));
        }
        let copy = LayerInfo { id: new_id, parent: 0, instance: true, ..info.clone() };
        let _ = self.update_layer(&text(serde_json::to_string(&copy)));
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == new_id) {
            layer.instance = true;
        }
        self.gpu.composite(&self.layers, &self.filters);
        self.switch_set(origin);
        self.instancing = false;
        self.instance_groups[group].push((target, new_id));
        self.dirty = true;
        Ok(new_id)
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
        self.push_env_params();
        let view_proj = if self.flat.is_some() { self.flat_matrix() } else { gpu::view_projection(eye, forward, right, up, self.fov, aspect, (self.distance - r * 2.0).max(r * 0.01), self.distance + r * 3.0) };
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

    fn flat_uv(&self, x: f32, y: f32) -> Option<[f32; 2]> {
        let f = self.flat?;
        Some([f[0] + (x - self.width as f32 * 0.5) / f[2], f[1] + (y - self.height as f32 * 0.5) / f[2]])
    }

    fn uv_triangle(&self, uv: [f32; 2], first: u32, last: u32) -> Option<u32> {
        let (mesh, _) = self.mesh.as_ref()?;
        (first..last).find(|&t| {
            let i = t as usize * 3;
            let [a, b, c] = [mesh.indices[i], mesh.indices[i + 1], mesh.indices[i + 2]].map(|v| mesh.uvs[v as usize]);
            let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            if area.abs() < 1e-12 {
                return false;
            }
            let w0 = ((b[0] - uv[0]) * (c[1] - uv[1]) - (b[1] - uv[1]) * (c[0] - uv[0])) / area;
            let w1 = ((c[0] - uv[0]) * (a[1] - uv[1]) - (c[1] - uv[1]) * (a[0] - uv[0])) / area;
            w0 >= 0.0 && w1 >= 0.0 && w0 + w1 <= 1.0
        })
    }

    fn flat_radius(&self, pressure: f32) -> f32 {
        let p = if self.brush.pressure_size { pressure.clamp(0.05, 1.0) } else { 1.0 };
        let s = self.flat.map(|f| f[2]).unwrap_or(1.0);
        (self.brush.size_px * 0.5 / s * p).max(1e-6)
    }

    fn flat_matrix(&self) -> [f32; 16] {
        let f = self.flat.unwrap_or([0.5, 0.5, 1.0]);
        let (sx, sy) = (2.0 * f[2] / self.width as f32, 2.0 * f[2] / self.height as f32);
        [sx, 0.0, 0.0, 0.0, 0.0, -sy, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -f[0] * sx, f[1] * sy, 0.5, 1.0]
    }

    fn flat_stroke_to(&mut self, x: f32, y: f32, pressure: f32) {
        let Some(uv) = self.flat_uv(x, y) else { return };
        self.flat_cursor = Some(uv);
        let Some(last) = self.last_uv else {
            self.last_uv = Some(uv);
            self.push_uv_stamp(uv, pressure, None);
            return;
        };
        let step = (self.flat_radius(pressure) * 2.0 * self.brush.spacing).max(1e-6);
        let d = [uv[0] - last[0], uv[1] - last[1]];
        let span = (d[0] * d[0] + d[1] * d[1]).sqrt();
        if span < step {
            return;
        }
        let steps = ((span / step).floor() as usize).min(512);
        for k in 1..=steps {
            let t = (k as f32 * step) / span;
            let p = [last[0] + d[0] * t, last[1] + d[1] * t];
            self.push_uv_stamp(p, pressure, Some(d));
            self.last_uv = Some(p);
        }
    }

    fn push_uv_stamp(&mut self, uv: [f32; 2], pressure: f32, direction: Option<[f32; 2]>) {
        if let (StrokeMode::Clone, None, Some(source)) = (self.brush.mode, self.gpu.clone_offset, self.clone_source) {
            let size = self.gpu.size as f32;
            self.gpu.clone_offset = Some([((source[0] - uv[0]) * size).round() as i32, ((source[1] - uv[1]) * size).round() as i32]);
        }
        let b = self.brush;
        let r = (self.flat_radius(pressure) * (1.0 - b.jitter_size.clamp(0.0, 1.0) * self.random())).max(1e-6);
        let mut angle = b.angle.to_radians();
        if b.follow_path {
            if let Some(d) = direction {
                angle += (-d[1]).atan2(d[0]);
            }
        }
        angle += (self.random() * 2.0 - 1.0) * b.jitter_angle.clamp(0.0, 1.0) * std::f32::consts::PI;
        let jp = b.jitter_position.clamp(0.0, 4.0) * r;
        let (jx, jy) = ((self.random() * 2.0 - 1.0) * jp, (self.random() * 2.0 - 1.0) * jp);
        let flow_scale = (if b.pressure_flow { pressure.clamp(0.0, 1.0) } else { 1.0 }) * (1.0 - b.jitter_flow.clamp(0.0, 1.0) * self.random());
        self.pending.push([0.0, 0.0, 0.0, r, 0.0, 0.0, 1.0, 2.0, 1.0, 0.0, 0.0, angle, uv[0] + jx, uv[1] + jy, r, flow_scale]);
        if self.pending.len() >= 256 {
            self.flush();
        }
    }

    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 40) as f32 / (1u64 << 24) as f32
    }

    fn push_stamp(&mut self, hit: &Hit, pressure: f32, direction: Option<Vec3>) {
        if let (StrokeMode::Clone, None, Some(source)) = (self.brush.mode, self.gpu.clone_offset, self.clone_source) {
            let size = self.gpu.size as f32;
            self.gpu.clone_offset = Some([((source[0] - hit.uv[0]) * size).round() as i32, ((source[1] - hit.uv[1]) * size).round() as i32]);
        }
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
        self.gpu.clone_layer = self.clone_layer.and_then(|id| self.layers.iter().position(|l| l.id == id));
        self.gpu.paint(&stamps, view_proj, eye, &self.brush, &self.channels, self.layer_index(self.active), to_mask);
        let active = self.active;
        if self.flat.is_some() {
            self.dirty = true;
            return;
        }
        self.each_sibling(|e| {
            let index = e.layer_index(active);
            let to_mask = e.paints_mask();
            e.gpu.clone_layer = e.clone_layer.and_then(|id| e.layers.iter().position(|l| l.id == id));
            e.gpu.paint(&stamps, view_proj, eye, &e.brush, &e.channels, index, to_mask);
            e.gpu.composite(&e.layers, &e.filters);
        });
        self.dirty = true;
    }

    fn paints_mask(&self) -> bool {
        let layer = &self.layers[self.layer_index(self.active)];
        layer.kind != LayerKind::Paint || (self.mask_target && layer.mask)
    }

    fn export_any(&mut self, channel: u32, wide: bool) -> js_sys::Promise {
        self.flush();
        if self.dirty {
            self.gpu.composite(&self.layers, &self.filters);
            self.dirty = false;
        }
        let (center, radius) = self.mesh.as_ref().map(|m| (m.0.center, m.0.radius)).unwrap_or(([0.0; 3], 1.0));
        self.gpu.export(channel, self.normal_strength, center, radius, wide)
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
        let gens: Vec<u32> = self.mask_gens.keys().copied().collect();
        for id in gens {
            self.render_mask_gen(id);
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
