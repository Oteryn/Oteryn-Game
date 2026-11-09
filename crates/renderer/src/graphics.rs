//! Supported renderer modes, shared by configuration and the native options page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsEngine {
    Direct3D12,
}
impl GraphicsEngine {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Direct3D12 => "Direct3D 12",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Antialiasing {
    Off,
}
impl Antialiasing {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "Off (pixel renderer)",
        }
    }
    pub const fn samples(self) -> u32 {
        match self {
            Self::Off => 1,
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
