//! Atomic multi-Character XP descendants and their immutable kill audit (PARTY-XP-1).
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, assert_gameplay_fence, decode_receipt, load_receipt,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_item_transaction::{SpellItemScopeAuthority, check_scope_transaction};
use super::{DurabilityError, hunting_xp};
use crate::domain::CharacterId;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

pub use super::db::commit_semantic_transaction as commit_party_kill_transaction;

type Result<T> = std::result::Result<T, CharacterProgressionError>;

#[derive(Debug)]
pub struct PartyKillXpMember<const N: usize> {
    pub fence: CurrentCharacterGameplayFence,
    pub request: ExperienceAwardRequest<N>,
    pub premium: bool,
    /// Proven membership of the frozen Task Board kill recipients.
    pub task_board_eligible: bool,
    /// Full dust award, independent of the member's XP share and stamina.
    pub forge_dust: Option<u32>,
    /// Current runtime stamina; durable stamina remains the upper bound.
    pub live_stamina: Option<u16>,
    pub shared: bool,
    pub party_id: Option<[u8; 16]>,
    pub party_revision: Option<u64>,
}

#[derive(Debug)]
pub struct PartyKillXpRequest<const N: usize> {
    pub death_key: Vec<u8>,
    pub principal: CharacterId,
    pub creature: String,
    pub base_xp: i64,
    pub death_at_ms: u64,
    pub death_at_us: u64,
    pub shared: bool,
    pub party_id: Option<[u8; 16]>,
    pub party_revision: Option<u64>,
    /// CharacterId order, protected by native Character sequencer slots at the caller.
    pub members: Vec<PartyKillXpMember<N>>,
}

#[derive(Debug)]
pub struct PartyKillXpAward {
    /// Existing committed base XP (before party sharing) and final awarded XP.
    pub raw: i64,
    pub gained: i64,
    pub character: CharacterId,
    /// Zero stamina can remove an award without redistributing it.
    pub xp: Option<ExperienceCommitOutcome>,
    pub forge_dust: Option<super::character_forge_dust::ForgeDustEntry>,
}

/// Existing Channel receipts keep their exact 52-byte framing. An Instance key
/// prefixes the same field layout with tag 2, making its 53-byte namespace disjoint.
/// Production callers can encode only a sealed committed lethal occurrence.
pub fn encode_death_key(death: crate::foundation::CreatureDeathOccurrenceKey) -> Vec<u8> {
    let mut key = Vec::with_capacity(53);
    match death.runtime_scope() {
        crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } => {
            key.extend_from_slice(world_id.as_bytes());
            key.extend_from_slice(channel_id.as_bytes());
        }
        crate::foundation::RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } => {
            key.push(2);
            key.extend_from_slice(world_id.as_bytes());
            key.extend_from_slice(&instance_id);
        }
    }
    key.extend_from_slice(&death.scope_ownership_generation().get().to_be_bytes());
    key.extend_from_slice(&death.actor_local_id().to_be_bytes());
    key.extend_from_slice(&death.actor_local_generation().to_be_bytes());
    key
}

/// Read-only tagged namespace decoding; grants no death or gameplay authority.
pub(crate) fn decoded_death_scope(
    death: &[u8],
) -> Option<(crate::foundation::RuntimeScopeRefV1, u64)> {
    use crate::foundation::{ChannelId, RuntimeScopeRefV1, WorldId};
    let (instance, fields) = match death {
        bytes if bytes.len() == 52 => (false, bytes),
        [2, fields @ ..] if fields.len() == 52 => (true, fields),
        _ => return None,
    };
    let world = WorldId::decode(&fields[..16]).ok()?;
    let scope = if instance {
        RuntimeScopeRefV1::instance(world, fields[16..32].try_into().ok()?).ok()?
    } else {
        RuntimeScopeRefV1::channel(world, ChannelId::decode(&fields[16..32]).ok()?)
    };
    let generation = u64::from_be_bytes(fields[32..40].try_into().ok()?);
    if generation == 0 || fields[40..44] == [0; 4] || fields[44..52] == [0; 8] {
        return None;
    }
    Some((scope, generation))
}

/// The same decoder fences XP and its native encounter descendants. Scope kind
/// is part of identity, including when Channel and Instance UUID bytes coincide.
pub(crate) fn death_matches_fence(death: &[u8], fence: &CurrentCharacterGameplayFence) -> bool {
    decoded_death_scope(death)
        == Some((fence.runtime_scope, fence.scope_ownership_generation.get()))
}

