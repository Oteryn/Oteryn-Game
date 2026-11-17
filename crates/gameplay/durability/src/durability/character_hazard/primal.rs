//! Primal Menace death consumer inside the native party-kill transaction.
//!
//! Crystal 201189f, creaturescripts_the_primal_menace_killed.lua:3-18 and
//! data/libs/systems/hazard.lua:116-126: contributors at the group's current
//! minimum advance only when their selected level is their own maximum.
use super::super::DurabilityError;
use super::super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root,
};
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use crate::domain::CharacterRevision;
use crate::foundation::RuntimeScopeRefV1;
use oteryn_gameplay_encounter_rules::hazard::unlocked_after_primal_menace;
use sqlx::{Postgres, Row, Transaction};

const PRIMAL_MENACE: &str = "oteryn:creature.the_primal_menace";
/// Crystal 201189f, data-global/scripts/quests/the_primal_ordeal/
/// creaturescripts_the_primal_menace_killed.lua:10-12: completion is independent of Hazard gain.
pub const PRIMAL_MENACE_KILLED_TRACK: &str =
    "oteryn:quest-progress/quest/u12_90/primal_ordeal/bosses/the_primal_menace_killed";
// Crystal the_primal_manace.lua:8-14 has exactly five participant slots.
const MAX_CONTRIBUTORS: usize = 5;
type Result<T> = std::result::Result<T, CharacterProgressionError>;

struct Contributor {
    fence: CurrentCharacterGameplayFence,
    unlocked: u16,
    selected: u16,
}

/// Run after `party_kill_xp::commit_in_transaction`, before its single commit.
///
/// `participants` is the native captured death census; the bool is a real damage
/// contribution, not a positive XP share (zero stamina must not lose progress).
/// All Character sequencer slots and roots remain held by the caller. This is
/// not a client command or an alternate source of combat/death authority.
pub async fn apply_hazard_kill_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    death_key: &[u8],
    creature: &str,
    participants: &[(CurrentCharacterGameplayFence, bool)],
) -> Result<()> {
    if creature != PRIMAL_MENACE {
        return Ok(());
    }
    let mut fences: Vec<_> = participants
        .iter()
        .filter(|(_, dealt_damage)| *dealt_damage)
        .map(|(fence, _)| *fence)
        .collect();
    fences.sort_by_key(|fence| fence.character_id);
    if fences.is_empty()
        || fences.len() > MAX_CONTRIBUTORS
        || fences
            .windows(2)
            .any(|pair| pair[0].character_id == pair[1].character_id)
        || fences.iter().any(|fence| {
            !death_matches_scope(
                death_key,
                fence.runtime_scope,
                fence.scope_ownership_generation.get(),
            )
        })
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let contributors: Vec<u8> = fences
        .iter()
        .flat_map(|fence| fence.character_id.as_bytes().iter().copied())
        .collect();
    let mut members = Vec::with_capacity(fences.len());
    for fence in fences {
        members.push(load_contributor(tx, node, death_key, fence).await?);
    }
    let receipts: Vec<(Vec<u8>, Vec<u8>)> = sqlx::query_as(
        "SELECT uuid_send(character_id),contributors FROM game_character_primal_hazard_receipts WHERE death_key=$1 ORDER BY character_id",
    ).bind(death_key).fetch_all(&mut **tx).await.map_err(DurabilityError::from)?;
    if !receipts.is_empty() {
        if receipts.len() != members.len()
            || receipts
                .iter()
                .zip(&members)
                .any(|((character, previous), member)| {
                    character.as_slice() != member.fence.character_id.as_bytes().as_slice()
                        || previous != &contributors
                })
        {
            return Err(CharacterProgressionError::ConflictingOccurrence);
        }
        return Ok(());
    }
    let current_death: bool = sqlx::query_scalar(
        "SELECT created_xact_id=pg_current_xact_id() FROM game_party_kill_xp_receipts WHERE death_key=$1",
    ).bind(death_key).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    if !current_death {
        return Err(CharacterProgressionError::AuthorityRejected);
    }
    let minimum = members
        .iter()
        .map(|member| member.selected)
        .min()
        .ok_or(CharacterProgressionError::InvalidInput)?;
    for member in members {
        commit_contributor(tx, death_key, &contributors, minimum, member).await?;
    }
    Ok(())
}

fn death_matches_scope(death: &[u8], scope: RuntimeScopeRefV1, generation: u64) -> bool {
    // Reuse the native death owner's disjoint Channel/Instance framing. Never
    // reinterpret an Instance UUID as a Channel identifier.
    super::super::party_kill_xp::decoded_death_scope(death) == Some((scope, generation))
}

