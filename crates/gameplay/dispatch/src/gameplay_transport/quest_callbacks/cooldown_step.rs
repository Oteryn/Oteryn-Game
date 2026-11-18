//! Crystal 201189f8 data-global/scripts/quests/cults_of_tibia/movements_boss_timer.lua:13-15,36-99.
use super::*;
use oteryn_gameplay_quests::quest::world_callbacks::BossCooldownGate;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn boss_cooldown_step_refusal(
        &self,
        permit: &SpellLanePermit,
        session: GameSessionId,
        gate: &BossCooldownGate,
    ) -> Option<String> {
        let remaining = match self.current_quest_fence(session).await {
            Ok(Some(fence)) => self
                .root
                .boss_portal_cooldown_seconds(
                    self.character,
                    self.holder,
                    permit,
                    fence.item_fence(),
                    &gate.boss,
                )
                .await
                .ok()
                .flatten(),
            _ => None,
        };
        match remaining {
            Some(0) => None,
            Some(seconds) => Some(format!(
                "You need to wait {} before facing {} again.",
                time_left(seconds),
                gate.name,
            )),
            None => Some("You cannot face this boss yet.".into()),
        }
    }
}

fn time_left(seconds: u64) -> String {
    let days = seconds / 86_400;
    let units = [
        (days, "day"),
        (seconds / 3_600 % 24, "hour"),
        (seconds / 60 % 60, "minute"),
        (if days == 0 { seconds % 60 } else { 0 }, "second"),
    ];
    let mut parts: Vec<_> = units
        .into_iter()
        .filter(|(amount, _)| *amount > 0)
        .map(|(amount, unit)| format!("{amount} {unit}{}", if amount == 1 { "" } else { "s" }))
        .collect();
    if parts.len() > 1
        && let Some(last) = parts.pop()
    {
        format!("{} and {last}", parts.join(", "))
    } else {
        parts.join("")
    }
}
