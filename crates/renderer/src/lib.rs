//! Deterministic renderer surface ownership for the bounded W5 spike.
use oteryn_foundation::ProcessGeneration;
use std::fmt::{self, Display, Formatter};

mod batch;
mod resources;

#[cfg(windows)]
mod scene_gpu;
#[cfg(windows)]
mod windows;

/// Static world atlas preparation.
pub mod world;

pub use batch::{
    AtlasImage, AtlasPage, BatchError, CellKey, CellSlot, MAX_ATLAS_DIMENSION, MAX_BATCH_QUADS,
    MAX_VIEW_PIXELS, PAGE_CELL_PX, PAGE_CELLS, PAGE_CELLS_PER_ROW, PlacedCell, QUAD_INSTANCE_BYTES,
    QuadBatches, QuadInstance, SpriteBatch, SpriteFrame, TileBatch, TileView, VERTICES_PER_QUAD,
    instance_bytes,
};
pub use oteryn_client_domain::TileCoord;
pub use resources::ResourceCache;

#[cfg(windows)]
pub use windows::WindowsRenderer;

pub const MAX_RECONFIGURE_ATTEMPTS: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceSize {
    width: u32,
    height: u32,
}

impl SurfaceSize {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub const fn width(self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> u32 {
        self.height
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfacePhase {
    Unconfigured,
    Configured,
    Suspended,
    Lost,
    Closing,
}

impl Display for SurfacePhase {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unconfigured => "unconfigured",
            Self::Configured => "configured",
            Self::Suspended => "suspended",
            Self::Lost => "lost",
            Self::Closing => "closing",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceEventKind {
    Resize,
    Suspend,
    Resume,
    Configured,
    Presented,
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Close,
}

impl Display for SurfaceEventKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Resize => "resize",
            Self::Suspend => "suspend",
            Self::Resume => "resume",
            Self::Configured => "configured",
            Self::Presented => "presented",
            Self::Timeout => "timeout",
            Self::Occluded => "occluded",
            Self::Outdated => "outdated",
            Self::Lost => "lost",
            Self::Close => "close",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceEvent {
    Resize {
        generation: ProcessGeneration,
        width: u32,
        height: u32,
    },
    Suspend {
        generation: ProcessGeneration,
    },
    Resume {
        generation: ProcessGeneration,
    },
    Configured {
        generation: ProcessGeneration,
    },
    Presented {
        generation: ProcessGeneration,
        suboptimal: bool,
    },
    Timeout {
        generation: ProcessGeneration,
    },
    Occluded {
        generation: ProcessGeneration,
    },
    Outdated {
        generation: ProcessGeneration,
    },
    Lost {
        generation: ProcessGeneration,
    },
    Close {
        generation: ProcessGeneration,
    },
}

impl SurfaceEvent {
    const fn generation(self) -> ProcessGeneration {
        match self {
            Self::Resize { generation, .. }
            | Self::Suspend { generation }
            | Self::Resume { generation }
            | Self::Configured { generation }
            | Self::Presented { generation, .. }
            | Self::Timeout { generation }
            | Self::Occluded { generation }
            | Self::Outdated { generation }
            | Self::Lost { generation }
            | Self::Close { generation } => generation,
        }
    }

    const fn kind(self) -> SurfaceEventKind {
        match self {
            Self::Resize { .. } => SurfaceEventKind::Resize,
            Self::Suspend { .. } => SurfaceEventKind::Suspend,
            Self::Resume { .. } => SurfaceEventKind::Resume,
            Self::Configured { .. } => SurfaceEventKind::Configured,
            Self::Presented { .. } => SurfaceEventKind::Presented,
            Self::Timeout { .. } => SurfaceEventKind::Timeout,
            Self::Occluded { .. } => SurfaceEventKind::Occluded,
            Self::Outdated { .. } => SurfaceEventKind::Outdated,
            Self::Lost { .. } => SurfaceEventKind::Lost,
            Self::Close { .. } => SurfaceEventKind::Close,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceDecision {
    None,
    Configure(SurfaceSize),
    Suspend,
    Present,
    PresentAndReconfigure(SurfaceSize),
    SkipTimeout,
    SkipOccluded,
    Reconfigure(SurfaceSize),
    Recreate(SurfaceSize),
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererCounter {
    PresentedFrames,
    ReconfigureAttempts,
}

impl Display for RendererCounter {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::PresentedFrames => "presented-frames",
            Self::ReconfigureAttempts => "reconfigure-attempts",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererError {
    StaleGeneration {
        expected: ProcessGeneration,
        received: ProcessGeneration,
    },
    InvalidTransition {
        phase: SurfacePhase,
        event: SurfaceEventKind,
    },
    CounterOverflow {
        counter: RendererCounter,
    },
    ReconfigureLimitExceeded {
        max_attempts: u8,
    },
    SurfaceCreation,
    AdapterUnavailable,
    DeviceRequest,
    SurfaceUnsupported,
    Validation,
    BackendFatal,
}

impl Display for RendererError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleGeneration { expected, received } => write!(
                formatter,
                "renderer rejected stale process generation {received}; current generation is {expected}"
            ),
            Self::InvalidTransition { phase, event } => {
                write!(formatter, "renderer cannot apply {event} while {phase}")
            }
            Self::CounterOverflow { counter } => {
                write!(formatter, "renderer {counter} counter reached its bound")
            }
            Self::ReconfigureLimitExceeded { max_attempts } => write!(
                formatter,
                "renderer exceeded {max_attempts} consecutive surface recovery attempts"
            ),
            Self::SurfaceCreation => formatter.write_str("renderer surface creation failed"),
            Self::AdapterUnavailable => {
                formatter.write_str("renderer found no compatible DX12 adapter")
            }
            Self::DeviceRequest => formatter.write_str("renderer device creation failed"),
            Self::SurfaceUnsupported => {
                formatter.write_str("renderer surface is unsupported by the adapter")
            }
            Self::Validation => formatter.write_str("renderer validation failed"),
            Self::BackendFatal => formatter.write_str("renderer backend failed"),
        }
    }
}

impl std::error::Error for RendererError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceState {
    process_generation: ProcessGeneration,
    phase: SurfacePhase,
    size: SurfaceSize,
    presented_frames: u64,
    reconfigure_attempts: u8,
}

impl SurfaceState {
    #[must_use]
    pub const fn new(process_generation: ProcessGeneration) -> Self {
        Self {
            process_generation,
            phase: SurfacePhase::Unconfigured,
            size: SurfaceSize::new(0, 0),
            presented_frames: 0,
            reconfigure_attempts: 0,
        }
    }

    #[must_use]
    pub const fn process_generation(&self) -> ProcessGeneration {
        self.process_generation
    }

    #[must_use]
    pub const fn phase(&self) -> SurfacePhase {
        self.phase
    }

    #[must_use]
    pub const fn size(&self) -> SurfaceSize {
        self.size
    }

    #[must_use]
    pub const fn presented_frames(&self) -> u64 {
        self.presented_frames
    }

    #[must_use]
    pub const fn reconfigure_attempts(&self) -> u8 {
        self.reconfigure_attempts
    }

    pub fn apply(&mut self, event: SurfaceEvent) -> Result<SurfaceDecision, RendererError> {
        self.require_generation(event.generation())?;
        let mut staged = self.clone();
        let decision = staged.apply_current(event)?;
        *self = staged;
        Ok(decision)
    }

    fn apply_current(&mut self, event: SurfaceEvent) -> Result<SurfaceDecision, RendererError> {
        if self.phase == SurfacePhase::Closing {
            return match event {
                SurfaceEvent::Close { .. } => Ok(SurfaceDecision::None),
                _ => Err(RendererError::InvalidTransition {
                    phase: self.phase,
                    event: event.kind(),
                }),
            };
        }

        match event {
            SurfaceEvent::Resize { width, height, .. } => self.resize(width, height),
            SurfaceEvent::Suspend { .. } => {
                self.phase = SurfacePhase::Suspended;
                Ok(SurfaceDecision::Suspend)
            }
            SurfaceEvent::Resume { .. } => {
                if self.size.is_zero() {
                    self.phase = SurfacePhase::Suspended;
                    Ok(SurfaceDecision::Suspend)
                } else {
                    self.phase = SurfacePhase::Lost;
                    self.next_recovery(SurfaceDecision::Reconfigure(self.size))
                }
            }
            SurfaceEvent::Configured { .. } => {
                if self.size.is_zero()
                    || !matches!(
                        self.phase,
                        SurfacePhase::Unconfigured | SurfacePhase::Suspended | SurfacePhase::Lost
                    )
                {
                    return Err(RendererError::InvalidTransition {
                        phase: self.phase,
                        event: SurfaceEventKind::Configured,
                    });
                }
                self.phase = SurfacePhase::Configured;
                Ok(SurfaceDecision::None)
            }
            SurfaceEvent::Presented { suboptimal, .. } => {
                self.require_presentable(SurfaceEventKind::Presented)?;
                self.presented_frames =
                    self.presented_frames
                        .checked_add(1)
                        .ok_or(RendererError::CounterOverflow {
                            counter: RendererCounter::PresentedFrames,
                        })?;
                if suboptimal {
                    self.phase = SurfacePhase::Lost;
                    self.next_recovery(SurfaceDecision::PresentAndReconfigure(self.size))
                } else {
                    self.reconfigure_attempts = 0;
                    Ok(SurfaceDecision::Present)
                }
            }
            SurfaceEvent::Timeout { .. } => {
                self.require_presentable(SurfaceEventKind::Timeout)?;
                Ok(SurfaceDecision::SkipTimeout)
            }
            SurfaceEvent::Occluded { .. } => {
                self.require_presentable(SurfaceEventKind::Occluded)?;
                Ok(SurfaceDecision::SkipOccluded)
            }
            SurfaceEvent::Outdated { .. } => {
                self.require_presentable(SurfaceEventKind::Outdated)?;
                self.phase = SurfacePhase::Lost;
                self.next_recovery(SurfaceDecision::Reconfigure(self.size))
            }
            SurfaceEvent::Lost { .. } => {
                self.require_presentable(SurfaceEventKind::Lost)?;
                self.phase = SurfacePhase::Lost;
                self.next_recovery(SurfaceDecision::Recreate(self.size))
            }
            SurfaceEvent::Close { .. } => {
                self.phase = SurfacePhase::Closing;
                Ok(SurfaceDecision::Close)
            }
        }
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<SurfaceDecision, RendererError> {
        self.size = SurfaceSize::new(width, height);
        if self.size.is_zero() {
            self.phase = SurfacePhase::Suspended;
            return Ok(SurfaceDecision::Suspend);
        }
        self.phase = SurfacePhase::Lost;
        Ok(SurfaceDecision::Configure(self.size))
    }

    fn require_presentable(&self, event: SurfaceEventKind) -> Result<(), RendererError> {
        if self.phase == SurfacePhase::Configured {
            Ok(())
        } else {
            Err(RendererError::InvalidTransition {
                phase: self.phase,
                event,
            })
        }
    }

    fn next_recovery(
        &mut self,
        decision: SurfaceDecision,
    ) -> Result<SurfaceDecision, RendererError> {
        self.reconfigure_attempts =
            self.reconfigure_attempts
                .checked_add(1)
                .ok_or(RendererError::CounterOverflow {
                    counter: RendererCounter::ReconfigureAttempts,
                })?;
        if self.reconfigure_attempts > MAX_RECONFIGURE_ATTEMPTS {
            return Err(RendererError::ReconfigureLimitExceeded {
                max_attempts: MAX_RECONFIGURE_ATTEMPTS,
            });
        }
        Ok(decision)
    }

    fn require_generation(&self, received: ProcessGeneration) -> Result<(), RendererError> {
        if received == self.process_generation {
            Ok(())
        } else {
            Err(RendererError::StaleGeneration {
                expected: self.process_generation,
                received,
            })
        }
    }
}

#[cfg(test)]
mod tests;
