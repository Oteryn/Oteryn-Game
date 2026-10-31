//! The production descendant of a frozen reward-boss death. Personal loot uses the complete
//! death-time census and the original immutable table; the ordinary corpse has no reward loot.
use super::*;
use crate::domain::item_definition::ItemDefinitionFacts;
use crate::durability::boss_encounter::reward_chest::{
    BOSS_REWARD_CONTRIBUTORS_MAX, BOSS_REWARD_ITEMS_MAX, BOSS_REWARD_PARTICIPANTS_MAX,
    BossRewardChestPlan, BossRewardContributor, BossRewardDeath, BossRewardItem,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::RuntimeScopeRefV1;
use oteryn_gameplay_encounters::bosses::reward_chest::{
    FrozenBossContribution, contribution_loot_factors, freeze_personal_rewards, reward_eligibility,
    reward_expiry, top_contributor,
};
use oteryn_gameplay_items::loot_plan::{LootPlan, LootPlanError, plan_boss_loot};
use sha2::{Digest, Sha256};

#[cfg(test)]
#[allow(clippy::expect_used, reason = "bounded owner settlement test fixtures")]
#[path = "boss_reward_settlement_tests.rs"]
mod tests;

#[derive(Debug)]
enum PlanError {
    InvalidCensus,
    InvalidIdentity,
    ContentUnavailable,
    Loot(LootPlanError),
    TooManyItems,
    Overflow,
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loot(error) => write!(formatter, "loot: {error}"),
            Self::InvalidCensus => formatter.write_str("invalid contribution census"),
            Self::InvalidIdentity => formatter.write_str("invalid captured identity"),
            Self::ContentUnavailable => formatter.write_str("item content unavailable"),
            Self::TooManyItems => formatter.write_str("reward item bound exceeded"),
            Self::Overflow => formatter.write_str("quantity overflow"),
        }
    }
}

/// A scope-owned death has one identity shared by all its recipients. XP occurrences are
/// per-principal and must not be substituted for this sealed actor-generation identity.
fn personal_death_id(death: CreatureDeathOccurrenceKey) -> [u8; 16] {
    scoped_death_id(
        death.runtime_scope(),
        death.scope_ownership_generation().get(),
        death.actor_local_id(),
        death.actor_local_generation(),
    )
}

fn scoped_death_id(
    scope: RuntimeScopeRefV1,
    generation: u64,
    actor: u32,
    actor_generation: u64,
) -> [u8; 16] {
    let (kind, scope_id) = match scope {
        RuntimeScopeRefV1::Channel { channel_id, .. } => (1_u8, *channel_id.as_bytes()),
        RuntimeScopeRefV1::Instance { instance_id, .. } => (2_u8, instance_id),
    };
    let digest = Sha256::new()
        .chain_update(b"oteryn:personal-boss-death:v1\0")
        .chain_update(scope.world_id().as_bytes())
        .chain_update([kind])
        .chain_update(scope_id)
        .chain_update(generation.to_be_bytes())
        .chain_update(actor.to_be_bytes())
        .chain_update(actor_generation.to_be_bytes())
        .finalize();
    let mut result = [0; 16];
    result.copy_from_slice(&digest[..16]);
    result
}

/// Aggregate identical selected Items before making physical stacks, like the donor's
/// result[itemId] reward accumulator. There is no reroll and no discarded remainder.
fn materialize_plan(
    plan: LootPlan,
    facts: &impl Fn(&TypedDefinitionRef) -> Option<ItemDefinitionFacts>,
) -> Result<Vec<BossRewardItem>, PlanError> {
    let mut totals: Vec<(TypedDefinitionRef, u64)> = Vec::new();
    for entry in plan.entries {
        if entry.needs_subtype_state() {
            return Err(PlanError::Loot(LootPlanError::UnsupportedItemSubtype));
        }
        if entry.parent.is_some() || entry.quantity == 0 {
            return Err(PlanError::ContentUnavailable);
        }
        let definition = TypedDefinitionRef {
            family: entry.item.family,
            production_key: entry.item.production_key,
            revision_ref: entry.item.revision_ref,
        };
        if let Some((_, count)) = totals.iter_mut().find(|(key, _)| *key == definition) {
            *count = count
                .checked_add(u64::from(entry.quantity))
                .ok_or(PlanError::Overflow)?;
        } else {
            totals.push((definition, u64::from(entry.quantity)));
        }
    }
    let mut items = Vec::new();
    for (definition, mut quantity) in totals {
        let item = facts(&definition).ok_or(PlanError::ContentUnavailable)?;
        if item.definition != definition {
            return Err(PlanError::ContentUnavailable);
        }
        // Use the same D82 admissibility as collection and its op4 audit. A content
        // maximum above 100 must fail before we freeze an uncollectable entitlement.
        let (maximum, _) = oteryn_gameplay_items::stacking::stack_maximum(item.stack)
            .map_err(|_| PlanError::ContentUnavailable)?;
        if item.container_capacity.is_some() && maximum != 1 {
            return Err(PlanError::ContentUnavailable);
        }
        let stacks = quantity.div_ceil(u64::from(maximum));
        if stacks > BOSS_REWARD_ITEMS_MAX.saturating_sub(items.len()) as u64 {
            return Err(PlanError::TooManyItems);
        }
        while quantity > 0 {
            let count = quantity.min(u64::from(maximum)) as u32;
            items.push(BossRewardItem {
                definition: definition.clone(),
                quantity: count,
            });
            quantity -= u64::from(count);
        }
    }
    Ok(items)
}

