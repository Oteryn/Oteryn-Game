//! Supported renderer modes, shared by configuration and the native options page.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphicsEngine {
    #[default]
    Direct3D12,
    Vulkan,
}
impl GraphicsEngine {
    pub const ALL: [Self; 2] = [Self::Direct3D12, Self::Vulkan];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Direct3D12 => "Direct3D 12",
            Self::Vulkan => "Vulkan",
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Antialiasing {
    #[default]
    Off,
    Msaa4,
}
impl Antialiasing {
    pub const ALL: [Self; 2] = [Self::Off, Self::Msaa4];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Msaa4 => "MSAA 4x",
        }
    }
    pub const fn samples(self) -> u32 {
        match self {
            Self::Off => 1,
            Self::Msaa4 => 4,
        }
    }
}
pub const GRAPHICS_ENGINE: GraphicsEngine = GraphicsEngine::Direct3D12;
pub const ANTIALIASING: Antialiasing = Antialiasing::Off;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PresentationMode {
    #[default]
    Synchronized,
    Unlocked,
}
