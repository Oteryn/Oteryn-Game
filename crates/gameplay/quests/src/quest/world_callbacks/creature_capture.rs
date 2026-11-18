//! Crystal 201189f8 data-global/scripts/spells/monster/spider_queen_wrap.lua:1-42
//! and data-global/monster/bosses/spider_queen.lua:67-69.
use super::*;

pub const PREFIX: &str = "oteryn:quest-creature-capture/";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CreatureCapture {
    pub creature: String,
    pub interval_ms: u32,
    pub range: u8,
    pub track: String,
    pub value: i64,
    pub condition_ms: u32,
    pub look_type: u16,
    /// Source speed coefficients in thousandths, in the donor's order.
    pub speed: [i32; 2],
    pub teleport_delay_ms: u32,
    pub landing: Position,
    pub message: String,
}

impl CreatureCapture {
    pub(super) fn valid(&self, callback: &Callback) -> bool {
        callback.actor_kind == ActorKind::Player
            && callback.trigger == Trigger::Use
            && callback.requires_world_admission
            && callback
                .creature_capture_key()
                .is_some_and(|key| super::super::valid_quest_key(&key))
            && callback
                .storage
                .iter()
                .any(|guard| guard.track == self.track && !guard.read_only)
            && self.creature.starts_with("oteryn:creature.")
            && super::super::valid_quest_key(&self.creature)
            && (1..=60_000).contains(&self.interval_ms)
            && (1..=8).contains(&self.range)
            && (1..=600_000).contains(&self.condition_ms)
            && (1..=self.condition_ms).contains(&self.teleport_delay_ms)
            && self.look_type != 0
            && self.speed.iter().all(|a| (-1_000..0).contains(a))
            && (-15..=0).contains(&self.landing.floor)
            && !self.message.is_empty()
            && self.message.len() <= 512
    }
}

impl Callback {
    pub fn creature_capture_key(&self) -> Option<String> {
        self.effects.creature_capture.as_ref()?;
        Some(format!(
            "{PREFIX}{}",
            self.key.strip_prefix("oteryn:quest-callback/")?
        ))
    }

    pub fn creature_capture_transition(&self) -> Result<Option<QuestTransition>, String> {
        let Some(capture) = &self.effects.creature_capture else {
            return Ok(None);
        };
        Ok(Some(QuestTransition {
            key: self
                .creature_capture_key()
                .ok_or("creature capture identity")?,
            quest: self.quest.clone(),
            effects: vec![QuestEffect {
                track: capture.track.clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::SetWithBinding {
                    value: capture.value,
                    binding: gated_binding(self)?.ok_or("creature capture binding")?,
                },
            }],
            completes: false,
            experience: None,
            world_change: None,
        }))
    }
}
