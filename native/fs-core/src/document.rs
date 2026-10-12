use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Add,
    Subtract,
    Darken,
    Lighten,
    Disable,
    Replace,
    Divide,
    InverseDivide,
    Difference,
    Exclusion,
    SignedAddition,
    LinearBurn,
    ColorBurn,
    ColorDodge,
    SoftLight,
    HardLight,
    VividLight,
    LinearLight,
    PinLight,
    Tint,
    Saturation,
    Color,
    Value,
    NormalMapCombine,
    NormalMapDetail,
    NormalMapInverseDetail,
    InverseSubtract,
}

impl BlendMode {
    pub const ALL: [BlendMode; 31] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Add,
        BlendMode::Subtract,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::Disable,
        BlendMode::Replace,
        BlendMode::Divide,
        BlendMode::InverseDivide,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::SignedAddition,
        BlendMode::LinearBurn,
        BlendMode::ColorBurn,
        BlendMode::ColorDodge,
        BlendMode::SoftLight,
        BlendMode::HardLight,
        BlendMode::VividLight,
        BlendMode::LinearLight,
        BlendMode::PinLight,
        BlendMode::Tint,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Value,
        BlendMode::NormalMapCombine,
        BlendMode::NormalMapDetail,
        BlendMode::NormalMapInverseDetail,
        BlendMode::InverseSubtract,
    ];

    pub fn code(self) -> u32 {
        BlendMode::ALL.iter().position(|m| *m == self).unwrap_or(0) as u32
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StrokeMode {
    Paint,
    Erase,
    Clone,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ChannelValues {
    pub base_color: [f32; 3],
    pub roughness: f32,
    pub metallic: f32,
    pub height: f32,
    pub use_color: bool,
    pub use_roughness: bool,
    pub use_metallic: bool,
    pub use_height: bool,
}

impl Default for ChannelValues {
    fn default() -> Self {
        ChannelValues {
            base_color: [0.72, 0.18, 0.12],
            roughness: 0.45,
            metallic: 0.0,
            height: 0.5,
            use_color: true,
            use_roughness: true,
            use_metallic: true,
            use_height: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    Camera,
    TangentWrap,
    TangentPlanar,
    Uv,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SizeSpace {
    Screen,
    Object,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tip {
    Round,
    Square,
    Spray,
    Image,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Symmetry {
    Off,
    Mirror,
    Radial,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BrushSettings {
    pub size_px: f32,
    pub hardness: f32,
    pub flow: f32,
    pub opacity: f32,
    pub spacing: f32,
    pub mode: StrokeMode,
    pub mirror_x: bool,
    pub backface_culling: bool,
    pub pressure_size: bool,
    pub pressure_flow: bool,
    pub alignment: Alignment,
    pub size_space: SizeSpace,
    pub angle: f32,
    pub follow_path: bool,
    pub jitter_size: f32,
    pub jitter_angle: f32,
    pub jitter_position: f32,
    pub jitter_flow: f32,
    pub lazy_px: f32,
    pub tip: Tip,
    pub spray_density: f32,
    pub symmetry: Symmetry,
    pub symmetry_axis: u32,
    pub radial_count: u32,
    pub stencil: bool,
    pub stencil_scale: f32,
    pub stencil_angle: f32,
    pub stencil_x: f32,
    pub stencil_y: f32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        BrushSettings {
            size_px: 40.0,
            hardness: 0.6,
            flow: 0.6,
            opacity: 1.0,
            spacing: 0.12,
            mode: StrokeMode::Paint,
            mirror_x: false,
            backface_culling: true,
            pressure_size: true,
            pressure_flow: false,
            alignment: Alignment::TangentWrap,
            size_space: SizeSpace::Screen,
            angle: 0.0,
            follow_path: false,
            jitter_size: 0.0,
            jitter_angle: 0.0,
            jitter_position: 0.0,
            jitter_flow: 0.0,
            lazy_px: 0.0,
            tip: Tip::Round,
            spray_density: 0.35,
            symmetry: Symmetry::Off,
            symmetry_axis: 0,
            radial_count: 6,
            stencil: false,
            stencil_scale: 1.0,
            stencil_angle: 0.0,
            stencil_x: 0.0,
            stencil_y: 0.0,
        }
    }
}

impl Alignment {
    pub fn code(self) -> f32 {
        match self {
            Alignment::Camera => 0.0,
            Alignment::TangentWrap => 1.0,
            Alignment::TangentPlanar => 2.0,
            Alignment::Uv => 3.0,
        }
    }
}

impl Tip {
    pub fn code(self) -> f32 {
        match self {
            Tip::Round => 0.0,
            Tip::Square => 1.0,
            Tip::Spray => 2.0,
            Tip::Image => 3.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    #[default]
    Paint,
    Fill,
    Filter,
    Folder,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayerInfo {
    pub id: u32,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend: BlendMode,
    #[serde(default)]
    pub kind: LayerKind,
    #[serde(default)]
    pub mask: bool,
    #[serde(default)]
    pub parent: u32,
    #[serde(default)]
    pub instance: bool,
}

pub fn tree_order(layers: &[LayerInfo]) -> Vec<LayerInfo> {
    fn emit(parent: u32, layers: &[LayerInfo], out: &mut Vec<LayerInfo>, depth: usize) {
        for layer in layers.iter().filter(|l| l.parent == parent) {
            if layer.kind == LayerKind::Folder && depth < 16 {
                emit(layer.id, layers, out, depth + 1);
            }
            out.push(layer.clone());
        }
    }
    let ids: std::collections::HashSet<u32> = layers.iter().filter(|l| l.kind == LayerKind::Folder).map(|l| l.id).collect();
    let fixed: Vec<LayerInfo> = layers.iter().map(|l| LayerInfo { parent: if ids.contains(&l.parent) && l.parent != l.id { l.parent } else { 0 }, ..l.clone() }).collect();
    let mut out = Vec::with_capacity(fixed.len());
    emit(0, &fixed, &mut out, 0);
    for layer in &fixed {
        if !out.iter().any(|o| o.id == layer.id) {
            out.push(LayerInfo { parent: 0, ..layer.clone() });
        }
    }
    out
}

pub fn ancestors(layers: &[LayerInfo], id: u32) -> Vec<usize> {
    let mut out = Vec::new();
    let mut parent = layers.iter().find(|l| l.id == id).map(|l| l.parent).unwrap_or(0);
    while parent != 0 && out.len() < 16 {
        let Some(index) = layers.iter().position(|l| l.id == parent) else { break };
        out.push(index);
        parent = layers[index].parent;
    }
    out
}

pub fn descendants(layers: &[LayerInfo], id: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut frontier = vec![id];
    while let Some(p) = frontier.pop() {
        for layer in layers.iter().filter(|l| l.parent == p) {
            out.push(layer.id);
            if layer.kind == LayerKind::Folder && out.len() < 4096 {
                frontier.push(layer.id);
            }
        }
    }
    out
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Projection {
    Uv,
    Triplanar,
    Planar,
    Spherical,
    Cylindrical,
    Warp,
}

pub const WARP_SIDE: usize = 4;

pub fn warp_rest() -> [f32; 32] {
    let mut grid = [0.0; 32];
    for j in 0..WARP_SIDE {
        for i in 0..WARP_SIDE {
            let k = (j * WARP_SIDE + i) * 2;
            grid[k] = i as f32 / (WARP_SIDE - 1) as f32;
            grid[k + 1] = j as f32 / (WARP_SIDE - 1) as f32;
        }
    }
    grid
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Pattern {
    Solid,
    Checker,
    Stripes,
    Noise,
    Image,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FillSettings {
    pub projection: Projection,
    pub pattern: Pattern,
    pub scale: f32,
    pub rotation: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub sharpness: f32,
    pub axis: u32,
    pub base_color: [f32; 3],
    pub roughness: f32,
    pub metallic: f32,
    pub height: f32,
    pub use_color: bool,
    pub use_roughness: bool,
    pub use_metallic: bool,
    pub use_height: bool,
    pub image_color: bool,
    pub opacity: f32,
    pub generator: Generator,
    pub generator_level: f32,
    pub generator_softness: f32,
    pub generator_invert: bool,
    pub generator_direction: [f32; 3],
    pub generator_noise: f32,
    pub noise_scale: f32,
    pub builder_ao: f32,
    pub builder_curvature: f32,
    pub builder_noise: f32,
    pub warp: [f32; 32],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Generator {
    None,
    Cavity,
    Edges,
    Thin,
    Top,
    Light,
    Position,
    Noise,
    MaskBuilder,
    Dirt,
    MetalEdgeWear,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FilterSettings {
    pub brightness: f32,
    pub contrast: f32,
    pub hue: f32,
    pub saturation: f32,
    pub gamma: f32,
    pub roughness_offset: f32,
    pub metallic_offset: f32,
    pub blur: f32,
    pub sharpen: f32,
    pub levels_in_low: f32,
    pub levels_in_high: f32,
    pub levels_out_low: f32,
    pub levels_out_high: f32,
    pub invert: bool,
    pub posterize: f32,
    pub pixelate: f32,
}

impl Default for FilterSettings {
    fn default() -> Self {
        FilterSettings {
            brightness: 0.0, contrast: 0.0, hue: 0.0, saturation: 1.0, gamma: 1.0, roughness_offset: 0.0, metallic_offset: 0.0, blur: 0.0, sharpen: 0.0,
            levels_in_low: 0.0, levels_in_high: 1.0, levels_out_low: 0.0, levels_out_high: 1.0, invert: false, posterize: 0.0, pixelate: 0.0,
        }
    }
}

impl Default for FillSettings {
    fn default() -> Self {
        FillSettings {
            projection: Projection::Uv,
            pattern: Pattern::Solid,
            scale: 8.0,
            rotation: 0.0,
            offset_x: 0.0,
            offset_y: 0.0,
            sharpness: 4.0,
            axis: 2,
            base_color: [0.18, 0.2, 0.24],
            roughness: 0.55,
            metallic: 0.0,
            height: 0.5,
            use_color: true,
            use_roughness: true,
            use_metallic: true,
            use_height: false,
            image_color: false,
            opacity: 1.0,
            generator: Generator::None,
            generator_level: 0.5,
            generator_softness: 0.1,
            generator_invert: false,
            generator_direction: [0.5, 0.8, 0.3],
            generator_noise: 0.0,
            noise_scale: 4.0,
            builder_ao: 1.0,
            builder_curvature: 0.0,
            builder_noise: 0.0,
            warp: warp_rest(),
        }
    }
}

impl FillSettings {
    pub fn uniform(&self, center: [f32; 3], radius: f32) -> [f32; 68] {
        let generator = match self.generator {
            Generator::None => 0.0,
            Generator::Cavity => 1.0,
            Generator::Edges => 2.0,
            Generator::Thin => 3.0,
            Generator::Top => 4.0,
            Generator::Light => 5.0,
            Generator::Position => 6.0,
            Generator::Noise => 7.0,
            Generator::MaskBuilder => 8.0,
            Generator::Dirt => 9.0,
            Generator::MetalEdgeWear => 10.0,
        };
        let d = self.generator_direction;
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-6);
        let flag = |b: bool| if b { 1.0 } else { 0.0 };
        let projection = match self.projection {
            Projection::Uv => 0.0,
            Projection::Triplanar => 1.0,
            Projection::Planar => 2.0,
            Projection::Spherical => 3.0,
            Projection::Cylindrical => 4.0,
            Projection::Warp => 5.0,
        };
        let pattern = match self.pattern {
            Pattern::Solid => 0.0,
            Pattern::Checker => 1.0,
            Pattern::Stripes => 2.0,
            Pattern::Noise => 3.0,
            Pattern::Image => 4.0,
        };
        let c = self.base_color;
        let head = [
            c[0], c[1], c[2], flag(self.image_color && self.pattern == Pattern::Image),
            self.roughness, self.metallic, self.height, self.opacity.clamp(0.0, 1.0),
            flag(self.use_color), flag(self.use_roughness), flag(self.use_metallic), flag(self.use_height),
            projection, pattern, self.scale.max(0.01), self.rotation.to_radians(),
            self.offset_x, self.offset_y, self.sharpness.clamp(1.0, 32.0), self.axis.min(2) as f32,
            center[0], center[1], center[2], radius,
            generator, self.generator_level.clamp(0.0, 1.0), self.generator_softness.clamp(0.001, 0.5), flag(self.generator_invert),
            self.builder_ao.clamp(0.0, 1.0), self.builder_curvature.clamp(0.0, 1.0), self.builder_noise.clamp(0.0, 1.0), self.noise_scale.clamp(0.1, 64.0),
            d[0] / len, d[1] / len, d[2] / len, self.generator_noise.clamp(0.0, 1.0),
        ];
        let mut out = [0.0; 68];
        out[..36].copy_from_slice(&head);
        for (o, w) in out[36..].iter_mut().zip(self.warp.iter()) {
            *o = if w.is_finite() { w.clamp(-2.0, 3.0) } else { 0.0 };
        }
        out
    }
}
