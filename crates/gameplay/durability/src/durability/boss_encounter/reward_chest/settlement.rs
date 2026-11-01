use super::*;
use crate::durability::activity_handoff::boss_room::{
    BossRoomDefeatDecision, classify_and_record_defeat_in_transaction,
};
use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::character_progression::assert_runtime_scope_fence;
use crate::durability::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_encounter_rules::boss::reward_chest::reward_expiry;
use serde_json::json;

impl DurabilityRoot {
    /// Records all frozen personal rewards in one semantic transaction. Only the current
    /// server scope can issue a new death. Retained participant issuance is proved separately;
    /// going offline after dealing damage cannot discard an earned reward. A matching death
    /// receipt answers before current-owner admission, so recovery cannot reroll or mint twice.
    /// `true` also acknowledges a retained, explicitly late private-room death: it awards no
    /// items, and it must drain so the existing timeout return can finish.
    pub async fn settle_boss_reward_chests(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        mut death: BossRewardDeath,
    ) -> Result<bool, DurabilityError> {
        death
            .contributors
            .sort_by_key(|participant| *participant.character_id.as_bytes());
        death
            .chests
            .sort_by_key(|chest| *chest.character_id.as_bytes());
        let Some(expires) = validate_death(&death) else {
            return Ok(false);
        };
        let intent = intent(scope, generation, &death)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let admissions = FreshAdmissionStore::from_root(self.clone());
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let world = scope.world_id();
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:boss-reward-death:' || encode($1,'hex') || ':' || encode($2,'hex'),0))")
                .bind(world.as_bytes().as_slice()).bind(death.death.as_slice()).execute(&mut *tx).await?;
            let previous: Option<Vec<u8>> = sqlx::query_scalar("SELECT intent FROM game_boss_reward_deaths WHERE world_id=encode($1,'hex')::uuid AND death_id=encode($2,'hex')::uuid")
                .bind(world.as_bytes().as_slice()).bind(death.death.as_slice()).fetch_optional(&mut *tx).await?;
            if let Some(previous) = previous {
                if previous != intent { return Err(DurabilityError::InvalidStoredState); }
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(true);
            }
            if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? { return Ok(false); }
            // Stable Character order prevents sibling death settlements from reversing locks.
            for participant in &death.contributors {
                let character = crate::foundation::CharacterId::decode(participant.character_id.as_bytes())
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                if !admissions.committed_session_use(&mut tx, character, participant.session_id).await? {
                    return Ok(false);
                }
                // The original session retains its issued Character lease even after
                // reconnect or terminalization. This proves the captured tuple without
                // requiring an offline contributor to hold the current Character lease.
                let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_roots r JOIN game_durability_reconnect_sessions s USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid AND r.world_id=encode($2,'hex')::uuid AND s.world_id=r.world_id AND s.game_session_id=encode($3,'hex')::uuid AND s.character_lease_generation=$4::text::numeric)")
                    .bind(participant.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
                    .bind(participant.session_id.as_bytes().as_slice()).bind(participant.lease_generation.to_string()).fetch_one(&mut *tx).await?;
                if !exists { return Ok(false); }
            }
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut *tx).await?;
            let minted = i64::try_from(death.minted_at_unix_ms).map_err(|_| DurabilityError::InvalidStoredState)?;
            if minted > now { return Ok(false); }
            let (kind, channel, instance) = match scope {
                RuntimeScopeRefV1::Channel { channel_id, .. } => (1_i16, Some(*channel_id.as_bytes()), None),
                RuntimeScopeRefV1::Instance { instance_id, .. } => (2, None, Some(instance_id)),
            };
            // A permanently late death cannot remain a transient retry: return seals
            // require this queue to drain. Record its exact census and immutable intent
            // without a chest, then let the existing timeout writer close the room.
            // Current authority and conflicting/future death checks still fail closed.
            let outcome = match classify_and_record_defeat_in_transaction(
                &mut tx, scope, generation, &death.boss_key, death.death, minted,
            ).await? {
                BossRoomDefeatDecision::Recorded | BossRoomDefeatDecision::Unrelated => 1_i16,
                BossRoomDefeatDecision::Late => 2,
                BossRoomDefeatDecision::Conflict => return Ok(false),
            };
            sqlx::query("INSERT INTO game_boss_reward_deaths(world_id,death_id,scope_kind,channel_id,instance_id,scope_generation,boss_key,minted_at,expires_at,participant_count,eligible_count,intent,settlement_outcome) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7,$8,$9,$10,$11,$12,$13)")
                .bind(world.as_bytes().as_slice()).bind(death.death.as_slice()).bind(kind)
                .bind(channel.as_ref().map(|id| id.as_slice())).bind(instance.as_ref().map(|id| id.as_slice()))
                .bind(generation.get().to_string()).bind(&death.boss_key).bind(minted).bind(expires)
                .bind(death.contributors.len() as i32).bind(death.chests.len() as i32).bind(&intent).bind(outcome).execute(&mut *tx).await?;
            for participant in &death.contributors {
                let chest = death.chests.iter().find(|chest| chest.character_id == participant.character_id);
                sqlx::query("INSERT INTO game_boss_reward_contributors(character_id,world_id,death_id,source_game_session_id,source_lease_generation,damage_dealt,damage_received,healing_others,eligible,item_count) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6::text::numeric,$7::text::numeric,$8::text::numeric,$9,$10)")
                    .bind(participant.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(death.death.as_slice())
                    .bind(participant.session_id.as_bytes().as_slice()).bind(participant.lease_generation.to_string())
                    .bind(participant.contribution.damage_dealt.to_string()).bind(participant.contribution.damage_received.to_string())
                    .bind(participant.contribution.healing_others.to_string()).bind(chest.is_some())
                    .bind(if outcome == 2 { 0 } else { chest.map_or(0, |chest| chest.items.len()) as i32 }).execute(&mut *tx).await?;
            }
            if outcome == 2 {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(true);
            }
            for chest in death.chests {
                sqlx::query("INSERT INTO game_character_boss_reward_chests(character_id,world_id,death_id,boss_key,minted_at,expires_at,item_count) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6,$7)")
                    .bind(chest.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(death.death.as_slice())
                    .bind(&death.boss_key).bind(minted).bind(expires).bind(if now >= expires { 0_i32 } else { chest.items.len() as i32 })
                    .execute(&mut *tx).await?;
                for (index, item) in chest.items.into_iter().enumerate() {
                    sqlx::query("INSERT INTO game_boss_reward_items(character_id,world_id,death_id,item_ordinal,definition_family,definition_production_key,definition_revision_ref,quantity,expired) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6,$7,$8,$9)")
                        .bind(chest.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(death.death.as_slice())
                        .bind((index + 1) as i32).bind(item.definition.family).bind(item.definition.production_key)
                        .bind(item.definition.revision_ref).bind(i64::from(item.quantity)).bind(now >= expires).execute(&mut *tx).await?;
                }
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(true)
        })).await
    }
}

