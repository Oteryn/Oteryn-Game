use super::{ProcessGeneration, SurfaceEventKind, SurfacePhase};
use std::fmt::{self, Display, Formatter};

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
                formatter.write_str("renderer found no compatible adapter for the selected engine")
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