fn plan_personal_death(
    death: [u8; 16],
    boss_key: &str,
    frozen: &boss_reward_contribution::FrozenBossDeath,
    facts: impl Fn(&TypedDefinitionRef) -> Option<ItemDefinitionFacts>,
) -> Result<BossRewardDeath, PlanError> {
    if frozen.participants.len() > BOSS_REWARD_CONTRIBUTORS_MAX {
        return Err(PlanError::InvalidCensus);
    }
    let mut participants = frozen.participants.iter().collect::<Vec<_>>();
    participants.sort_by_key(|participant| *participant.character.as_bytes());
    if participants
        .windows(2)
        .any(|pair| pair[0].character == pair[1].character)
        || participants
            .iter()
            .any(|participant| participant.lease_generation == 0)
    {
        return Err(PlanError::InvalidCensus);
    }
    let census = participants
        .iter()
        .map(|participant| FrozenBossContribution {
            character: *participant.character.as_bytes(),
            contribution: participant.contribution,
        })
        .collect::<Vec<_>>();
    let eligible = reward_eligibility(
        &census
            .iter()
            .map(|row| row.contribution)
            .collect::<Vec<_>>(),
    );
    if eligible.iter().filter(|value| **value).count() > BOSS_REWARD_PARTICIPANTS_MAX {
        return Err(PlanError::InvalidCensus);
    }
    let factors = contribution_loot_factors(&census);
    let top = top_contributor(&census);
    let mut contributors = Vec::with_capacity(participants.len());
    for participant in participants {
        let character_id =
            crate::domain::CharacterId::from_bytes(*participant.character.as_bytes())
                .map_err(|_| PlanError::InvalidIdentity)?;
        contributors.push(BossRewardContributor {
            character_id,
            session_id: participant.session,
            lease_generation: participant.lease_generation,
            contribution: participant.contribution,
        });
    }
    let expires = reward_expiry(frozen.died_at_unix_ms).ok_or(PlanError::Overflow)?;
    let mut failure = None;
    // The existing personal-container owner freezes eligibility before invoking the loot
    // callback. A fallible roll rejects the entire death plan; its empty temporary result
    // cannot escape this function as an earned-but-empty durable entitlement.
    let personal = freeze_personal_rewards(boss_key, death, expires, &census, |character, _| {
        let planned = (|| {
            let index = census
                .iter()
                .position(|row| row.character == character)
                .ok_or(PlanError::InvalidCensus)?;
            let loot = plan_boss_loot(
                death,
                character,
                &frozen.reward.loot_table_ref,
                &frozen.reward.loot_table,
                factors[index],
                top == Some(character),
            )
            .map_err(PlanError::Loot)?;
            materialize_plan(loot, &facts)
        })();
        match planned {
            Ok(items) => items,
            Err(error) => {
                failure.get_or_insert(error);
                Vec::new()
            }
        }
    });
    if let Some(error) = failure {
        return Err(error);
    }
    let mut chests = Vec::with_capacity(personal.len());
    for chest in personal {
        let participant = contributors
            .iter()
            .find(|row| row.character_id.as_bytes() == &chest.character)
            .ok_or(PlanError::InvalidCensus)?;
        chests.push(BossRewardChestPlan {
            character_id: participant.character_id,
            session_id: participant.session_id,
            lease_generation: participant.lease_generation,
            contribution: chest.contribution,
            items: chest.items().to_vec(),
        });
    }
    Ok(BossRewardDeath {
        death,
        boss_key: boss_key.to_owned(),
        minted_at_unix_ms: frozen.died_at_unix_ms,
        contributors,
        chests,
    })
}