fn validate_death(death: &BossRewardDeath) -> Option<i64> {
    use crate::durability::item_mint_audit::check_content_key;
    if death.death == [0; 16]
        || death.minted_at_unix_ms == 0
        || check_content_key(&death.boss_key).is_err()
        || death.chests.len() > BOSS_REWARD_PARTICIPANTS_MAX
    {
        return None;
    }
    if death.contributors.len() > BOSS_REWARD_CONTRIBUTORS_MAX {
        return None;
    }
    let mut census = std::collections::BTreeMap::new();
    for participant in &death.contributors {
        if participant.lease_generation == 0
            || census
                .insert(participant.character_id, participant)
                .is_some()
        {
            return None;
        }
    }
    let eligibility = oteryn_gameplay_encounter_rules::boss::reward_eligibility(
        &death
            .contributors
            .iter()
            .map(|participant| participant.contribution)
            .collect::<Vec<_>>(),
    );
    let expected: std::collections::BTreeSet<_> = death
        .contributors
        .iter()
        .zip(eligibility)
        .filter(|(_, eligible)| *eligible)
        .map(|(participant, _)| participant.character_id)
        .collect();
    let mut seen = std::collections::BTreeSet::new();
    for chest in &death.chests {
        let participant = census.get(&chest.character_id)?;
        if !seen.insert(chest.character_id)
            || !expected.contains(&chest.character_id)
            || chest.session_id != participant.session_id
            || chest.lease_generation != participant.lease_generation
            || chest.contribution != participant.contribution
            || chest.items.len() > BOSS_REWARD_ITEMS_MAX
        {
            return None;
        }
        for item in &chest.items {
            if item.definition.family != "Item"
                || item.quantity == 0
                || check_content_key(&item.definition.production_key).is_err()
                || check_content_key(&item.definition.revision_ref).is_err()
            {
                return None;
            }
        }
    }
    if seen != expected {
        return None;
    }
    i64::try_from(reward_expiry(death.minted_at_unix_ms)?).ok()
}

