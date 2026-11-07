//! Bounded predicates for one exact carried source and its authored replacement.
use super::Position;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActorRegion {
    pub minimum: Position,
    pub maximum: Position,
}
impl ActorRegion {
    pub fn valid(&self) -> bool {
        self.minimum.x <= self.maximum.x
            && self.minimum.y <= self.maximum.y
            && (-15..=0).contains(&self.minimum.floor)
            && (-15..=0).contains(&self.maximum.floor)
            && self.minimum.floor <= self.maximum.floor
    }
    pub fn contains(&self, position: Position) -> bool {
        self.valid()
            && (self.minimum.x..=self.maximum.x).contains(&position.x)
            && (self.minimum.y..=self.maximum.y).contains(&position.y)
            && (self.minimum.floor..=self.maximum.floor).contains(&position.floor)
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TimeWindow {
    pub from_minute: u16,
    pub to_minute: u16,
}
impl TimeWindow {
    pub fn valid(&self) -> bool {
        self.from_minute < 1440 && self.to_minute < 1440 && self.from_minute != self.to_minute
    }
    pub fn contains(&self, minute: u32) -> bool {
        if !self.valid() || minute >= 1440 {
            return false;
        }
        let (from, to) = (u32::from(self.from_minute), u32::from(self.to_minute));
        if from < to {
            (from..to).contains(&minute)
        } else {
            minute >= from || minute < to
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CarriedUse {
    pub actor_region: ActorRegion,
    pub time_window: TimeWindow,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn definition(callback: super::super::Callback) -> Result<[u8; 32], String> {
        let base = crate::quest::QuestStateCatalogue::new("binding-test", vec![], vec![])
            .map_err(|error| format!("{error:?}"))?;
        let quest = callback.quest.clone();
        super::super::extend_callbacks(base, &[callback])?
            .definition_hash(&quest)
            .ok_or_else(|| "quest hash".into())
    }

    #[test]
    fn published_self_use_binding_is_unchanged_and_target_terms_are_bound() -> Result<(), String> {
        let rows = super::super::load()?;
        let old = rows
            .iter()
            .find(|callback| callback.key.contains("/threatened-light/"))
            .ok_or("self callback")?;
        let legacy: [u8; 32] = Sha256::digest(
            serde_json::to_vec(&(
                &old.key,
                &old.quest,
                old.trigger,
                &old.storage,
                &old.item_on_target,
                &old.carried_use,
            ))
            .map_err(|error| error.to_string())?,
        )
        .into();
        let base = crate::quest::QuestStateCatalogue::new("binding-test", vec![], vec![])
            .map_err(|error| format!("{error:?}"))?;
        let book = super::super::extend_callbacks(base, std::slice::from_ref(old))?;
        let transition = book.transition(&old.key).ok_or("self transition")?;
        let mut bound = 0;
        for effect in &transition.effects {
            if let crate::quest::QuestEffectKind::SetWithBinding { binding, .. } = effect.effect {
                assert_eq!(
                    binding, legacy,
                    "already published self-use receipts retain their definition encoding"
                );
                bound += 1;
            }
        }
        assert!(bound > 0);
        let target = rows
            .into_iter()
            .find(|callback| callback.trigger == super::super::Trigger::UseCarriedOnTarget)
            .ok_or("target callback")?;
        let original = definition(target.clone())?;
        let mut other = target.clone();
        other.selector.position.as_mut().ok_or("target position")?.x += 1;
        assert_ne!(definition(other)?, original);
        let mut level = target;
        level.minimum_level = Some(30);
        assert_ne!(definition(level)?, original);
        Ok(())
    }
    #[test]
    fn day_night_boundaries_and_midnight_are_exact() {
        let day = TimeWindow {
            from_minute: 360,
            to_minute: 1080,
        };
        let night = TimeWindow {
            from_minute: 1080,
            to_minute: 360,
        };
        for (minute, is_day) in [
            (0, false),
            (359, false),
            (360, true),
            (1079, true),
            (1080, false),
            (1439, false),
        ] {
            assert_eq!(day.contains(minute), is_day);
            assert_eq!(night.contains(minute), !is_day);
        }
        assert!(!day.contains(1440));
        assert!(
            !TimeWindow {
                from_minute: 0,
                to_minute: 0
            }
            .valid()
        );
        assert!(
            !TimeWindow {
                from_minute: 1440,
                to_minute: 1
            }
            .valid()
        );
    }
    #[test]
    fn rectangle_edges_and_floor_are_admission_facts() {
        let region = ActorRegion {
            minimum: Position {
                x: 100,
                y: 20,
                floor: -7,
            },
            maximum: Position {
                x: 110,
                y: 25,
                floor: -7,
            },
        };
        assert!(region.contains(region.minimum));
        assert!(region.contains(region.maximum));
        for position in [
            Position {
                x: 99,
                y: 20,
                floor: -7,
            },
            Position {
                x: 111,
                y: 25,
                floor: -7,
            },
            Position {
                x: 100,
                y: 19,
                floor: -7,
            },
            Position {
                x: 110,
                y: 26,
                floor: -7,
            },
            Position {
                x: 100,
                y: 20,
                floor: -8,
            },
        ] {
            assert!(!region.contains(position));
        }
    }
    #[test]
    fn changed_region_or_period_invalidates_the_durable_definition() -> Result<(), String> {
        let callback = super::super::load()?
            .into_iter()
            .find(|callback| callback.carried_use.is_some())
            .ok_or("carried callback")?;
        let hash = |callback: super::super::Callback| -> Result<_, String> {
            let base = crate::quest::QuestStateCatalogue::new("binding-test", vec![], vec![])
                .map_err(|error| format!("{error:?}"))?;
            let quest = callback.quest.clone();
            Ok(super::super::extend_callbacks(base, &[callback])?.definition_hash(&quest))
        };
        let original = hash(callback.clone())?;
        let mut area = callback.clone();
        area.carried_use
            .as_mut()
            .ok_or("area")?
            .actor_region
            .maximum
            .x += 1;
        assert_ne!(hash(area)?, original);
        let mut period = callback.clone();
        period
            .carried_use
            .as_mut()
            .ok_or("period")?
            .time_window
            .from_minute -= 1;
        assert_ne!(hash(period)?, original);
        let mut wording = callback;
        wording.success_message = Some("Presentation only.".into());
        assert_eq!(hash(wording)?, original);
        Ok(())
    }
}
