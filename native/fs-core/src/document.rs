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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayerInfo {
    pub id: u32,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend: BlendMode,
}