fn intent(
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    death: &BossRewardDeath,
) -> Result<Vec<u8>, DurabilityError> {
    let (kind, scope_id) = match scope {
        RuntimeScopeRefV1::Channel { channel_id, .. } => (1, *channel_id.as_bytes()),
        RuntimeScopeRefV1::Instance { instance_id, .. } => (2, instance_id),
    };
    let contributors = death.contributors.iter().map(|participant| json!({
        "character":participant.character_id.as_bytes(), "session":participant.session_id.as_bytes(),
        "lease":participant.lease_generation, "dealt":participant.contribution.damage_dealt,
        "received":participant.contribution.damage_received, "healed":participant.contribution.healing_others,
    })).collect::<Vec<_>>();
    let participants = death.chests.iter().map(|chest| json!({
        "character": chest.character_id.as_bytes(), "session": chest.session_id.as_bytes(),
        "lease": chest.lease_generation, "dealt": chest.contribution.damage_dealt,
        "received": chest.contribution.damage_received, "healed": chest.contribution.healing_others,
        "items": chest.items.iter().map(|item| json!({"family":item.definition.family,
            "key":item.definition.production_key,"revision":item.definition.revision_ref,"quantity":item.quantity})).collect::<Vec<_>>()
    })).collect::<Vec<_>>();
    serde_json::to_vec(&json!({"world":scope.world_id().as_bytes(),"scope_kind":kind,
        "scope":scope_id,"generation":generation.get(),"death":death.death,
        "boss":death.boss_key,"minted":death.minted_at_unix_ms,"contributors":contributors,"participants":participants}))
        .map_err(|_| DurabilityError::InvalidStoredState)
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "bounded test identities")]
mod tests {
    use super::*;
    fn death() -> BossRewardDeath {
        let mut character = [1; 16];
        character[6] = 0x71;
        character[8] = 0x81;
        BossRewardDeath {
            death: [2; 16],
            boss_key: "oteryn:boss.test".into(),
            minted_at_unix_ms: 100,
            contributors: vec![BossRewardContributor {
                character_id: CharacterId::from_bytes(character).expect("character"),
                session_id: GameSessionId::decode(&character).expect("session"),
                lease_generation: 1,
                contribution: RewardParticipation {
                    damage_dealt: 1,
                    damage_received: 0,
                    healing_others: 0,
                },
            }],
            chests: vec![BossRewardChestPlan {
                character_id: CharacterId::from_bytes(character).expect("character"),
                session_id: GameSessionId::decode(&character).expect("session"),
                lease_generation: 1,
                contribution: RewardParticipation {
                    damage_dealt: 1,
                    damage_received: 0,
                    healing_others: 0,
                },
                items: vec![BossRewardItem {
                    definition: TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.test".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    quantity: 1,
                }],
            }],
        }
    }

    #[test]
    fn boss_reward_settlement_refuses_duplicates_missing_participation_and_overflow() {
        let base = death();
        assert!(validate_death(&base).is_some());
        let mut duplicate = base.clone();
        duplicate.chests.push(duplicate.chests[0].clone());
        assert!(validate_death(&duplicate).is_none());
        let mut missing = base.clone();
        missing.chests[0].contribution.damage_dealt = 0;
        assert!(validate_death(&missing).is_none());
        let mut invalid = base;
        invalid.minted_at_unix_ms = u64::MAX;
        assert!(validate_death(&invalid).is_none());
    }
    #[test]
    fn boss_reward_settlement_retains_support_census_and_requires_exact_eligibility() {
        let mut value = death();
        let mut character = [3; 16];
        character[6] = 0x73;
        character[8] = 0x83;
        value.contributors.push(BossRewardContributor {
            character_id: CharacterId::from_bytes(character).expect("character"),
            session_id: GameSessionId::decode(&character).expect("session"),
            lease_generation: 2,
            contribution: RewardParticipation {
                damage_dealt: 0,
                damage_received: 900,
                healing_others: 900,
            },
        });
        assert!(validate_death(&value).is_some());
        let mut omitted = value.clone();
        omitted.chests.clear();
        assert!(validate_death(&omitted).is_none());
        let mut wrong_tuple = value.clone();
        wrong_tuple.chests[0].contribution.damage_received = 1;
        assert!(validate_death(&wrong_tuple).is_none());
        value.contributors.push(value.contributors[0].clone());
        assert!(validate_death(&value).is_none());
    }
}
