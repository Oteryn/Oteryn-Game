//! Deterministic renderer surface ownership for the bounded W5 spike.
use oteryn_foundation::ProcessGeneration;
use std::fmt::{self, Display, Formatter};

mod batch;
mod resources;

#[cfg(windows)]
mod scene_gpu;
#[cfg(windows)]
mod windows;

pub use batch::{
    AtlasImage, AtlasPage, BatchError, CellKey, CellSlot, MAX_ATLAS_DIMENSION, MAX_BATCH_QUADS,
    MAX_VIEW_PIXELS, PAGE_CELL_PX, PAGE_CELLS, PAGE_CELLS_PER_ROW, PlacedCell, QUAD_INSTANCE_BYTES,
    QuadBatches, QuadInstance, SpriteBatch, SpriteFrame, TileBatch, TileCoord, TileView,
    VERTICES_PER_QUAD, instance_bytes,
};
pub use resources::ResourceCache;

#[cfg(windows)]
pub use windows::WindowsRenderer;

pub const MAX_RECONFIGURE_ATTEMPTS: u8 = 3;

/// A surface-sized, row-major RGBA readback of one presented frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl CapturedFrame {
    /// Keeps only complete physical pixels inside the supplied renderer viewport.
    /// Fractional outer pixels are discarded so a game-only capture cannot include
    /// adjacent client chrome.
    pub fn crop_physical_viewport(
        self,
        [x, y, width, height]: [f32; 4],
    ) -> Result<Self, RendererError> {
        let expected_len = self
            .width
            .checked_mul(self.height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|bytes| usize::try_from(bytes).ok())
            .ok_or(RendererError::CaptureSize)?;
        let right = x + width;
        let bottom = y + height;
        if self.rgba.len() != expected_len
            || ![x, y, width, height, right, bottom]
                .iter()
                .all(|value| value.is_finite())
            || x < 0.0
            || y < 0.0
            || width <= 0.0
            || height <= 0.0
            || right > self.width as f32
            || bottom > self.height as f32
        {
            return Err(RendererError::CaptureSize);
        }
        let left = x.ceil() as u32;
        let top = y.ceil() as u32;
        let right = right.floor() as u32;
        let bottom = bottom.floor() as u32;
        let output_width = right
            .checked_sub(left)
            .filter(|width| *width > 0)
            .ok_or(RendererError::CaptureSize)?;
        let output_height = bottom
            .checked_sub(top)
            .filter(|height| *height > 0)
            .ok_or(RendererError::CaptureSize)?;
        let source_row_bytes = usize::try_from(self.width)
            .ok()
            .and_then(|width| width.checked_mul(4))
            .ok_or(RendererError::CaptureSize)?;
        let output_row_bytes = usize::try_from(output_width)
            .ok()
            .and_then(|width| width.checked_mul(4))
            .ok_or(RendererError::CaptureSize)?;
        let output_len = output_row_bytes
            .checked_mul(
                usize::try_from(output_height).map_err(|_error| RendererError::CaptureSize)?,
            )
            .ok_or(RendererError::CaptureSize)?;
        let left_bytes = usize::try_from(left)
            .ok()
            .and_then(|left| left.checked_mul(4))
            .ok_or(RendererError::CaptureSize)?;
        let mut rgba = Vec::with_capacity(output_len);
        for row in top..bottom {
            let start = usize::try_from(row)
                .ok()
                .and_then(|row| row.checked_mul(source_row_bytes))
                .and_then(|offset| offset.checked_add(left_bytes))
                .ok_or(RendererError::CaptureSize)?;
            let end = start
                .checked_add(output_row_bytes)
                .ok_or(RendererError::CaptureSize)?;
            rgba.extend_from_slice(
                self.rgba
                    .get(start..end)
                    .ok_or(RendererError::CaptureSize)?,
            );
        }
        Ok(Self {
            width: output_width,
            height: output_height,
            rgba,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(any(windows, test))]
enum CapturedPixelOrder {
    Rgba,
    Bgra,
}

#[cfg(any(windows, test))]
fn unpack_capture_rows(
    source: &[u8],
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    order: CapturedPixelOrder,
) -> Result<Vec<u8>, RendererError> {
    let row_bytes = width.checked_mul(4).ok_or(RendererError::CaptureSize)?;
    if padded_bytes_per_row < row_bytes {
        return Err(RendererError::CaptureSize);
    }
    let source_len = padded_bytes_per_row
        .checked_mul(height)
        .and_then(|length| usize::try_from(length).ok())
        .ok_or(RendererError::CaptureSize)?;
    let output_len = row_bytes
        .checked_mul(height)
        .and_then(|length| usize::try_from(length).ok())
        .ok_or(RendererError::CaptureSize)?;
    if source.len() != source_len {
        return Err(RendererError::CaptureSize);
    }
    let row_bytes = usize::try_from(row_bytes).map_err(|_error| RendererError::CaptureSize)?;
    let padded =
        usize::try_from(padded_bytes_per_row).map_err(|_error| RendererError::CaptureSize)?;
    let mut rgba = Vec::with_capacity(output_len);
    for row in source.chunks_exact(padded) {
        let pixels = &row[..row_bytes];
        match order {
            CapturedPixelOrder::Rgba => rgba.extend_from_slice(pixels),
            CapturedPixelOrder::Bgra => {
                for pixel in pixels.chunks_exact(4) {
                    rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
                }
            }
        }
    }
    Ok(rgba)
}

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
    CaptureUnsupported,
    CaptureSize,
    CaptureMap,
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
            Self::CaptureUnsupported => {
                formatter.write_str("renderer surface does not support native frame capture")
            }
            Self::CaptureSize => formatter.write_str("renderer capture dimensions are invalid"),
            Self::CaptureMap => formatter.write_str("renderer frame readback failed"),
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

    #[test]
    fn capture_readback_removes_padding_and_converts_bgra() {
        let source = [
            3, 2, 1, 4, 7, 6, 5, 8, 0, 0, 0, 0, 0, 0, 0, 0, // row one
            11, 10, 9, 12, 15, 14, 13, 16, 0, 0, 0, 0, 0, 0, 0, 0, // row two
        ];
        assert_eq!(
            unpack_capture_rows(&source, 2, 2, 16, CapturedPixelOrder::Bgra),
            Ok(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
        );
    }

    #[test]
    fn capture_readback_rejects_truncated_or_narrow_rows() {
        assert_eq!(
            unpack_capture_rows(&[0; 15], 2, 1, 16, CapturedPixelOrder::Rgba),
            Err(RendererError::CaptureSize)
        );
        assert_eq!(
            unpack_capture_rows(&[0; 4], 2, 1, 4, CapturedPixelOrder::Rgba),
            Err(RendererError::CaptureSize)
        );
    }

    #[test]
    fn capture_crop_keeps_only_complete_physical_viewport_pixels() {
        let mut rgba = Vec::new();
        for pixel in 0_u8..12 {
            rgba.extend_from_slice(&[pixel, pixel, pixel, 255]);
        }
        let cropped = CapturedFrame {
            width: 4,
            height: 3,
            rgba,
        }
        .crop_physical_viewport([0.25, 0.25, 3.5, 2.5]);
        assert_eq!(
            cropped,
            Ok(CapturedFrame {
                width: 2,
                height: 1,
                rgba: vec![5, 5, 5, 255, 6, 6, 6, 255],
            })
        );
    }

    #[test]
    fn capture_crop_rejects_invalid_bounds_and_storage() {
        let frame = CapturedFrame {
            width: 2,
            height: 2,
            rgba: vec![0; 16],
        };
        for viewport in [
            [-1.0, 0.0, 1.0, 1.0],
            [0.0, 0.0, 3.0, 1.0],
            [0.0, 0.0, 0.5, 0.5],
            [f32::NAN, 0.0, 1.0, 1.0],
        ] {
            assert_eq!(
                frame.clone().crop_physical_viewport(viewport),
                Err(RendererError::CaptureSize)
            );
        }
        assert_eq!(
            CapturedFrame {
                width: 2,
                height: 2,
                rgba: vec![0; 15],
            }
            .crop_physical_viewport([0.0, 0.0, 2.0, 2.0]),
            Err(RendererError::CaptureSize)
        );
    }
}
