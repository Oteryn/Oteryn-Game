//! Health/mana arc geometry uses admitted server vitals; no estimated health is drawn.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArcPreferences {
    pub health: bool,
    pub mana: bool,
    pub distance: u16,
    pub size: u16,
    pub opacity: u8,
}
impl Default for ArcPreferences {
    fn default() -> Self {
        Self {
            health: false,
            mana: false,
            distance: 20,
            size: 60,
            opacity: 85,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArcStroke {
    pub health: bool,
    pub points: Vec<[i32; 2]>,
    pub filled_segments: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArcLayout {
    pub origin: [i32; 2],
    pub width: i32,
    pub height: i32,
    pub alpha: u8,
    pub strokes: Vec<ArcStroke>,
    pub condition_icons: Vec<String>,
}
impl ArcPreferences {
    pub fn valid(self) -> bool {
        self.distance <= 120 && (20..=120).contains(&self.size) && self.opacity <= 100
    }
    pub fn layout(
        self,
        center: [i32; 2],
        vitals: Option<&oteryn_session::ActorVitals>,
    ) -> Option<ArcLayout> {
        if !self.valid() || (!self.health && !self.mana) || self.opacity == 0 {
            return None;
        }
        let vitals = vitals?;
        let half_width = i32::from(self.distance) + i32::from(self.size) + 8;
        let half_height = i32::from(self.size) + 8;
        let mut strokes = Vec::new();
        for (health, value, maximum) in [
            (true, vitals.health, vitals.max_health),
            (false, vitals.mana, vitals.max_mana),
        ] {
            if (health && !self.health) || (!health && !self.mana) {
                continue;
            }
            let points = (0..=80)
                .map(|step| {
                    let fraction = f64::from(step) / 80.0;
                    let angle = std::f64::consts::PI
                        * if health {
                            0.6 + fraction * 0.8
                        } else {
                            0.4 - fraction * 0.8
                        };
                    let offset = if health {
                        -f64::from(self.distance)
                    } else {
                        f64::from(self.distance)
                    };
                    [
                        (f64::from(half_width) + offset + angle.cos() * f64::from(self.size))
                            .round() as i32,
                        (f64::from(half_height) + angle.sin() * f64::from(self.size)).round()
                            as i32,
                    ]
                })
                .collect();
            let filled_segments = if maximum == 0 {
                0
            } else {
                usize::try_from(u64::from(value.min(maximum)) * 80 / u64::from(maximum)).ok()?
            };
            strokes.push(ArcStroke {
                health,
                points,
                filled_segments,
            });
        }
        Some(ArcLayout {
            origin: [
                center[0].saturating_sub(half_width),
                center[1].saturating_sub(half_height),
            ],
            width: half_width * 2,
            height: half_height * 2,
            alpha: u8::try_from(u16::from(self.opacity) * 255 / 100).ok()?,
            strokes,
            condition_icons: Vec::new(),
        })
    }
}
impl crate::play::PlayView {
    pub fn hud_arc_layout(&self) -> Option<ArcLayout> {
        let icons = self.special_condition_rows(true);
        let mut layout = if let Some(layout) = self
            .preferences()
            .arcs
            .layout(self.scene.hud_center(), self.actor_vitals())
        {
            layout
        } else {
            let options = self.preferences().arcs;
            if icons.is_empty() || !options.valid() || options.opacity == 0 {
                return None;
            }
            let half_width = i32::from(options.distance) + i32::from(options.size) + 8;
            let half_height = i32::from(options.size) + 8;
            let center = self.scene.hud_center();
            ArcLayout {
                origin: [
                    center[0].saturating_sub(half_width),
                    center[1].saturating_sub(half_height),
                ],
                width: half_width * 2,
                height: half_height * 2,
                alpha: (u16::from(options.opacity) * 255 / 100) as u8,
                strokes: Vec::new(),
                condition_icons: Vec::new(),
            }
        };
        if !icons.is_empty() {
            layout.origin[0] = layout.origin[0].saturating_sub(28);
            layout.width += 28;
            layout.height = layout.height.max(7 * 18);
            for stroke in &mut layout.strokes {
                for point in &mut stroke.points {
                    point[0] += 28;
                }
            }
            layout.condition_icons = icons.iter().take(6).map(|row| row.1.clone()).collect();
            if icons.len() > 6 {
                layout.condition_icons.push("…".into());
            }
        }
        Some(layout)
    }
}
