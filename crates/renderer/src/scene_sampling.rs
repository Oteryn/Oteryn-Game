//! Whole-scene sampling, independent of atlas filtering and gameplay coordinates.
use crate::RendererError;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SceneSampling {
    #[default]
    None,
    Antialiasing,
    SmoothRetro,
}

impl SceneSampling {
    /// Dimensions of the source image; `None` bypasses resampling at native size.
    pub fn source_size(
        self,
        native: [u32; 2],
        destination: [f32; 2],
        maximum: u32,
    ) -> Result<Option<[u32; 2]>, RendererError> {
        if native.contains(&0) || destination.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err(RendererError::Validation);
        }
        if destination == native.map(|v| v as f32) {
            return Ok(None);
        }
        let scale = destination[1] / native[1] as f32;
        let multiple = if self == Self::SmoothRetro && scale > 1.0 {
            scale.ceil()
        } else {
            1.0
        };
        let size = native.map(|v| v as f32 * multiple);
        if size.iter().any(|v| *v > maximum as f32) {
            return Err(RendererError::Validation);
        }
        Ok(Some(size.map(|v| v as u32)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retro_uses_the_original_ceil_scale_not_fixed_double_size() -> Result<(), RendererError> {
        for (destination, expected) in [
            ([240.0, 176.0], Some([480, 352])),
            ([480.0, 352.0], None),
            ([720.0, 528.0], Some([960, 704])),
            ([960.0, 704.0], Some([960, 704])),
            ([1200.0, 880.0], Some([1440, 1056])),
        ] {
            assert_eq!(
                SceneSampling::SmoothRetro.source_size([480, 352], destination, 4096)?,
                expected
            );
        }
        for mode in [SceneSampling::None, SceneSampling::Antialiasing] {
            assert_eq!(
                mode.source_size([480, 352], [1200.0, 880.0], 4096)?,
                Some([480, 352])
            );
        }
        Ok(())
    }

    #[test]
    fn invalid_or_over_budget_sources_are_rejected() {
        for destination in [
            [0.0, 1.0],
            [1.0, -1.0],
            [f32::NAN, 1.0],
            [1.0, f32::INFINITY],
        ] {
            assert!(
                SceneSampling::None
                    .source_size([480, 352], destination, 4096)
                    .is_err()
            );
        }
        assert!(
            SceneSampling::SmoothRetro
                .source_size([480, 352], [6000.0, 4400.0], 4096)
                .is_err()
        );
        assert!(
            SceneSampling::None
                .source_size([0, 352], [480.0, 352.0], 4096)
                .is_err()
        );
    }
}
