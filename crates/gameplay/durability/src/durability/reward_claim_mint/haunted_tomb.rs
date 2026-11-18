//! A signed-in Channel player's one sarcophagus use and its optional timed key.
use super::*;
use crate::durability::{
    character_equipment::{EquipmentError, assert_current_equipment_in_transaction},
    item_loop_move::ItemLoopContent,
    item_mint_audit::Type2Transaction,
    spell_items_abi::QualifiedItemDefinition,
    spell_owner_commit::SpellLanePermit,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_items::{haunted_cellar, haunted_tomb};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HauntedTombUse {
    Ineligible,
    CoolingDown,
    Curse,
    Key,
    NoRoom,
    Replay,
}

#[derive(Clone, Copy)]
pub struct HauntedTombSource {
    pub placement: [u8; 8],
    pub map_digest: [u8; 32],
    pub expected_revision: u64,
    pub roll: u32,
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_haunted_tomb_use<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        holder: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        source: HauntedTombSource,
        native: &NativeGameplayState,
        content: &C,
    ) -> std::result::Result<HauntedTombUse, DurabilityError> {
        if fence.game_session_id != command.game_session_id()
            || native.source_digest() != content.loop_content_digest()
        {
            return Err(DurabilityError::CommitRejected);
        }
        permit.check_scope(fence.runtime_scope)?;
        let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::CommitRejected)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let holder = holder.clone();
        let mut context = (native, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |connection, deadline, context| Box::pin(async move {
            let mut tx = begin_type2_transaction(connection, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let (native, content) = *context;
            assert_current_equipment_in_transaction(&mut tx, &recovery, &holder, &fence, Some(command), native.source_digest()).await.map_err(|error| match error {
                EquipmentError::Durability(error) => error,
                EquipmentError::Database(error) => error.into(),
                EquipmentError::Rejected(_) => DurabilityError::CommitRejected,
            })?;
            lock_cause(&mut tx, command).await?;
            let previous: Option<bool> = sqlx::query_scalar(
                "SELECT uuid_send(character_id)=$3 AND map_digest=$4 AND placement=$5 AND source_revision=$6::text::numeric
                 FROM game_haunted_tomb_uses WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(command.game_session_id().as_bytes().as_slice())
                .bind(command.command_id().get().to_string())
                .bind(fence.character_id.as_bytes().as_slice())
                .bind(source.map_digest.as_slice()).bind(source.placement.as_slice())
                .bind(source.expected_revision.to_string())
                .fetch_optional(&mut **tx).await?;
            if let Some(matches) = previous {
                if !matches {
                    return Err(DurabilityError::CommitRejected);
                }
                tx.commit(deadline).await?;
                return Ok(HauntedTombUse::Replay);
            }
            let (questline, tomb, ruleset, simulation): (i64, i64, String, String) = sqlx::query_as(
                "SELECT coalesce(q.value,-1),coalesce(t.value,-1),s.ruleset_revision,s.simulation_revision
                 FROM game_character_progression_state s
                 LEFT JOIN game_character_quest_tracks q ON q.character_id=s.character_id AND q.track_key=$2
                 LEFT JOIN game_character_quest_tracks t ON t.character_id=s.character_id AND t.track_key=$3
                 WHERE s.character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice())
                .bind(haunted_cellar::QUESTLINE).bind(haunted_tomb::TOMB_TRACK)
                .fetch_one(&mut **tx).await?;
            let Some(outcome) = haunted_tomb::select(questline, tomb, source.roll) else {
                return Ok(HauntedTombUse::Ineligible);
            };
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:haunted-tomb:'||encode($1,'hex')||':'||encode($2,'hex')||':'||encode($3,'hex'),0))")
                .bind(channel.as_bytes().as_slice()).bind(source.map_digest.as_slice()).bind(source.placement.as_slice())
                .execute(&mut **tx).await?;
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&mut **tx).await?;
            let generation = fence.scope_ownership_generation.get().to_string();
            let busy: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_haunted_tomb_uses
                  WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid
                    AND scope_generation=$3::text::numeric AND map_digest=$4 AND placement=$5
                    AND expires_at>$6)")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                .bind(&generation).bind(source.map_digest.as_slice()).bind(source.placement.as_slice())
                .bind(now).fetch_one(&mut **tx).await?;
            if busy {
                return Ok(HauntedTombUse::CoolingDown);
            }
            let mut key_mint = None;
            let result = match outcome {
                haunted_tomb::Outcome::Curse => HauntedTombUse::Curse,
                haunted_tomb::Outcome::Key => {
                    match grant_key(&mut tx, &holder, fence, command, native, content, ruleset, simulation).await? {
                        Ok(grant) => { key_mint = Some(grant); HauntedTombUse::Key }
                        Err(RewardClaimMintError::Refused(
                            RewardClaimRefusal::NoMainBackpack | RewardClaimRefusal::MainBackpackFull | RewardClaimRefusal::CarryingCapacity
                        )) => HauntedTombUse::NoRoom,
                        Err(_) => return Err(DurabilityError::CommitRejected),
                    }
                }
            };
            let expires = now.checked_add(i64::from(haunted_tomb::COOLDOWN_MS))
                .ok_or(DurabilityError::InvalidStoredState)?;
            let reward_tx = key_mint.as_ref().map(|grant| grant.transaction_id);
            sqlx::query("INSERT INTO game_haunted_tomb_uses(game_session_id,command_id,character_id,world_id,channel_id,scope_generation,map_digest,placement,source_revision,roll,occurred_at,expires_at,reward_transaction_id)
                VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7,$8,$9::text::numeric,$10,$11,$12,encode($13,'hex')::uuid)")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(generation)
                .bind(source.map_digest.as_slice()).bind(source.placement.as_slice()).bind(source.expected_revision.to_string())
                .bind(i16::try_from(source.roll).map_err(|_| DurabilityError::CommitRejected)?)
                .bind(now).bind(expires).bind(reward_tx.as_ref().map(|bytes| bytes.as_slice()))
                .execute(&mut **tx).await?;
            if let Some(grant) = key_mint {
                let duration = i64::from(haunted_tomb::KEY_LIFETIME_MS);
                sqlx::query("INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,source_ordinal,duration_millis,expires_at_unix_ms,definition_family,definition_production_key,definition_revision,content_digest)
                    VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3,$4,'Item','oteryn:item.tibia.i29351','definition-r1',$5)")
                    .bind(grant.item_instance_id.as_slice()).bind(grant.transaction_id.as_slice()).bind(duration)
                    .bind(grant.occurred_at_unix_ms.checked_add(duration).ok_or(DurabilityError::InvalidStoredState)?)
                    .bind(native.source_digest().as_slice()).execute(&mut **tx).await?;
            }
            tx.commit(deadline).await?;
            Ok(result)
        })).await
    }
}