/// Holds the death lock through planning; an existing receipt is terminal data, never a grant.
pub async fn reconcile_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    scope: &SpellItemScopeAuthority,
    death: &[u8],
    principal: CharacterId,
    creature: &str,
    base_xp: i64,
    death_times: (u64, u64),
) -> Result<Option<Vec<PartyKillXpAward>>> {
    let (death_at_ms, death_at_us) = death_times;
    let death_at_ms =
        i64::try_from(death_at_ms).map_err(|_| CharacterProgressionError::InvalidInput)?;
    if decoded_death_scope(death) != Some((scope.runtime_scope(), scope.generation())) {
        return Err(CharacterProgressionError::AuthorityRejected);
    }
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:party-kill-xp:' || encode($1,'hex'),0))")
        .bind(death)
        .execute(&mut **tx)
        .await
        .map_err(DurabilityError::from)?;
    let receipt = sqlx::query("SELECT principal_id::text,creature_key,base_xp,death_at_ms,death_at_us::text,principal_count FROM game_party_kill_xp_receipts WHERE death_key=$1")
        .bind(death).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
    let Some(receipt) = receipt else {
        return Ok(None);
    };
    if super::character_progression::uuid_text(
        receipt
            .try_get("principal_id")
            .map_err(DurabilityError::from)?,
    )? != *principal.as_bytes()
        || receipt
            .try_get::<String, _>("creature_key")
            .map_err(DurabilityError::from)?
            != creature
        || receipt
            .try_get::<i64, _>("base_xp")
            .map_err(DurabilityError::from)?
            != base_xp
        || receipt
            .try_get::<i64, _>("death_at_ms")
            .map_err(DurabilityError::from)?
            != death_at_ms
        || receipt
            .try_get::<String, _>("death_at_us")
            .map_err(DurabilityError::from)?
            != death_at_us.to_string()
    {
        return Err(CharacterProgressionError::ConflictingOccurrence);
    }
    let rows = sqlx::query("SELECT character_id::text,reward_occurrence_id::text,xp_awarded,bounty_dust_key FROM game_party_kill_xp_members WHERE death_key=$1 ORDER BY character_id LIMIT 51")
        .bind(death).fetch_all(&mut **tx).await.map_err(DurabilityError::from)?;
    if rows.is_empty()
        || rows.len() > 50
        || rows.len()
            != usize::try_from(
                receipt
                    .try_get::<i16, _>("principal_count")
                    .map_err(DurabilityError::from)?,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?
    {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let character = CharacterId::from_bytes(super::character_progression::uuid_text(
            row.try_get("character_id").map_err(DurabilityError::from)?,
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        let occurrence = row
            .try_get::<Option<String>, _>("reward_occurrence_id")
            .map_err(DurabilityError::from)?;
        let xp = if let Some(id) = occurrence {
            let occurrence = super::character_progression::ExperienceRewardOccurrence::from_bytes(
                super::character_progression::uuid_text(&id)?,
            )?;
            let stored = load_receipt(tx, occurrence)
                .await?
                .ok_or(DurabilityError::InvalidStoredState)?;
            Some(ExperienceCommitOutcome::AlreadyCommitted(decode_receipt(
                &stored,
            )?))
        } else {
            None
        };
        let mut forge_dust = super::character_forge_dust::read_creature_gain(
            tx,
            character,
            forge_id(death, character, 0),
        )
        .await
        .map_err(|_| CharacterProgressionError::Unavailable(DurabilityError::Unavailable))?;
        if let Some(key) = row
            .try_get::<Option<Vec<u8>>, _>("bounty_dust_key")
            .map_err(DurabilityError::from)?
        {
            forge_dust = super::character_forge_dust::read_creature_gain(
                tx,
                character,
                forge_id(&key, character, 0),
            )
            .await
            .map_err(|_| CharacterProgressionError::Unavailable(DurabilityError::Unavailable))?;
            if forge_dust.is_none() {
                return Err(DurabilityError::InvalidStoredState.into());
            }
        }
        result.push(PartyKillXpAward {
            raw: base_xp,
            gained: row.try_get("xp_awarded").map_err(DurabilityError::from)?,
            character,
            xp,
            forge_dust,
        });
    }
    Ok(Some(result))
}

/// Called only inside the consuming owner's fenced transaction with all native slots held.
/// No partial award or audit can escape a failed member. The caller commits once on success.
pub async fn commit_in_transaction<const N: usize>(
    tx: &mut Transaction<'_, Postgres>,
    scope: &SpellItemScopeAuthority,
    node: &NodeIncarnationProof,
    input: PartyKillXpRequest<N>,
) -> Result<Vec<PartyKillXpAward>> {
    if input.members.is_empty()
        || input.members.len() > 50
        || input
            .members
            .windows(2)
            .any(|m| m[0].fence.character_id >= m[1].fence.character_id)
        || input.party_id.is_some() != input.party_revision.is_some()
        || input.base_xp < 0
        || input.death_at_us / 1000 != input.death_at_ms
        || input.creature.is_empty()
        || input.creature.len() > 256
        || decoded_death_scope(&input.death_key)
            != Some((scope.runtime_scope(), scope.generation()))
        || !input
            .members
            .iter()
            .any(|m| m.fence.character_id == input.principal)
        || input.members.iter().any(|m| {
            m.party_id.is_some() != m.party_revision.is_some() || (m.shared && m.party_id.is_none())
        })
        || input.members.iter().any(|member| {
            member.fence.runtime_scope != scope.runtime_scope()
                || member.fence.scope_ownership_generation.get() != scope.generation()
        })
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    if let Some(awards) = reconcile_in_transaction(
        tx,
        scope,
        &input.death_key,
        input.principal,
        &input.creature,
        input.base_xp,
        (input.death_at_ms, input.death_at_us),
    )
    .await?
    {
        return Ok(awards);
    }
    for member in &input.members {
        assert_gameplay_fence(tx, &member.fence, node).await??;
    }
    sqlx::query("INSERT INTO game_party_kill_xp_receipts(death_key,principal_id,creature_key,base_xp,death_at_ms,death_at_us,principal_count,shared,party_id,party_revision) VALUES($1,encode($2,'hex')::uuid,$3,$4,$5,$6::text::numeric,$7,$8,encode($9,'hex')::uuid,$10::text::numeric)")
        .bind(input.death_key.as_slice()).bind(input.principal.as_bytes().as_slice())
        .bind(&input.creature).bind(input.base_xp).bind(i64::try_from(input.death_at_ms).map_err(|_|CharacterProgressionError::InvalidInput)?).bind(input.death_at_us.to_string()).bind(i16::try_from(input.members.len()).map_err(|_|CharacterProgressionError::InvalidInput)?).bind(input.shared)
        .bind(input.party_id.as_ref().map(|id|id.as_slice())).bind(input.party_revision.map(|r|r.to_string()))
        .execute(&mut **tx).await.map_err(DurabilityError::from)?;
    super::world_event_outcomes::capture_kill(tx, &input).await?;
    super::world_event::capture_kill(tx, &input).await?;
    let mut awards = Vec::with_capacity(input.members.len());
    for member in input.members {
        let bounty = super::task_board::read_talisman_in_transaction(tx, member.fence).await?;
        let elite_dust = if member.premium && member.task_board_eligible {
            bounty.completion_dust(&input.creature)
        } else {
            0
        };
        let mut forge_dust = if let Some(amount) = member.forge_dust {
            use super::character_forge_dust::{
                ForgeDustGainCause, ForgeDustOutcome, ForgeDustWriteIds,
                gain_forge_dust_in_transaction,
            };
            let character = member.fence.character_id;
            let outcome = gain_forge_dust_in_transaction(
                tx,
                &member.fence,
                ForgeDustGainCause::CreatureKill {
                    occurrence: forge_id(&input.death_key, character, 0),
                },
                amount,
                ForgeDustWriteIds {
                    entry_id: forge_id(&input.death_key, character, 1),
                    transaction_id: forge_id(&input.death_key, character, 2),
                },
            )
            .await
            .map_err(|_| CharacterProgressionError::Unavailable(DurabilityError::Unavailable))?;
            Some(match outcome {
                ForgeDustOutcome::Written(entry) | ForgeDustOutcome::AlreadyWritten(entry) => entry,
            })
        } else {
            None
        };
        // A selection identity, rather than this death, fences completion dust. Another
        // death can settle before the separate Task Board credit has drained.
        let mut bounty_dust_key = (elite_dust > 0).then(|| bounty.selection_serial().to_be_bytes());
        if let Some(key) = bounty_dust_key {
            use super::character_forge_dust::{
                ForgeDustGainCause, ForgeDustOutcome, ForgeDustWriteIds,
                gain_forge_dust_in_transaction,
            };
            let character = member.fence.character_id;
            let outcome = gain_forge_dust_in_transaction(
                tx,
                &member.fence,
                ForgeDustGainCause::CreatureKill {
                    occurrence: forge_id(&key, character, 0),
                },
                elite_dust,
                ForgeDustWriteIds {
                    entry_id: forge_id(&key, character, 1),
                    transaction_id: forge_id(&key, character, 2),
                },
            )
            .await
            .map_err(|_| CharacterProgressionError::Unavailable(DurabilityError::Unavailable))?;
            match outcome {
                ForgeDustOutcome::Written(entry) => forge_dust = Some(entry),
                ForgeDustOutcome::AlreadyWritten(entry) if forge_dust.is_none() => {
                    forge_dust = Some(entry)
                }
                ForgeDustOutcome::AlreadyWritten(_) => bounty_dust_key = None,
            }
        }
        let mut modifiers =
            hunting_xp::read_in_transaction(tx, &member.fence, &input.creature, member.premium)
                .await?;
        if let Some(live) = member.live_stamina {
            modifiers.stamina_minutes = modifiers.stamina_minutes.min(i32::from(live));
        }
        let raw = member.request.amount.get();
        let amount = oteryn_gameplay_characters::hunting_experience::hunting_experience(
            raw,
            modifiers.prey_percent,
            modifiers.xp_boost_percent,
            modifiers.stamina_minutes,
            member.premium,
            modifiers.level,
        )
        .map_err(|_| CharacterProgressionError::InvalidInput)?;
        let mut request = member.request;
        request.amount = oteryn_simulation_determinism::ExactI64::new(amount);
        let xp = if amount > 0 {
            let outcome = super::character_experience_transaction::commit_in_transaction(
                tx,
                node,
                member.fence,
                &request,
                false,
                Some(hunting_binding(
                    &input.death_key,
                    raw,
                    modifiers,
                    member.party_id,
                    member.party_revision,
                    member.shared,
                )),
            )
            .await??;
            Some(outcome)
        } else {
            None
        };
        sqlx::query("INSERT INTO game_party_kill_xp_members(death_key,character_id,reward_occurrence_id,shared,party_id,party_revision,raw_share,prey_slot,prey_epoch,prey_grade,prey_percent,stamina_minutes,xp_awarded,xp_boost_percent,bounty_dust_key) VALUES($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid,$6::text::numeric,$7,$8,$9,$10,$11,$12,$13,$14,$15)")
            .bind(input.death_key.as_slice()).bind(member.fence.character_id.as_bytes().as_slice())
            .bind(xp.as_ref().map(|_|request.occurrence.as_bytes().as_slice()))
            .bind(member.shared).bind(member.party_id.as_ref().map(|id|id.as_slice())).bind(member.party_revision.map(|r|r.to_string()))
            .bind(raw).bind(modifiers.prey_slot).bind(modifiers.prey_epoch).bind(modifiers.prey_grade)
            .bind(modifiers.prey_percent).bind(modifiers.stamina_minutes).bind(amount).bind(modifiers.xp_boost_percent).bind(bounty_dust_key.as_ref().map(|key| key.as_slice()))
            .execute(&mut **tx).await.map_err(DurabilityError::from)?;
        awards.push(PartyKillXpAward {
            raw: input.base_xp,
            gained: amount,
            character: member.fence.character_id,
            xp,
            forge_dust,
        });
    }
    Ok(awards)
}

fn hunting_binding(
    death: &[u8],
    raw: i64,
    modifiers: hunting_xp::HuntingXpModifiers,
    party_id: Option<[u8; 16]>,
    party_revision: Option<u64>,
    shared: bool,
) -> [u8; 32] {
    let mut binding = Vec::new();
    binding.extend_from_slice(death);
    binding.extend_from_slice(&raw.to_be_bytes());
    binding.push(u8::from(shared));
    binding.extend_from_slice(&party_id.unwrap_or([0; 16]));
    binding.extend_from_slice(&party_revision.unwrap_or(0).to_be_bytes());
    binding.extend_from_slice(&modifiers.prey_slot.unwrap_or(0).to_be_bytes());
    binding.extend_from_slice(&modifiers.prey_epoch.unwrap_or(0).to_be_bytes());
    binding.extend_from_slice(&modifiers.prey_grade.unwrap_or(0).to_be_bytes());
    binding.extend_from_slice(&modifiers.prey_percent.to_be_bytes());
    binding.extend_from_slice(&modifiers.xp_boost_percent.to_be_bytes());
    binding.extend_from_slice(&modifiers.stamina_minutes.to_be_bytes());
    // Appended only while the bonus applies, so level-50+ bindings stay as before PAR-C14.
    if modifiers.level < 50 {
        binding.extend_from_slice(&modifiers.level.to_be_bytes());
    }
    Sha256::digest(binding).into()
}

// The full scope/death/Character tuple is the source identity. Tags separate the ledger
// occurrence, entry and transaction; UUIDv7 shape is required by the existing ledger owner.
fn forge_id(death: &[u8], character: CharacterId, tag: u8) -> [u8; 16] {
    let hash = Sha256::new()
        .chain_update(b"FORGECREATURE/dust/v1")
        .chain_update(death)
        .chain_update(character.as_bytes())
        .chain_update([tag])
        .finalize();
    let mut id = [0; 16];
    id.copy_from_slice(&hash[..16]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    id
}

#[cfg(test)]
mod scoped_death_tests {
    use super::*;

    fn id(seed: u8) -> [u8; 16] {
        let mut value = [seed; 16];
        value[6] = 0x70;
        value[8] = 0x80;
        value
    }
    fn channel_bytes() -> Vec<u8> {
        [
            id(1).as_slice(),
            id(2).as_slice(),
            3_u64.to_be_bytes().as_slice(),
            4_u32.to_be_bytes().as_slice(),
            5_u64.to_be_bytes().as_slice(),
        ]
        .concat()
    }

    #[test]
    fn native_death_fence_keeps_scope_kind_world_and_generation() -> std::result::Result<(), String>
    {
        use crate::foundation::{ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};
        let channel = channel_bytes();
        let mut instance = vec![2];
        instance.extend_from_slice(&channel);
        let (channel_scope, generation) = decoded_death_scope(&channel).ok_or("channel source")?;
        let (instance_scope, _) = decoded_death_scope(&instance).ok_or("Instance source")?;
        let mut fence = CurrentCharacterGameplayFence {
            character_id: CharacterId::from_bytes(id(3)).map_err(|e| format!("{e:?}"))?,
            game_session_id: GameSessionId::decode(&id(4)).map_err(|e| format!("{e:?}"))?,
            connection_generation: ConnectionGeneration::new(1).map_err(|e| format!("{e:?}"))?,
            character_lease_generation: 1,
            runtime_scope: channel_scope,
            scope_ownership_generation: ScopeOwnershipGeneration::new(generation)
                .map_err(|e| format!("{e:?}"))?,
            expected_character_revision: crate::domain::CharacterRevision::new(1)
                .map_err(|e| format!("{e:?}"))?,
        };
        assert!(death_matches_fence(&channel, &fence));
        assert!(!death_matches_fence(&instance, &fence));
        fence.runtime_scope = instance_scope;
        assert!(death_matches_fence(&instance, &fence));
        assert!(!death_matches_fence(&channel, &fence));
        let mut foreign = instance.clone();
        foreign[1..17].copy_from_slice(&id(9));
        assert!(!death_matches_fence(&foreign, &fence));
        foreign = instance.clone();
        foreign[0] = 1;
        assert!(!death_matches_fence(&foreign, &fence));
        for length in 0..instance.len() {
            assert!(!death_matches_fence(&instance[..length], &fence));
        }
        fence.scope_ownership_generation =
            ScopeOwnershipGeneration::new(generation + 1).map_err(|e| format!("{e:?}"))?;
        assert!(!death_matches_fence(&instance, &fence));
        Ok(())
    }

    #[test]
    fn party_kill_death_scope_decoder_preserves_channel_and_separates_instance() {
        let channel = channel_bytes();
        let mut instance = vec![2];
        instance.extend_from_slice(&channel);
        let legacy = decoded_death_scope(&channel);
        let private = decoded_death_scope(&instance);
        assert!(matches!(
            legacy,
            Some((crate::foundation::RuntimeScopeRefV1::Channel { .. }, 3))
        ));
        assert!(matches!(
            private,
            Some((crate::foundation::RuntimeScopeRefV1::Instance { .. }, 3))
        ));
        assert_ne!(legacy, private);
        assert_eq!(channel.len(), 52);
        assert_eq!(instance.len(), 53);
        for length in 0..52 {
            assert_eq!(decoded_death_scope(&channel[..length]), None);
        }
        instance[0] = 1;
        assert_eq!(decoded_death_scope(&instance), None);
        let mut zero_actor = channel.clone();
        zero_actor[40..44].fill(0);
        assert_eq!(decoded_death_scope(&zero_actor), None);
        let mut zero_generation = channel;
        zero_generation[32..40].fill(0);
        assert_eq!(decoded_death_scope(&zero_generation), None);
    }
}
