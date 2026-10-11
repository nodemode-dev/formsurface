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
}

impl BlendMode {
    pub const ALL: [BlendMode; 8] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Add,
        BlendMode::Subtract,
        BlendMode::Darken,
        BlendMode::Lighten,
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
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Projection {
    Uv,
    Triplanar,
    Planar,
    Spherical,
    Cylindrical,
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
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Generator {
    None,
    Cavity,
    Edges,
    Thin,
    Top,
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
}

impl Default for FilterSettings {
    fn default() -> Self {
        FilterSettings { brightness: 0.0, contrast: 0.0, hue: 0.0, saturation: 1.0, gamma: 1.0, roughness_offset: 0.0, metallic_offset: 0.0 }
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
        }
    }
}

impl FillSettings {
    pub fn uniform(&self, center: [f32; 3], radius: f32) -> [f32; 28] {
        let generator = match self.generator {
            Generator::None => 0.0,
            Generator::Cavity => 1.0,
            Generator::Edges => 2.0,
            Generator::Thin => 3.0,
            Generator::Top => 4.0,
        };
        let flag = |b: bool| if b { 1.0 } else { 0.0 };
        let projection = match self.projection {
            Projection::Uv => 0.0,
            Projection::Triplanar => 1.0,
            Projection::Planar => 2.0,
            Projection::Spherical => 3.0,
            Projection::Cylindrical => 4.0,
        };
        let pattern = match self.pattern {
            Pattern::Solid => 0.0,
            Pattern::Checker => 1.0,
            Pattern::Stripes => 2.0,
            Pattern::Noise => 3.0,
            Pattern::Image => 4.0,
        };
        let c = self.base_color;
        [
            c[0], c[1], c[2], flag(self.image_color && self.pattern == Pattern::Image),
            self.roughness, self.metallic, self.height, self.opacity.clamp(0.0, 1.0),
            flag(self.use_color), flag(self.use_roughness), flag(self.use_metallic), flag(self.use_height),
            projection, pattern, self.scale.max(0.01), self.rotation.to_radians(),
            self.offset_x, self.offset_y, self.sharpness.clamp(1.0, 32.0), self.axis.min(2) as f32,
            center[0], center[1], center[2], radius,
            generator, self.generator_level.clamp(0.0, 1.0), self.generator_softness.clamp(0.001, 0.5), flag(self.generator_invert),
        ]
    }
}