impl DurableKillSettle<'_, '_, '_, '_> {
    /// The room's primary native death starts its authored after-kill return
    /// even when that creature creates no personal reward group. W03 keeps its
    /// existing atomic room decision and chest settlement in the transaction below.
    pub(super) async fn settle_activity_boss_defeat(&self, entry: &PendingKillSettlement) -> bool {
        let Some(captured) = &entry.activity_death else {
            return true;
        };
        if entry.boss_reward.is_some() {
            return true;
        }
        let admission = self.admission;
        let Some(native) = admission
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            operator_event("activity_defeat_deferred reason=content_unavailable");
            return false;
        };
        let digest = native.source_digest();
        let revision = format!(
            "sha256:{}",
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let current_pin = {
            let runtime = admission.runtime.lock().await;
            runtime.owner_fence().is_ok()
                && runtime.binding().runtime_scope() == entry.facts.death.runtime_scope()
                && runtime.binding().scope_generation()
                    == entry.facts.death.scope_ownership_generation()
                && runtime.content_pin().server_artifact_digest() == digest
                && entry.ground.content_revision == revision
        };
        if !current_pin {
            operator_event("activity_defeat_deferred reason=scope_or_content_fenced");
            return false;
        }
        let owner =
            crate::durability::activity_handoff::ActivityHandoffOwner::new(admission.root.clone());
        match owner
            .record_boss_defeat(
                admission.character,
                admission.holder,
                entry.facts.death.runtime_scope(),
                entry.facts.death.scope_ownership_generation(),
                crate::durability::activity_handoff::boss_room::BossDefeatOccurrence {
                    boss_key: &captured.creature,
                    death: personal_death_id(entry.facts.death),
                    minted_unix_ms: captured.died_at_unix_ms,
                },
            )
            .await
        {
            Ok(true) => true,
            result => {
                operator_event(&format!(
                    "activity_defeat_deferred creature={} result={result:?}",
                    captured.creature
                ));
                false
            }
        }
    }

    /// No current recipient lease is needed to issue an earned entitlement. This call is
    /// authorized by the current scope owner and the frozen, retained participant admissions;
    /// collection later proves the recipient's current session independently.
    pub(super) async fn settle_personal_boss_rewards(&self, entry: &PendingKillSettlement) -> bool {
        let Some(frozen) = &entry.boss_reward else {
            return true;
        };
        let admission = self.admission;
        let Some(native) = admission
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            operator_event("boss_reward_settlement_deferred reason=content_unavailable");
            return false;
        };
        let digest = native.source_digest();
        let revision = format!(
            "sha256:{}",
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let current_pin = {
            let runtime = admission.runtime.lock().await;
            runtime.owner_fence().is_ok()
                && runtime.binding().runtime_scope() == entry.facts.death.runtime_scope()
                && runtime.binding().scope_generation()
                    == entry.facts.death.scope_ownership_generation()
                && runtime.content_pin().server_artifact_digest() == digest
                && entry.ground.content_revision == revision
        };
        if !current_pin {
            operator_event("boss_reward_settlement_deferred reason=scope_or_content_fenced");
            return false;
        }
        // Use the authored boss identity when present, otherwise the qualified creature key.
        // reward_boss is the native classification; Bosstiary membership is not required.
        let key = frozen
            .reward
            .boss
            .as_ref()
            .map_or(entry.creature.as_str(), |boss| boss.key.as_str());
        let plan = match plan_personal_death(
            personal_death_id(entry.facts.death),
            key,
            frozen,
            |definition| {
                super::super::item_ref_admission::generation_item_facts(native, definition).ok()
            },
        ) {
            Ok(plan) => plan,
            Err(error) => {
                operator_event(&format!(
                    "boss_reward_settlement_deferred creature={} reason={error}",
                    entry.creature
                ));
                return false;
            }
        };
        match admission
            .root
            .settle_boss_reward_chests(
                admission.character,
                admission.holder,
                entry.facts.death.runtime_scope(),
                entry.facts.death.scope_ownership_generation(),
                plan,
            )
            .await
        {
            Ok(true) => true,
            result => {
                operator_event(&format!(
                    "boss_reward_settlement_deferred creature={} result={result:?}",
                    entry.creature
                ));
                false
            }
        }
    }
}