async fn load_contributor(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    death_key: &[u8],
    mut fence: CurrentCharacterGameplayFence,
) -> Result<Contributor> {
    let proven: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_party_kill_xp_receipts d JOIN game_party_kill_xp_members m USING(death_key) WHERE d.death_key=$1 AND d.creature_key=$2 AND m.character_id=encode($3,'hex')::uuid)",
    ).bind(death_key).bind(PRIMAL_MENACE).bind(fence.character_id.as_bytes().as_slice())
        .fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    if !proven {
        return Err(CharacterProgressionError::AuthorityRejected);
    }
    // The XP owner advanced this already-locked root. Rebind only its revision;
    // never relax the session, connection, lease, scope or node incarnation.
    let revision: String = sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    fence.expected_character_revision = CharacterRevision::new(
        revision
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    assert_gameplay_fence(tx, &fence, node).await??;
    sqlx::query("INSERT INTO game_character_hazard(character_id) VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING")
        .bind(fence.character_id.as_bytes().as_slice()).execute(&mut **tx).await.map_err(DurabilityError::from)?;
    let (unlocked, selected): (i16, i16) = sqlx::query_as(
        "SELECT unlocked,selected FROM game_character_hazard WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    Ok(Contributor {
        fence,
        unlocked: u16::try_from(unlocked).map_err(|_| DurabilityError::InvalidStoredState)?,
        selected: u16::try_from(selected).map_err(|_| DurabilityError::InvalidStoredState)?,
    })
}

