//! Deterministic renderer surface ownership for the bounded W5 spike.
use oteryn_foundation::ProcessGeneration;
use std::fmt::{self, Display, Formatter};

mod resources;

#[cfg(windows)]
mod windows;

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
mod tests {
    use super::*;

    const CURRENT: ProcessGeneration = ProcessGeneration::new(7);
    const STALE: ProcessGeneration = ProcessGeneration::new(6);

    fn configured() -> SurfaceState {
        let mut state = SurfaceState::new(CURRENT);
        assert_eq!(
            state.apply(SurfaceEvent::Resize {
                generation: CURRENT,
                width: 960,
                height: 540,
            }),
            Ok(SurfaceDecision::Configure(SurfaceSize::new(960, 540)))
        );
        assert_eq!(
            state.apply(SurfaceEvent::Configured {
                generation: CURRENT,
            }),
            Ok(SurfaceDecision::None)
        );
        state
    }

    #[test]
    fn non_zero_resize_configures_and_zero_size_suspends() {
        let mut state = SurfaceState::new(CURRENT);
        assert_eq!(
            state.apply(SurfaceEvent::Resize {
                generation: CURRENT,
                width: 640,
                height: 480,
            }),
            Ok(SurfaceDecision::Configure(SurfaceSize::new(640, 480)))
        );
        assert_eq!(state.phase(), SurfacePhase::Lost);
        assert_eq!(
            state.apply(SurfaceEvent::Configured {
                generation: CURRENT,
            }),
            Ok(SurfaceDecision::None)
        );
        assert_eq!(state.phase(), SurfacePhase::Configured);
        assert_eq!(
            state.apply(SurfaceEvent::Resize {
                generation: CURRENT,
                width: 0,
                height: 480,
            }),
            Ok(SurfaceDecision::Suspend)
        );
        assert_eq!(state.phase(), SurfacePhase::Suspended);
    }

    #[test]
    fn stale_generation_is_transactional() {
        let mut state = configured();
        let before = state.clone();
        assert_eq!(
            state.apply(SurfaceEvent::Resize {
                generation: STALE,
                width: 1,
                height: 1,
            }),
            Err(RendererError::StaleGeneration {
                expected: CURRENT,
                received: STALE,
            })
        );
        assert_eq!(state, before);
    }

    #[test]
    fn timeout_and_occlusion_skip_without_mutation() {
        let mut state = configured();
        let before = state.clone();
        assert_eq!(
            state.apply(SurfaceEvent::Timeout {
                generation: CURRENT,
            }),
            Ok(SurfaceDecision::SkipTimeout)
        );
        assert_eq!(state, before);
        assert_eq!(
            state.apply(SurfaceEvent::Occluded {
                generation: CURRENT,
            }),
            Ok(SurfaceDecision::SkipOccluded)
        );
        assert_eq!(state, before);
    }

    #[test]
    fn outdated_and_lost_request_bounded_recovery() {
        let mut state = configured();
        assert_eq!(
            state.apply(SurfaceEvent::Outdated {
                generation: CURRENT,
            }),
            Ok(SurfaceDecision::Reconfigure(SurfaceSize::new(960, 540)))
        );
        assert_eq!(state.phase(), SurfacePhase::Lost);
        assert_eq!(state.reconfigure_attempts(), 1);
        assert_eq!(
            state.apply(SurfaceEvent::Configured {
                generation: CURRENT,
            }),
            Ok(SurfaceDecision::None)
        );
        assert_eq!(
            state.apply(SurfaceEvent::Lost {
                generation: CURRENT,
            }),
            Ok(SurfaceDecision::Recreate(SurfaceSize::new(960, 540)))
        );
        assert_eq!(state.reconfigure_attempts(), 2);
    }

    #[test]
    fn recovery_limit_failure_does_not_mutate() {
        let mut state = configured();
        for _ in 0..MAX_RECONFIGURE_ATTEMPTS {
            assert!(
                state
                    .apply(SurfaceEvent::Outdated {
                        generation: CURRENT,
                    })
                    .is_ok()
            );
            if state.reconfigure_attempts() < MAX_RECONFIGURE_ATTEMPTS {
                assert_eq!(
                    state.apply(SurfaceEvent::Configured {
                        generation: CURRENT,
                    }),
                    Ok(SurfaceDecision::None)
                );
            }
        }
        let before = state.clone();
        assert_eq!(
            state.apply(SurfaceEvent::Resume {
                generation: CURRENT,
            }),
            Err(RendererError::ReconfigureLimitExceeded {
                max_attempts: MAX_RECONFIGURE_ATTEMPTS,
            })
        );
        assert_eq!(state, before);
    }

    #[test]
    fn successful_present_resets_recovery_and_counts_frames() {
        let mut state = configured();
        assert_eq!(
            state.apply(SurfaceEvent::Presented {
                generation: CURRENT,
                suboptimal: false,
            }),
            Ok(SurfaceDecision::Present)
        );
        assert_eq!(state.presented_frames(), 1);
        assert_eq!(state.reconfigure_attempts(), 0);
    }

    #[test]
    fn suboptimal_frame_presents_then_requests_reconfiguration() {
        let mut state = configured();
        assert_eq!(
            state.apply(SurfaceEvent::Presented {
                generation: CURRENT,
                suboptimal: true,
            }),
            Ok(SurfaceDecision::PresentAndReconfigure(SurfaceSize::new(
                960, 540
            )))
        );
        assert_eq!(state.phase(), SurfacePhase::Lost);
        assert_eq!(state.presented_frames(), 1);
    }

    #[test]
    fn suspend_resume_uses_latest_non_zero_size() {
        let mut state = configured();
        assert_eq!(
            state.apply(SurfaceEvent::Suspend {
                generation: CURRENT,
            }),
            Ok(SurfaceDecision::Suspend)
        );
        assert_eq!(
            state.apply(SurfaceEvent::Resume {
                generation: CURRENT,
            }),
            Ok(SurfaceDecision::Reconfigure(SurfaceSize::new(960, 540)))
        );
    }

    #[test]
    fn presented_frame_overflow_fails_without_mutation() {
        let mut state = configured();
        state.presented_frames = u64::MAX;
        let before = state.clone();
        assert_eq!(
            state.apply(SurfaceEvent::Presented {
                generation: CURRENT,
                suboptimal: false,
            }),
            Err(RendererError::CounterOverflow {
                counter: RendererCounter::PresentedFrames,
            })
        );
        assert_eq!(state, before);
    }

    #[test]
    fn close_is_idempotent_and_rejects_presentation() {
        let mut state = configured();
        assert_eq!(
            state.apply(SurfaceEvent::Close {
                generation: CURRENT,
            }),
            Ok(SurfaceDecision::Close)
        );
        assert_eq!(
            state.apply(SurfaceEvent::Close {
                generation: CURRENT,
            }),
            Ok(SurfaceDecision::None)
        );
        assert_eq!(
            state.apply(SurfaceEvent::Presented {
                generation: CURRENT,
                suboptimal: false,
            }),
            Err(RendererError::InvalidTransition {
                phase: SurfacePhase::Closing,
                event: SurfaceEventKind::Presented,
            })
        );
    }
}
