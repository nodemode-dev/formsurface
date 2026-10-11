use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, Serialize)]
pub struct TextureSet {
    pub material: String,
    pub udim: u32,
    pub tiled: bool,
    pub start: u32,
    pub count: u32,
}

impl TextureSet {
    pub fn name(&self) -> String {
        if self.tiled { format!("{}.{}", self.material, self.udim) } else { self.material.clone() }
    }
}

pub struct SplitMesh {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub uvs: Vec<f32>,
    pub indices: Vec<u32>,
    pub sets: Vec<TextureSet>,
}

pub fn udim_of(u: f32, v_stored: f32) -> (u32, u32) {
    let col = u.floor().clamp(0.0, 9.0) as u32;
    let row = (1.0 - v_stored).floor().clamp(0.0, 99.0) as u32;
    (col, row)
}

pub fn split(positions: &[f32], normals: &[f32], uvs: &[f32], indices: &[u32], materials: &[u32], names: &[String]) -> Result<SplitMesh, String> {
    let count = positions.len() / 3;
    if positions.is_empty() || positions.len() % 3 != 0 {
        return Err("Mesh positions must be a non-empty multiple of 3".into());
    }
    if uvs.len() != count * 2 {
        return Err("This mesh has no UV coordinates. FormSurface paints into UV textures, so the mesh needs a UV layout.".into());
    }
    let indices: Vec<u32> = if indices.is_empty() { (0..count as u32).collect() } else { indices.to_vec() };
    if indices.len() % 3 != 0 || indices.iter().any(|&i| i as usize >= count) {
        return Err("Mesh indices are out of range".into());
    }
    let triangles = indices.len() / 3;
    let has_normals = normals.len() == count * 3;
    let mut groups: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    let mut tiles: Vec<(u32, u32)> = Vec::with_capacity(triangles);
    for t in 0..triangles {
        let tri = &indices[t * 3..t * 3 + 3];
        let (mut u, mut v) = (0.0, 0.0);
        for &i in tri {
            u += uvs[i as usize * 2] / 3.0;
            v += uvs[i as usize * 2 + 1] / 3.0;
        }
        let (col, row) = udim_of(u, v);
        tiles.push((col, row));
        let material = materials.get(t).copied().filter(|_| materials.len() == triangles).unwrap_or(0);
        groups.entry((material, 1001 + col + 10 * row)).or_default().push(t);
    }
    let mut tiled_materials = std::collections::BTreeSet::new();
    for (material, udim) in groups.keys() {
        if *udim != 1001 {
            tiled_materials.insert(*material);
        }
    }
    let mut out = SplitMesh { positions: Vec::new(), normals: Vec::new(), uvs: Vec::new(), indices: Vec::with_capacity(indices.len()), sets: Vec::new() };
    for ((material, udim), tris) in groups {
        let mut remap: HashMap<u32, u32> = HashMap::new();
        let start = out.indices.len() as u32;
        for t in tris {
            let (col, row) = tiles[t];
            for &i in &indices[t * 3..t * 3 + 3] {
                let next = (out.positions.len() / 3) as u32;
                let local = *remap.entry(i).or_insert_with(|| {
                    let k = i as usize;
                    out.positions.extend_from_slice(&positions[k * 3..k * 3 + 3]);
                    if has_normals {
                        out.normals.extend_from_slice(&normals[k * 3..k * 3 + 3]);
                    }
                    out.uvs.extend_from_slice(&[uvs[k * 2] - col as f32, uvs[k * 2 + 1] + row as f32]);
                    next
                });
                out.indices.push(local);
            }
        }
        let name = names.get(material as usize).filter(|n| !n.trim().is_empty()).cloned().unwrap_or_else(|| format!("material_{material}"));
        out.sets.push(TextureSet { material: name, udim, tiled: tiled_materials.contains(&material), start, count: out.indices.len() as u32 - start });
    }
    Ok(out)
}