async fn commit_contributor(
    tx: &mut Transaction<'_, Postgres>,
    death_key: &[u8],
    contributors: &[u8],
    minimum: u16,
    member: Contributor,
) -> Result<()> {
    let fence = member.fence;
    let after = unlocked_after_primal_menace(member.unlocked, member.selected, minimum)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let changed = after != member.unlocked;
    let before = fence.expected_character_revision.get();
    let next = before
        .checked_add(u64::from(changed))
        .ok_or(DurabilityError::InvalidStoredState)?;
    let state = sqlx::query("SELECT character_revision::text,level,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    let root = sqlx::query("SELECT profile_revision,ruleset_revision,content_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    let root = super::super::character_progression::FencedCharacterRoot {
        revision: before,
        profile_revision: root
            .try_get("profile_revision")
            .map_err(DurabilityError::from)?,
        ruleset_revision: root
            .try_get("ruleset_revision")
            .map_err(DurabilityError::from)?,
        content_revision: root
            .try_get("content_revision")
            .map_err(DurabilityError::from)?,
    };
    if numeric_u64(&state, "character_revision")? != before || !state_matches_root(&state, &root) {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let result = vec![after as u8, member.selected as u8];
    sqlx::query("INSERT INTO game_character_primal_hazard_receipts(character_id,death_key,contributors,group_minimum,unlocked_before,unlocked_after,selected,result,command_session_id,connection_generation,character_lease_generation,scope_ownership_generation,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) SELECT s.character_id,$2,$3,$4,$5,$6,$7,$8,encode($9,'hex')::uuid,$10::text::numeric,$11::text::numeric,$12::text::numeric,s.character_revision,$13::text::numeric,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision FROM game_character_progression_state s WHERE s.character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).bind(death_key).bind(contributors)
        .bind(minimum as i16).bind(member.unlocked as i16).bind(after as i16).bind(member.selected as i16).bind(&result)
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.connection_generation.get().to_string())
        .bind(fence.character_lease_generation.to_string()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(changed.then(|| next.to_string())).execute(&mut **tx).await.map_err(DurabilityError::from)?;
    if changed {
        for query in [
            "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
            "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
        ] {
            if sqlx::query(query)
                .bind(fence.character_id.as_bytes().as_slice())
                .bind(next.to_string())
                .bind(before.to_string())
                .execute(&mut **tx)
                .await
                .map_err(DurabilityError::from)?
                .rows_affected()
                != 1
            {
                return Err(DurabilityError::InvalidStoredState.into());
            }
        }
        if sqlx::query("UPDATE game_character_hazard SET unlocked=$2 WHERE character_id=encode($1,'hex')::uuid AND unlocked=$3 AND selected=$4")
            .bind(fence.character_id.as_bytes().as_slice()).bind(after as i16).bind(member.unlocked as i16).bind(member.selected as i16)
            .execute(&mut **tx).await.map_err(DurabilityError::from)?.rows_affected() != 1
        { return Err(DurabilityError::InvalidStoredState.into()); }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primal_death_scope_accepts_tagged_instance_without_channel_alias()
    -> std::result::Result<(), String> {
        let mut identity = [1; 16];
        identity[6] = 0x70;
        identity[8] = 0x80;
        let world = crate::foundation::WorldId::decode(&identity).map_err(|e| format!("{e:?}"))?;
        let channel =
            crate::foundation::ChannelId::decode(&identity).map_err(|e| format!("{e:?}"))?;
        let channel_scope = RuntimeScopeRefV1::channel(world, channel);
        let instance_scope =
            RuntimeScopeRefV1::instance(world, identity).map_err(|e| format!("{e:?}"))?;
        let mut legacy = identity.to_vec();
        legacy.extend_from_slice(&identity);
        legacy.extend_from_slice(&3_u64.to_be_bytes());
        legacy.extend_from_slice(&7_u32.to_be_bytes());
        legacy.extend_from_slice(&9_u64.to_be_bytes());
        let mut tagged = vec![2];
        tagged.extend_from_slice(&legacy);
        assert!(death_matches_scope(&legacy, channel_scope, 3));
        assert!(death_matches_scope(&tagged, instance_scope, 3));
        assert!(!death_matches_scope(&legacy, instance_scope, 3));
        assert!(!death_matches_scope(&tagged, channel_scope, 3));
        assert!(!death_matches_scope(&tagged, instance_scope, 4));
        assert!(!death_matches_scope(&tagged[..20], instance_scope, 3));
        Ok(())
    }
}

#[cfg(test)]
mod scope_tests {
    use super::*;
    type TestResult<T = ()> = std::result::Result<T, String>;
    use crate::domain::CharacterId;
    use crate::foundation::{
        ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
        ScopeOwnershipGeneration, WorldId,
    };
    fn id(n: u8) -> [u8; 16] {
        [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
    }
    fn fence(instance: bool) -> TestResult<CurrentCharacterGameplayFence> {
        Ok(CurrentCharacterGameplayFence {
            character_id: CharacterId::from_bytes(id(1)).map_err(|error| format!("{error:?}"))?,
            game_session_id: GameSessionId::decode(&id(2)).map_err(|error| format!("{error:?}"))?,
            connection_generation: ConnectionGeneration::new(1)
                .map_err(|error| format!("{error:?}"))?,
            character_lease_generation: 1,
            runtime_scope: if instance {
                RuntimeScopeRefV1::Instance {
                    world_id: WorldId::decode(&id(3)).map_err(|error| format!("{error:?}"))?,
                    instance_id: id(4),
                }
            } else {
                RuntimeScopeRefV1::Channel {
                    world_id: WorldId::decode(&id(3)).map_err(|error| format!("{error:?}"))?,
                    channel_id: ChannelId::decode(&id(4)).map_err(|error| format!("{error:?}"))?,
                }
            },
            scope_ownership_generation: ScopeOwnershipGeneration::new(7)
                .map_err(|error| format!("{error:?}"))?,
            expected_character_revision: CharacterRevision::new(1)
                .map_err(|error| format!("{error:?}"))?,
        })
    }
    #[test]
    fn primal_death_scope_preserves_channel_and_accepts_only_exact_tagged_instance() -> TestResult {
        let channel = fence(false)?;
        let instance = fence(true)?;
        let mut old = Vec::new();
        old.extend(id(3));
        old.extend(id(4));
        old.extend(7u64.to_be_bytes());
        old.extend(1u32.to_be_bytes());
        old.extend(1u64.to_be_bytes());
        let mut tagged = vec![2];
        tagged.extend(&old);
        let old: [u8; 52] = old
            .try_into()
            .map_err(|error| format!("legacy Channel death: {error:?}"))?;
        let tagged: [u8; 53] = tagged
            .try_into()
            .map_err(|error| format!("tagged Instance death: {error:?}"))?;
        assert!(death_matches_scope(
            &old,
            channel.runtime_scope,
            channel.scope_ownership_generation.get()
        ));
        assert!(!death_matches_scope(
            &old,
            instance.runtime_scope,
            instance.scope_ownership_generation.get()
        ));
        assert!(death_matches_scope(
            &tagged,
            instance.runtime_scope,
            instance.scope_ownership_generation.get()
        ));
        assert!(!death_matches_scope(
            &tagged,
            channel.runtime_scope,
            channel.scope_ownership_generation.get()
        ));
        for byte in [0, 1, 17, 33] {
            let mut changed = tagged;
            changed[byte] ^= 1;
            assert!(
                !death_matches_scope(
                    &changed,
                    instance.runtime_scope,
                    instance.scope_ownership_generation.get()
                ),
                "wrong exact native death field at{byte}"
            );
        }
        for span in [41..45, 45..53] {
            let mut zero = tagged;
            zero[span].fill(0);
            assert!(!death_matches_scope(
                &zero,
                instance.runtime_scope,
                instance.scope_ownership_generation.get()
            ));
        }
        for n in [0, 1, 51, 52] {
            assert!(!death_matches_scope(
                &tagged[..n],
                instance.runtime_scope,
                instance.scope_ownership_generation.get()
            ));
        }
        Ok(())
    }
}
