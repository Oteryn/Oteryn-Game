//! Client presentation of server-owned creature lights. No gameplay timers live here.
use crate::{QUAD_INSTANCE_BYTES, QuadInstance};

pub const LIT_INSTANCE_BYTES: usize = QUAD_INSTANCE_BYTES + 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight {
    pub position: [f32; 2],
    pub radius: f32,
    pub color: [u8; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneLighting {
    pub opacity: Vec<(std::ops::Range<usize>, u8)>,
    pub ambient: [u8; 3],
    pub sources: Vec<PointLight>,
    /// Emissive UI palette, in atlas UV coordinates.
    pub palette: Option<[f32; 4]>,
}

impl Default for SceneLighting {
    fn default() -> Self {
        Self {
            opacity: Vec::new(),
            ambient: [255; 3],
            sources: Vec::new(),
            palette: None,
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

    /// GPU upload layout extends the stable quad layout with an RGB multiplier.
    /// The shader samples an sRGB texture in linear space, so convert light here.
    #[must_use]
    pub fn instance_bytes(&self, instances: &[QuadInstance]) -> Vec<u8> {
        self.upload_bytes(instances, true)
    }

    /// Terrain never inherits opacity ranges belonging to the separate sprite batch.
    #[must_use]
    pub fn opaque_instance_bytes(&self, instances: &[QuadInstance]) -> Vec<u8> {
        self.upload_bytes(instances, false)
    }

    fn upload_bytes(&self, instances: &[QuadInstance], sprite_opacity: bool) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(instances.len() * LIT_INSTANCE_BYTES);
        for (index, quad) in instances.iter().enumerate() {
            bytes.extend_from_slice(&quad.to_bytes());
            let emissive = self.palette.is_some_and(|p| {
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
            for value in rgb {
                let linear = if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                };
                bytes.extend_from_slice(&linear.to_ne_bytes());
            }
            let alpha = if !sprite_opacity {
                1.0
            } else {
                self.opacity
                    .iter()
                    .rev()
                    .find(|(range, _)| range.contains(&index))
                    .map_or(1.0, |(_, percent)| f32::from((*percent).min(100)) / 100.0)
            };
            bytes.extend_from_slice(&alpha.to_ne_bytes());
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
