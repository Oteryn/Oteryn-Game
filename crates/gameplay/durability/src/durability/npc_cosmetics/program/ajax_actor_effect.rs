//! Closed Ajax FIRE child of an already qualified source NPC transaction.
use crate::durability::DurabilityError;
use crate::durability::character_condition_snapshot::{self, CharacterConditionSnapshot};
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::npc_cosmetics::NpcProgramContext;
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct NpcSourceActorEffect {
    pub actor_identity: [u8; 16],
    pub movement_revision: u64,
    pub state_revision: u64,
    pub before: CharacterConditionSnapshot,
    pub after: CharacterConditionSnapshot,
}
impl NpcSourceActorEffect {
    pub(crate) fn valid(&self, ctx: &NpcProgramContext<'_>) -> bool {
        self.actor_identity != [0; 16]
            && self.movement_revision > 0
            && self.state_revision > 0
            && ctx.npc_key == "oteryn:npc.ajax"
            && ctx.source_key == "oteryn:npc-cosmetic/ajax/barbarian-fire"
            && self.before.valid()
            && self.after.valid()
            && self.before.lease_generation == ctx.fence.character_lease_generation
            && self.after.lease_generation == ctx.fence.character_lease_generation
            && self.before.content_digest == ctx.native.source_digest()
            && self.after.content_digest == ctx.native.source_digest()
            && exact_fire_successor(&self.before.conditions, &self.after.conditions)
    }
    /// Added by the Root owner to its secret-armed before_state, never accepted
    /// as a client payload. The parent source plan independently selects FIRE.
    pub(crate) fn observation(&self) -> Value {
        json!({"kind":"ajax_fire","actor_identity":self.actor_identity.iter().map(|b|format!("{b:02x}")).collect::<String>(),
            "movement_revision":self.movement_revision.to_string(),"state_revision":self.state_revision.to_string(),
            "lease_generation":self.before.lease_generation.to_string(),
            "content_digest":self.after.content_digest.iter().map(|b|format!("{b:02x}")).collect::<String>(),
            "conditions_before":self.before.conditions,"conditions_after":self.after.conditions})
    }
}
// Replay the same deterministic Foundation kernel on frozen source snapshots.
// Only the newly allocated Fire sequence is process-local metadata omitted by
// the persistence format; require it to remain later than all prior instances.
fn exact_fire_successor(before: &Value, after: &Value) -> bool {
    use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
    use oteryn_gameplay_foundation::foundation::condition::{
        ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionStore,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    let Ok(mut replay) = ConditionStore::<String>::from_persistent_snapshot(before, 0) else {
        return false;
    };
    let Some(after_conditions) = after.get("conditions").and_then(Value::as_array) else {
        return false;
    };
    let source_fire = after_conditions.iter().find(|instance| {
        instance.pointer("/definition/key").and_then(Value::as_str) == Some("npc.ajax.source_fire")
    });
    let base_speed = source_fire
        .and_then(|instance| instance.get("speed_base"))
        .and_then(Value::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| *value > 0)
        .unwrap_or(1);
    let Some(definition) = ConditionDefinition::new(
        "npc.ajax.source_fire",
        1,
        ConditionValues::DamageOverTime {
            element: DotElement::Fire,
            total_min: 100,
            total_max: 100,
            per_tick: 10,
            interval_ms: 2000,
            delayed: true,
        },
    ) else {
        return false;
    };
    let root = GameplayDecisionRoot::from_bytes([0; 32]);
    let facts = ApplicationFacts {
        now: 0,
        base_speed,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: DecisionOccurrenceId::from_bytes([0; 16]),
    };
    let fresh = match replay.apply(&definition, None, ConditionSourceKind::SelfUse, &[], &facts) {
        Ok(_) => true,
        Err(oteryn_gameplay_domain::domain::condition::ConditionRefusal::KeptCurrent) => false,
        Err(_) => return false,
    };
    let Ok(mut expected) = replay.persistent_snapshot(0) else {
        return false;
    };
    if fresh {
        let Some(source_fire) = source_fire else {
            return false;
        };
        let Some(sequence) = source_fire
            .get("sequence")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
        else {
            return false;
        };
        let Some(prior) = before.get("conditions").and_then(Value::as_array) else {
            return false;
        };
        let maximum = prior
            .iter()
            .filter_map(|instance| instance.get("sequence").and_then(Value::as_u64))
            .max();
        if sequence == u32::MAX || maximum.is_some_and(|old| u64::from(sequence) <= old) {
            return false;
        }
        let Some(expected_fire) = expected
            .get_mut("conditions")
            .and_then(Value::as_array_mut)
            .and_then(|conditions| {
                conditions.iter_mut().find(|instance| {
                    instance.pointer("/definition/key").and_then(Value::as_str)
                        == Some("npc.ajax.source_fire")
                })
            })
        else {
            return false;
        };
        expected_fire["sequence"] = Value::from(sequence);
    }
    expected == *after
}

pub(crate) async fn persist_child(
    tx: &mut Type2Transaction<'_>,
    ctx: &NpcProgramContext<'_>,
    effect: &NpcSourceActorEffect,
) -> Result<(), DurabilityError> {
    if !effect.valid(ctx) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let written=sqlx::query("INSERT INTO game_npc_source_actor_effects(npc_transaction_id,character_id,game_session_id,command_id,source_key,program_sha256,committed_revision,observation) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7::text::numeric,$8)")
        .bind(ctx.npc_transaction.as_slice()).bind(ctx.fence.character_id.as_bytes().as_slice())
        .bind(ctx.fence.game_session_id.as_bytes().as_slice()).bind(ctx.command.command_id().get().to_string())
        .bind(ctx.source_key).bind(ctx.program_sha256.as_slice()).bind(ctx.next_revision.to_string()).bind(effect.observation())
        .execute(&mut ***tx).await?.rows_affected();
    if written != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    // The same XID parent receipt and private Node/actor observation already
    // exist. Snapshot triggers independently require this exact sealed after.
    character_condition_snapshot::write_in_transaction(tx, &ctx.fence.item_fence(), &effect.after)
        .await
}

impl crate::durability::DurabilityRoot {
    /// Read-only unknown-outcome resolution; never repeats FIRE or advances a
    /// source track. The live caller retains its original before/after memo.
    pub async fn read_npc_ajax_actor_effect(
        &self,
        authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
        fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
        request: super::super::NpcCosmeticRequest,
        effect: NpcSourceActorEffect,
    ) -> Result<bool, DurabilityError> {
        if request.npc != "oteryn:npc.ajax"
            || request.command.game_session_id() != fence.game_session_id
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let binding = super::super::binding(fence.character_id, &request)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            use sqlx::Row;
            let mut tx=crate::durability::db::begin_semantic_transaction(holder,deadline).await?;
            crate::durability::character_authority::assert_recovery_fence(&mut tx,&recovery).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:npc-cosmetic:'||encode($1,'hex')||':'||$2,0))").bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut *tx).await?;
            let row=sqlx::query("SELECT a.observation=$4::jsonb AS same_effect,r.request_binding=$5 AS same_request FROM game_npc_source_actor_effects a JOIN game_npc_cosmetic_receipts r ON r.transaction_id=a.npc_transaction_id WHERE a.character_id=encode($1,'hex')::uuid AND a.game_session_id=encode($2,'hex')::uuid AND a.command_id=$3::text::numeric AND a.source_key='oteryn:npc-cosmetic/ajax/barbarian-fire'")
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(effect.observation()).bind(binding).fetch_optional(&mut *tx).await?;
            let committed=if let Some(row)=row {
                if !row.try_get::<bool,_>("same_effect")? || !row.try_get::<bool,_>("same_request")? {return Err(DurabilityError::InvalidStoredState);}
                true
            }else {
                let orphan:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_npc_cosmetic_receipts WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric AND source_key='oteryn:npc-cosmetic/ajax/barbarian-fire')")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_one(&mut *tx).await?;
                if orphan {return Err(DurabilityError::InvalidStoredState);}false
            };
            crate::durability::db::commit_semantic_transaction(tx,deadline).await?;
            Ok(committed)
        })).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv7_ajax_actor_child_accepts_only_exact_native_fire_successor() -> Result<(), String> {
        use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement, StatusKind};
        use oteryn_gameplay_foundation::foundation::condition::{
            ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionStore,
        };
        use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
        let root = GameplayDecisionRoot::from_bytes([0; 32]);
        let facts = ApplicationFacts {
            now: 0,
            base_speed: 220,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes([0; 16]),
        };
        let mut state = ConditionStore::<String>::new();
        let drunk = ConditionDefinition::new(
            "source.fixture.drunk",
            1,
            ConditionValues::TimedStatus {
                kind: StatusKind::Drunk,
                duration_ms: 30000,
            },
        )
        .ok_or("definition")?;
        state
            .apply(&drunk, None, ConditionSourceKind::SelfUse, &[], &facts)
            .map_err(|e| format!("{e:?}"))?;
        let before = state.persistent_snapshot(0).map_err(|e| format!("{e:?}"))?;
        assert!(
            !exact_fire_successor(&before, &before),
            "unchanged non-Fire snapshot cannot satisfy the child"
        );
        let fire = ConditionDefinition::new(
            "npc.ajax.source_fire",
            1,
            ConditionValues::DamageOverTime {
                element: DotElement::Fire,
                total_min: 100,
                total_max: 100,
                per_tick: 10,
                interval_ms: 2000,
                delayed: true,
            },
        )
        .ok_or("fire")?;
        state
            .apply(&fire, None, ConditionSourceKind::SelfUse, &[], &facts)
            .map_err(|e| format!("{e:?}"))?;
        let after = state.persistent_snapshot(0).map_err(|e| format!("{e:?}"))?;
        assert!(exact_fire_successor(&before, &after));
        let mut wrong = after.clone();
        wrong["conditions"][1]["remaining_total"] = json!(99);
        assert!(
            !exact_fire_successor(&before, &wrong),
            "exact100damage is required"
        );
        let mut wrong = after.clone();
        wrong["conditions"][1]["sequence"] = json!(0);
        assert!(
            !exact_fire_successor(&before, &wrong),
            "new Fire cannot precede an old condition"
        );
        let mut wrong = after.clone();
        wrong["conditions"][0]["ends_at"] = json!(1);
        assert!(
            !exact_fire_successor(&before, &wrong),
            "unrelated Drunk state remains exact"
        );
        let mut gap = after;
        gap["conditions"][1]["sequence"] = json!(42);
        assert!(
            exact_fire_successor(&before, &gap),
            "expired allocator history is not persisted but ordering remains later"
        );
        Ok(())
    }
}