#[allow(clippy::too_many_arguments)]
async fn grant_key<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    holder: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    native: &NativeGameplayState,
    content: &C,
    ruleset_revision: String,
    sim_revision: String,
) -> Pass<CommittedRewardClaimMint> {
    let Some(policy) = native.item_policy_for_source_id(haunted_tomb::KEY) else {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    };
    let definition = QualifiedItemDefinition::from_native_policy(policy)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if definition.decay.is_none_or(|decay| {
        decay.target.is_some() || decay.duration_millis != haunted_tomb::KEY_LIFETIME_MS
    }) {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    }
    let Some(slot) = load_slot(tx, fence.character_id).await? else {
        return Ok(Err(RewardClaimMintError::Refused(
            RewardClaimRefusal::NoMainBackpack,
        )));
    };
    let (Some(item), Some(backpack)) = (
        content.loop_facts(&definition.definition),
        content.loop_facts(&slot.definition),
    ) else {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    };
    let request = RewardClaimMintRequest {
        backpack_outputs: false,
        additional_items: Vec::new(),
        command,
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: format!(
                "oteryn:reward-claim.haunted-tomb.{}.{}",
                hex(command.game_session_id().as_bytes()),
                command.command_id().get()
            ),
            revision_ref: "haunted-tomb-r1".into(),
        },
        source_placement: "oteryn:haunted-tomb/sarcophagus".into(),
        item,
        quantity: 1,
        contents: Vec::new(),
        root_text: None,
        root_fluid_subtype: None,
        root_charges_remaining: None,
        backpack,
        content_revision: format!("sha256:{}", hex(&native.source_digest())),
        ruleset_revision,
        sim_revision,
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    super::marlin_trophy::commit_prepared_source_grant(tx, holder, fence, content, request).await
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
