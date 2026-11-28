//! Native areas selected for a carried detector or thermometer.
use super::SelfTarget;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Survey {
    selection: Selection,
    areas: Vec<SelfTarget>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Selection {
    Personal,
    WorldReset,
}

impl Survey {
    pub(super) fn valid(&self) -> bool {
        (1..=16).contains(&self.areas.len()) && self.areas.iter().all(SelfTarget::valid)
    }

    /// The report timestamp identifies the commission across reconnects. Each floor
    /// draws its own coordinates from the Crystal detector rectangle.
    pub fn presence(&self, character: &[u8; 16], reported_at: i64) -> Option<(i32, i32, i16)> {
        if self.selection != Selection::Personal {
            return None;
        }
        let floor = self.areas.first()?.minimum.floor;
        let mut hash = Sha256::new();
        hash.update(character);
        hash.update(reported_at.to_le_bytes());
        hash.update(floor.to_le_bytes());
        let digest = hash.finalize();
        let x = u32::from_le_bytes(digest[..4].try_into().ok()?);
        let y = u32::from_le_bytes(digest[4..8].try_into().ok()?);
        Some((
            32171 + (x % 182) as i32,
            32512 + (y % 160) as i32,
            -i16::from(floor),
        ))
    }

    pub fn clue(
        &self,
        character: &[u8; 16],
        reported_at: i64,
        pool_draw: u64,
        x: i32,
        y: i32,
        floor: i16,
    ) -> Option<String> {
        if self.areas.is_empty() {
            return Some("No reading.".into());
        }
        let Some((tx, ty, depth)) = self.presence(character, reported_at) else {
            let pool = &self.areas[(pool_draw % self.areas.len() as u64) as usize];
            return (!pool.contains(x, y, floor)).then(|| "This is not the hot spot!".into());
        };
        let dx = f64::from(tx) - f64::from(x);
        let dy = f64::from(ty) - f64::from(y);
        if dx.abs() < 5.0 && dy.abs() < 5.0 {
            return match floor.cmp(&depth) {
                std::cmp::Ordering::Less => Some("The detector points below you.".into()),
                std::cmp::Ordering::Greater => Some("The detector points above you.".into()),
                std::cmp::Ordering::Equal => None,
            };
        }
        let horizontal = if dx > 0.0 { "east" } else { "west" };
        let vertical = if dy > 0.0 { "south" } else { "north" };
        let direction = if dy.abs() < dx.abs() * 0.4142 {
            horizontal.to_owned()
        } else if dy.abs() < dx.abs() * 2.4142 {
            format!("{vertical}-{horizontal}")
        } else {
            vertical.to_owned()
        };
        let squared = dx * dx + dy * dy;
        let distance = if squared >= 75625.0 {
            "very far to the"
        } else if squared >= 10000.0 {
            "far to the"
        } else if floor < depth {
            "on a lower level to the"
        } else if floor > depth {
            "on a higher level to the"
        } else {
            "to the"
        };
        Some(format!("The detector points {distance} {direction}."))
    }
}
