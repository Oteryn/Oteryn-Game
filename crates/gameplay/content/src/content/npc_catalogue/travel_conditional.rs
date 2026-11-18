//! Ordered native route choices. Crystal 201189f8 data-global/npc/buddel.lua:54-126.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TravelCell {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

impl TravelCell {
    fn valid(&self) -> bool {
        (0..=65535).contains(&self.x)
            && (0..=65535).contains(&self.y)
            && (0..=15).contains(&self.floor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressWindow {
    pub track: String,
    pub absent: i64,
    pub minimum: i64,
    pub maximum: i64,
}

impl ProgressWindow {
    fn valid(&self) -> bool {
        self.minimum <= self.maximum
            && self.track.len() <= 128
            && self
                .track
                .strip_prefix("oteryn:quest-progress/")
                .is_some_and(|suffix| {
                    !suffix.is_empty()
                        && suffix
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"._/-".contains(&b))
                })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Diversion {
    pub denominator: u8,
    pub preferred_successes: u8,
    pub diversion_successes: u8,
    pub alternatives: Vec<TravelCell>,
}

impl Diversion {
    fn valid(&self) -> bool {
        self.denominator > 1
            && (1..self.denominator).contains(&self.preferred_successes)
            && (1..self.denominator).contains(&self.diversion_successes)
            && (1..=8).contains(&self.alternatives.len())
            && self.alternatives.iter().all(TravelCell::valid)
    }

    /// The second branch draws separately only after the first branch rejects.
    pub fn select(
        &self,
        preferred: TravelCell,
        mut draw: impl FnMut(u8) -> Option<u8>,
    ) -> Option<TravelCell> {
        if !self.valid() {
            return None;
        }
        if draw(self.denominator)? < self.preferred_successes {
            return Some(preferred);
        }
        if draw(self.denominator)? < self.diversion_successes {
            return self
                .alternatives
                .get(usize::from(draw(self.alternatives.len() as u8)?))
                .copied();
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionalTravel {
    pub progress: Vec<ProgressWindow>,
    pub diversion: Diversion,
    pub offer_reply: String,
    #[serde(default)]
    pub unavailable_reply: Option<String>,
    /// Crystal buddel.lua:62-67 opens a paid offer after the ring is declined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declined_exchange: Option<DeclinedExchange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel_reply: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeclinedExchange {
    pub topic: i32,
    pub reply: String,
    pub progress: Vec<ProgressWindow>,
}

fn valid_reply(reply: &str) -> bool {
    !reply.is_empty() && reply.len() <= 256 && !reply.contains(['\0', '\n', '\r'])
}

impl ConditionalTravel {
    pub fn valid(&self) -> bool {
        (1..=8).contains(&self.progress.len())
            && self.progress.iter().all(ProgressWindow::valid)
            && self.diversion.valid()
            && valid_reply(&self.offer_reply)
            && self.unavailable_reply.as_deref().is_none_or(valid_reply)
            && self.cancel_reply.as_deref().is_none_or(valid_reply)
            && self.declined_exchange.as_ref().is_none_or(|offer| {
                offer.topic > 0
                    && valid_reply(&offer.reply)
                    && offer.progress.len() <= 8
                    && offer.progress.iter().all(ProgressWindow::valid)
            })
    }
}
