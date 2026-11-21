//! Client presentation of server-owned creature lights. No gameplay timers live here.
use crate::{QUAD_INSTANCE_BYTES, QuadInstance};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuadLayer {
    Terrain,
    Overlay,
}

pub const LIT_INSTANCE_BYTES: usize = QUAD_INSTANCE_BYTES + 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    pub position: [f32; 2],
    pub radius: f32,
    pub color: [u8; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneLighting {
    /// Per-frame overlay ranges and their opacity percentage; later ranges take precedence.
    pub opacity: Vec<(std::ops::Range<usize>, u8)>,
    pub ambient: [u8; 3],
    pub sources: Vec<PointLight>,
    /// Emissive UI palette, in atlas UV coordinates.
    pub palette: Option<[f32; 4]>,
    /// Transient overlays appended after world sprites use their original colors.
    pub emissive_from: Option<usize>,
    /// Dimming of bounded painter-order ranges (lower floors); UI quads lie outside them.
    pub dimmed: Vec<(std::ops::Range<usize>, u8)>,
}

impl Default for SceneLighting {
    fn default() -> Self {
        Self {
            opacity: Vec::new(),
            ambient: [255; 3],
            sources: Vec::new(),
            palette: None,
            emissive_from: None,
            dimmed: Vec::new(),
        }
    }
}

/// The indexed Tibia colour cube used by both world and creature light.
#[must_use]
pub fn light_color(index: u8) -> [u8; 3] {
    let index = u16::from(index);
    [index / 36 % 6, index / 6 % 6, index % 6].map(|v| (v * 51) as u8)
}

impl SceneLighting {
    pub fn set_opacity(
        &mut self,
        range: std::ops::Range<usize>,
        percentage: u8,
    ) -> Result<(), crate::BatchError> {
        if range.start > range.end || range.end > crate::MAX_BATCH_QUADS || percentage > 100 {
            return Err(crate::BatchError::InvalidView);
        }
        if self.opacity.len() == crate::MAX_BATCH_QUADS {
            return Err(crate::BatchError::CapacityExceeded {
                max: crate::MAX_BATCH_QUADS,
            });
        }
        if !range.is_empty() {
            self.opacity.push((range, percentage));
        }
        Ok(())
    }
    /// Maximum channel contribution, with a linear radial falloff in screen pixels.
    /// Sampling once per sprite keeps the native tile presentation deterministic.
    #[must_use]
    pub fn sample(&self, position: [f32; 2]) -> [f32; 3] {
        let mut rgb = self.ambient.map(|v| f32::from(v) / 255.0);
        for light in &self.sources {
            if !light.radius.is_finite() || light.radius <= 0.0 {
                continue;
            }
            let distance = (position[0] - light.position[0]).hypot(position[1] - light.position[1]);
            let strength = (1.0 - distance / light.radius).clamp(0.0, 1.0);
            for (channel, color) in rgb.iter_mut().zip(light.color) {
                *channel = channel.max(f32::from(color) / 255.0 * strength);
            }
        }
        rgb
    }

    /// GPU upload extends the stable quad layout with linear RGB and overlay opacity.
    /// The shader samples an sRGB texture in linear space, so convert light here.
    #[must_use]
    pub fn instance_bytes(&self, instances: &[QuadInstance]) -> Vec<u8> {
        self.upload_bytes(instances, true)
    }

    /// Terrain uses the same light samples without sprite overlay opacity ranges.
    #[must_use]
    pub fn opaque_instance_bytes(&self, instances: &[QuadInstance]) -> Vec<u8> {
        self.upload_bytes(instances, false)
    }

    /// The native GPU and alternative hosts share the exact per-quad linear tint.
    /// Terrain ignores overlay opacity; all other painter-order rules are identical.
    #[must_use]
    pub fn quad_tint(&self, quad: &QuadInstance, index: usize, layer: QuadLayer) -> [f32; 4] {
        let emissive = self.emissive_from.is_some_and(|start| index >= start)
            || self.palette.is_some_and(|p| {
                quad.uv[0] >= p[0] && quad.uv[1] >= p[1] && quad.uv[2] <= p[2] && quad.uv[3] <= p[3]
            });
        let rgb = if emissive {
            [1.0; 3]
        } else {
            self.sample([
                quad.position[0] + quad.size[0] / 2.0,
                quad.position[1] + quad.size[1] / 2.0,
            ])
        };
        let brightness = self
            .dimmed
            .iter()
            .find(|(r, _)| r.contains(&index))
            .map_or(1.0, |(_, d)| 1.0 - f32::from((*d).min(100)) / 100.0);
        let [r, g, b] = rgb.map(|value| {
            let value = value * brightness;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        });
        let alpha = self
            .opacity
            .iter()
            .rev()
            .find(|(range, _)| layer == QuadLayer::Overlay && range.contains(&index))
            .map_or(1.0, |(_, p)| f32::from((*p).min(100)) / 100.0);
        [r, g, b, alpha]
    }

    fn upload_bytes(&self, instances: &[QuadInstance], overlays: bool) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(instances.len() * LIT_INSTANCE_BYTES);
        for (index, quad) in instances.iter().enumerate() {
            bytes.extend_from_slice(&quad.to_bytes());
            let layer = if overlays {
                QuadLayer::Overlay
            } else {
                QuadLayer::Terrain
            };
            for value in self.quad_tint(quad, index, layer) {
                bytes.extend_from_slice(&value.to_ne_bytes());
            }
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlay_alpha_upload_preserves_terrain_opacity() -> Result<(), crate::BatchError> {
        let mut lighting = SceneLighting::default();
        lighting.set_opacity(0..1, 25)?;
        let quad = QuadInstance {
            position: [0.0; 2],
            size: [1.0; 2],
            uv: [0.0; 4],
        };
        let sprites = lighting.instance_bytes(&[quad, quad]);
        let terrain = lighting.opaque_instance_bytes(&[quad, quad]);
        assert_eq!(
            &sprites[LIT_INSTANCE_BYTES - 4..LIT_INSTANCE_BYTES],
            &0.25_f32.to_ne_bytes()
        );
        assert_eq!(
            &sprites[2 * LIT_INSTANCE_BYTES - 4..],
            &1.0_f32.to_ne_bytes()
        );
        for instance in terrain.as_chunks::<LIT_INSTANCE_BYTES>().0 {
            assert_eq!(&instance[LIT_INSTANCE_BYTES - 4..], &1.0_f32.to_ne_bytes());
        }
        assert!(
            lighting
                .set_opacity(std::ops::Range { start: 2, end: 1 }, 50)
                .is_err()
        );
        assert!(lighting.set_opacity(0..1, 101).is_err());
        Ok(())
    }
    #[test]
    fn clxa_local_light_falls_off_and_leaves_ui_emissive() {
        let lighting = SceneLighting {
            opacity: Vec::new(),
            ambient: [0; 3],
            sources: vec![PointLight {
                position: [16.0; 2],
                radius: 64.0,
                color: light_color(180),
            }],
            palette: Some([0.5, 0.5, 1.0, 1.0]),
            emissive_from: None,
            dimmed: Vec::new(),
        };
        assert_eq!(lighting.sample([16.0; 2]), [1.0, 0.0, 0.0]);
        assert_eq!(lighting.sample([48.0, 16.0]), [0.5, 0.0, 0.0]);
        assert_eq!(lighting.sample([80.0, 16.0]), [0.0; 3]);
        let quad = QuadInstance {
            position: [0.0; 2],
            size: [32.0; 2],
            uv: [0.0, 0.0, 0.25, 0.25],
        };
        let bytes = lighting.instance_bytes(&[
            quad,
            QuadInstance {
                uv: [0.6, 0.6, 0.7, 0.7],
                ..quad
            },
        ]);
        assert_eq!(bytes.len(), 2 * LIT_INSTANCE_BYTES);
        assert_eq!(&bytes[..QUAD_INSTANCE_BYTES], &quad.to_bytes());
        for (i, value) in [1.0_f32, 0.0, 0.0].into_iter().enumerate() {
            assert_eq!(
                &bytes[QUAD_INSTANCE_BYTES + i * 4..QUAD_INSTANCE_BYTES + (i + 1) * 4],
                &value.to_ne_bytes()
            );
        }
        for chunk in bytes[LIT_INSTANCE_BYTES + QUAD_INSTANCE_BYTES..]
            .as_chunks::<4>()
            .0
        {
            assert_eq!(chunk, &1.0_f32.to_ne_bytes());
        }
    }
}

#[cfg(test)]
mod effect_tests {
    use super::*;
    #[test]
    fn t012_effect_sprites_remain_visible_in_complete_darkness() {
        let lighting = SceneLighting {
            ambient: [0; 3],
            emissive_from: Some(1),
            ..SceneLighting::default()
        };
        let quad = QuadInstance {
            position: [0.0; 2],
            size: [32.0; 2],
            uv: [0.0, 0.0, 1.0, 1.0],
        };
        let bytes = lighting.instance_bytes(&[quad, quad]);
        for index in 0..2 {
            for channel in 0..3 {
                let at = index * LIT_INSTANCE_BYTES + QUAD_INSTANCE_BYTES + channel * 4;
                assert_eq!(&bytes[at..at + 4], &(index as f32).to_ne_bytes());
            }
        }
    }
}

#[cfg(test)]
mod tint_tests {
    use super::*;
    #[test]
    fn typed_tint_and_gpu_preserve_order_opacity_emission_dimming_and_linear_light() {
        let quad = QuadInstance {
            position: [0.; 2],
            size: [32.; 2],
            uv: [0., 0., 0.25, 0.25],
        };
        let palette = QuadInstance {
            uv: [0.6, 0.6, 0.7, 0.7],
            ..quad
        };
        let lighting = SceneLighting {
            ambient: [128; 3],
            palette: Some([0.5, 0.5, 1., 1.]),
            emissive_from: Some(3),
            dimmed: vec![(0..1, 50), (0..2, 75)],
            opacity: vec![(0..4, 50), (1..3, 25)],
            ..SceneLighting::default()
        };
        let instances = [quad, quad, palette, quad];
        let linear = |value: f32| ((value + 0.055) / 1.055).powf(2.4);
        let expected = [
            [linear(128. / 255. * 0.5); 3],
            [linear(128. / 255. * 0.25); 3],
            [1.; 3],
            [1.; 3],
        ];
        for layer in [QuadLayer::Terrain, QuadLayer::Overlay] {
            let bytes = if layer == QuadLayer::Terrain {
                lighting.opaque_instance_bytes(&instances)
            } else {
                lighting.instance_bytes(&instances)
            };
            for (index, quad) in instances.iter().enumerate() {
                let tint = lighting.quad_tint(quad, index, layer);
                let alpha = if layer == QuadLayer::Terrain {
                    1.
                } else {
                    [0.5, 0.25, 0.25, 0.5][index]
                };
                assert_eq!(
                    tint,
                    [
                        expected[index][0],
                        expected[index][1],
                        expected[index][2],
                        alpha
                    ]
                );
                let offset = index * LIT_INSTANCE_BYTES + QUAD_INSTANCE_BYTES;
                for (channel, value) in tint.into_iter().enumerate() {
                    assert_eq!(
                        &bytes[offset + channel * 4..offset + (channel + 1) * 4],
                        &value.to_ne_bytes()
                    );
                }
            }
        }
    }
}
