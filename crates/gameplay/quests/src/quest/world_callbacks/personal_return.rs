//! Source-armed personal expiry with a bounded recovery landing.
use super::*;

/// Crystal 201189f8 mission06_run_like_wolf.lua:137-163.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PersonalReturn {
    pub delay_ms: u32,
    pub landing: Position,
    pub retry_value: i64,
    pub failure_message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub haste: Option<ReturnHaste>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReturnHaste {
    /// Thousandths of base movement speed.
    pub multiplier: i32,
    pub offset: i32,
}

impl Callback {
    pub fn personal_return_key(&self) -> Option<String> {
        self.effects.personal_return.as_ref()?;
        Some(format!(
            "oteryn:quest-personal-return/{}",
            self.key.strip_prefix("oteryn:quest-callback/")?
        ))
    }

    pub fn personal_return_transition(&self) -> Result<Option<QuestTransition>, String> {
        let Some(timer) = &self.effects.personal_return else {
            return Ok(None);
        };
        let [storage] = self.storage.as_slice() else {
            return Err("personal return requires exactly one source track".into());
        };
        Ok(Some(QuestTransition {
            key: self.personal_return_key().ok_or("personal return key")?,
            quest: self.quest.clone(),
            effects: vec![QuestEffect {
                track: storage.track.clone(),
                from: QuestComparison::Eq(storage.set),
                effect: QuestEffectKind::SetWithBinding {
                    value: timer.retry_value,
                    binding: gated_binding(self)?.ok_or("personal return binding")?,
                },
            }],
            completes: false,
            experience: None,
            world_change: None,
        }))
    }
}

pub(super) fn valid(callback: &Callback, timer: &PersonalReturn) -> bool {
    let [storage] = callback.storage.as_slice() else {
        return false;
    };
    callback.trigger == Trigger::StepIn
        && callback.actor_kind == ActorKind::Player
        && callback.requires_world_admission
        && callback.personal_return_key().is_some_and(|key| super::super::valid_quest_key(&key))
        && callback.operation_count() == 0
        && callback.teleport.is_none()
        && callback.passages.is_empty()
        && callback.lever.is_none()
        && callback.effects.encounter.is_none()
        && callback.effects.ambush.is_empty()
        && callback.effects.party_transfer.is_none()
        && callback.effects.scene.is_none()
        && !storage.preserve && !storage.read_only && !storage.increment && !storage.at_least
        && storage.set_if.is_none() && storage.add.is_none()
        && storage.minimum == storage.maximum
        && storage.minimum == timer.retry_value
        && timer.retry_value != storage.set
        && (1..=3_600_000).contains(&timer.delay_ms)
        && (-15..=0).contains(&timer.landing.floor)
        && !timer.failure_message.is_empty()
        && timer.failure_message.len() <= 512
        && timer.haste.as_ref().is_none_or(|haste| {
            (1..=1000).contains(&haste.multiplier) && (-100..=100).contains(&haste.offset)
        })
}
