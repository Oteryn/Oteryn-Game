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
    /// Current runtime stamina; durable stamina remains the upper bound.
    pub live_stamina: Option<u16>,
    pub shared: bool,
    pub party_id: Option<[u8; 16]>,
    pub party_revision: Option<u64>,
}

#[derive(Debug)]
pub struct PartyKillXpRequest<const N: usize> {
    pub death_key: [u8; 52],
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
    pub character: CharacterId,
    /// Zero stamina can remove an award without redistributing it.
    pub xp: Option<ExperienceCommitOutcome>,
}

/// Holds the death lock through planning; an existing receipt is terminal data, never a grant.
pub async fn reconcile_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    scope: &SpellItemScopeAuthority,
    death: &[u8; 52],
    principal: CharacterId,
    creature: &str,
    base_xp: i64,
    death_times: (u64, u64),
) -> Result<Option<Vec<PartyKillXpAward>>> {
    let (death_at_ms, death_at_us) = death_times;
    let death_at_ms =
        i64::try_from(death_at_ms).map_err(|_| CharacterProgressionError::InvalidInput)?;
    if death[..16] != scope.world_bytes() || death[16..32] != scope.channel_bytes() {
        return Err(CharacterProgressionError::AuthorityRejected);
    }
    check_scope_transaction(tx, scope)
        .await
        .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:party-kill-xp:' || encode($1,'hex'),0))")
        .bind(death.as_slice())
        .execute(&mut **tx)
        .await
        .map_err(DurabilityError::from)?;
    let receipt = sqlx::query("SELECT principal_id::text,creature_key,base_xp,death_at_ms,death_at_us::text,principal_count FROM game_party_kill_xp_receipts WHERE death_key=$1")
        .bind(death.as_slice()).fetch_optional(&mut **tx).await.map_err(DurabilityError::from)?;
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
    let rows = sqlx::query("SELECT character_id::text,reward_occurrence_id::text FROM game_party_kill_xp_members WHERE death_key=$1 ORDER BY character_id LIMIT 51")
        .bind(death.as_slice()).fetch_all(&mut **tx).await.map_err(DurabilityError::from)?;
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
        result.push(PartyKillXpAward { character, xp });
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
        || input.death_key[..16] != scope.world_bytes()
        || input.death_key[16..32] != scope.channel_bytes()
        || input.death_key[32..40] != scope.generation().to_be_bytes()
        || !input
            .members
            .iter()
            .any(|m| m.fence.character_id == input.principal)
        || input.death_key[40..44] == [0; 4]
        || input.death_key[44..52] == [0; 8]
        || input.members.iter().any(|m| {
            m.party_id.is_some() != m.party_revision.is_some() || (m.shared && m.party_id.is_none())
        })
        || input.members.iter().any(|m| match m.fence.runtime_scope {
            crate::foundation::RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } => {
                *world_id.as_bytes() != scope.world_bytes()
                    || *channel_id.as_bytes() != scope.channel_bytes()
                    || m.fence.scope_ownership_generation.get() != scope.generation()
            }
            _ => true,
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
    let mut awards = Vec::with_capacity(input.members.len());
    for member in input.members {
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
        sqlx::query("INSERT INTO game_party_kill_xp_members(death_key,character_id,reward_occurrence_id,shared,party_id,party_revision,raw_share,prey_slot,prey_epoch,prey_grade,prey_percent,stamina_minutes,xp_awarded,xp_boost_percent) VALUES($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid,$6::text::numeric,$7,$8,$9,$10,$11,$12,$13,$14)")
            .bind(input.death_key.as_slice()).bind(member.fence.character_id.as_bytes().as_slice())
            .bind(xp.as_ref().map(|_|request.occurrence.as_bytes().as_slice()))
            .bind(member.shared).bind(member.party_id.as_ref().map(|id|id.as_slice())).bind(member.party_revision.map(|r|r.to_string()))
            .bind(raw).bind(modifiers.prey_slot).bind(modifiers.prey_epoch).bind(modifiers.prey_grade)
            .bind(modifiers.prey_percent).bind(modifiers.stamina_minutes).bind(amount).bind(modifiers.xp_boost_percent)
            .execute(&mut **tx).await.map_err(DurabilityError::from)?;
        awards.push(PartyKillXpAward {
            character: member.fence.character_id,
            xp,
        });
    }
    Ok(awards)
}

fn hunting_binding(
    death: &[u8; 52],
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
    Sha256::digest(binding).into()
}
