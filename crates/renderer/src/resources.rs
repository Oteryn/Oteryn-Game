#[cfg(any(windows, test))]
use crate::RendererError;
#[cfg(any(windows, test))]
use oteryn_foundation::ProcessGeneration;
// Keep physical acquisition behind the same testable entry boundary on every backend.
#[cfg(any(windows, test))]
pub(crate) fn acquire_frame<R>(
    state: &crate::SurfaceState,
    generation: ProcessGeneration,
    acquire: impl FnOnce() -> R,
) -> Result<R, RendererError> {
    if generation != state.process_generation() {
        return Err(RendererError::StaleGeneration {
            expected: state.process_generation(),
            received: generation,
        });
    }
    state.require_presentable(crate::SurfaceEventKind::Presented)?;
    Ok(acquire())
}

#[cfg(test)]
mod acquisition_tests {
    use super::acquire_frame;
    use crate::{RendererError, SurfaceEvent, SurfaceState};
    use oteryn_foundation::ProcessGeneration;
    use std::cell::Cell;

    const CURRENT: ProcessGeneration = ProcessGeneration::new(7);
    const STALE: ProcessGeneration = ProcessGeneration::new(6);

    fn configured() -> Result<SurfaceState, RendererError> {
        let mut state = SurfaceState::new(CURRENT);
        state.apply(SurfaceEvent::Resize {
            generation: CURRENT,
            width: 640,
            height: 480,
        })?;
        state.apply(SurfaceEvent::Configured {
            generation: CURRENT,
        })?;
        Ok(state)
    }

    #[test]
    fn stale_generation_never_invokes_frame_acquisition() -> Result<(), RendererError> {
        let state = configured()?;
        let before = state.clone();
        let acquisitions = Cell::new(0);
        let result = acquire_frame(&state, STALE, || {
            acquisitions.set(acquisitions.get() + 1);
            42_u8
        });
        assert_eq!(
            result,
            Err(RendererError::StaleGeneration {
                expected: CURRENT,
                received: STALE,
            })
        );
        assert_eq!(acquisitions.get(), 0);
        assert_eq!(state, before);
        Ok(())
    }

    #[test]
    fn current_generation_acquires_once_without_advancing_state() -> Result<(), RendererError> {
        let state = configured()?;
        let before = state.clone();
        let acquisitions = Cell::new(0);
        let frame = acquire_frame(&state, CURRENT, || {
            acquisitions.set(acquisitions.get() + 1);
            42_u8
        })?;
        assert_eq!(frame, 42);
        assert_eq!(acquisitions.get(), 1);
        assert_eq!(state, before);
        Ok(())
    }

    #[test]
    fn every_non_presentable_phase_blocks_acquisition() -> Result<(), RendererError> {
        let mut suspended = configured()?;
        suspended.apply(SurfaceEvent::Suspend {
            generation: CURRENT,
        })?;
        let mut lost = configured()?;
        lost.apply(SurfaceEvent::Lost {
            generation: CURRENT,
        })?;
        let mut closing = configured()?;
        closing.apply(SurfaceEvent::Close {
            generation: CURRENT,
        })?;
        for state in [SurfaceState::new(CURRENT), suspended, lost, closing] {
            let acquisitions = Cell::new(0);
            assert!(matches!(
                acquire_frame(&state, CURRENT, || acquisitions.set(1)),
                Err(RendererError::InvalidTransition { .. })
            ));
            assert_eq!(acquisitions.get(), 0);
        }
        Ok(())
    }

    #[test]
    fn generation_denial_precedes_phase_denial() {
        let state = SurfaceState::new(CURRENT);
        let acquisitions = Cell::new(0);
        assert_eq!(
            acquire_frame(&state, STALE, || acquisitions.set(1)),
            Err(RendererError::StaleGeneration {
                expected: CURRENT,
                received: STALE,
            })
        );
        assert_eq!(acquisitions.get(), 0);
    }
}
