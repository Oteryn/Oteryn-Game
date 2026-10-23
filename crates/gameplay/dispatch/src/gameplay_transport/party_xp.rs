//! PARTY-XP-1 planning under the consuming runtime/player and SQL locks.
use super::{actor_spell::ChannelSpellStates, kill_reward::PendingKillSettlement};
use crate::durability::{
    DurabilityRoot,
    spell_item_transaction::{SpellItemError, SpellItemScopeAuthority},
    world_party,
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, PartySnapshot};
use oteryn_gameplay_social_rules::shared_experience_split::{
    SharedExperienceMember, SharedExperiencePool, damage_experience_pool, shared_experience_share,
};
use sqlx::{Postgres, Transaction};

type Result<T> = std::result::Result<T, SpellItemError>;
#[derive(Debug, Clone, Copy)]
pub(super) struct XpParticipant {
    pub character: crate::domain::CharacterId,
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    pub share: i64,
    pub premium: bool,
    pub shared: bool,
    pub party_id: Option<[u8; 16]>,
    pub party_revision: Option<u64>,
}
#[derive(Debug)]
pub(super) struct PartyXpPlan {
    pub source: PartySnapshot,
    pub shared: bool,
    pub members: Vec<XpParticipant>,
}
fn vocation(v: crate::spell::Vocation) -> u8 {
    use crate::spell::Vocation::*;
    match v {
        // TUTORIAL-1's vocationless Character adds no vocation to the shared bonus.
        crate::spell::Vocation::None => 0,
        Knight | EliteKnight => 1,
        Paladin | RoyalPaladin => 2,
        Sorcerer | MasterSorcerer => 3,
        Druid | ElderDruid => 4,
        Monk | ExaltedMonk => 5,
    }
}

/// One death can contain damage from several parties and solo Characters. Their damage
/// pools stay separate; eligibility failure in one party cannot confiscate a foreign share.
pub(super) async fn plan_shared_xp_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    scope: &SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    entry: &PendingKillSettlement,
) -> Result<PartyXpPlan> {
    if entry.activity_at_death.is_none() {
        return Err(SpellItemError::Rejected("XP captured party bound"));
    }
    let first = plan_group(
        tx,
        root,
        scope,
        runtime,
        states,
        entry,
        (entry.facts.principal.actor, entry.facts.principal.session),
    )
    .await?;
    let mut covered = std::collections::BTreeSet::new();
    if first.source.party_id.is_some() {
        covered.extend(first.source.members.iter().map(|m| m.character));
    } else {
        covered.insert(first.source.character);
    }
    let mut result = first;
    let mut contributors = entry
        .damage_shares
        .iter()
        .map(|d| *d.character.as_bytes())
        .collect::<Vec<_>>();
    contributors.sort();
    let census = runtime
        .positioned_actor_census()
        .map_err(|_| SpellItemError::Rejected("XP census"))?;
    for character in contributors {
        if covered.contains(&character) {
            continue;
        }
        let current = census.iter().find_map(|(actor, _, session)| {
            let session = (*session)?;
            runtime
                .bound_attacker_lease(*actor, session)
                .ok()
                .filter(|l| {
                    *l.character_id().as_bytes() == character
                        && entry.damage_shares.iter().any(|d| {
                            *d.character.as_bytes() == character
                                && d.session == Some(session)
                                && d.lease_generation == Some(l.generation())
                        })
                })
                .map(|_| (*actor, session))
        });
        // D52: a contributor whose original physical session has gone receives no new grant.
        let Some(current) = current else { continue };
        let next = plan_group(tx, root, scope, runtime, states, entry, current).await?;
        if next.source.party_id.is_some() {
            covered.extend(next.source.members.iter().map(|m| m.character));
        } else {
            covered.insert(character);
        }
        result.members.extend(next.members);
        if result.members.len() > 50 {
            return Err(SpellItemError::Rejected("XP principal bound"));
        }
    }
    result.members.sort_by_key(|m| m.character);
    if result
        .members
        .windows(2)
        .any(|m| m[0].character == m[1].character)
    {
        return Err(SpellItemError::Rejected("XP duplicate principal"));
    }
    Ok(result)
}

async fn plan_group(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    scope: &SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    entry: &PendingKillSettlement,
    (actor, session): (ExactActorRef, GameSessionId),
) -> Result<PartyXpPlan> {
    let observed = world_party::read_world_party_xp_for_scope_actor_in_transaction(
        tx, root, scope, runtime, actor, session,
    )
    .await?;
    let source = observed.source;
    let now = entry.activity_observed_at.ok_or(SpellItemError::Rejected(
        "XP precise death time unavailable",
    ))?;
    let now_us = now.get();
    if now_us / 1000 != entry.facts.death_at_ms {
        return Err(SpellItemError::Rejected("XP death clock mismatch"));
    }
    let cells = entry
        .activity_at_death
        .as_deref()
        .ok_or(SpellItemError::Rejected("XP activity unavailable"))?;
    let belongs = |character: &[u8; 16]| {
        if source.party_id.is_some() {
            source.members.iter().any(|m| m.character == *character)
        } else {
            *character == source.character
        }
    };
    let mut local = Vec::new();
    let mut rules = Vec::new();
    for (actor, position, session) in runtime
        .positioned_actor_census()
        .map_err(|_| SpellItemError::Rejected("XP census"))?
    {
        let Some(session) = session else { continue };
        let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
            continue;
        };
        let character = crate::domain::CharacterId::from_bytes(*lease.character_id().as_bytes())
            .map_err(|_| SpellItemError::Rejected("XP character"))?;
        if !belongs(character.as_bytes()) {
            continue;
        }
        let current=crate::durability::admission_journal::party_target_binding::current_colocated_party_character_for_scope_in_transaction(tx,root,scope,runtime,actor,session).await?;
        if current != Some(*character.as_bytes()) {
            return Err(SpellItemError::Rejected("XP current actor"));
        }
        let state = states
            .get(runtime, actor, session)
            .ok_or(SpellItemError::Rejected("XP player state"))?;
        let level:Option<i64>=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let level = level
            .map_or(Ok(state.character_facts().level), u32::try_from)
            .map_err(|_| SpellItemError::Rejected("XP level"))?;
        if source.party_id.is_some() {
            let p = position.position();
            rules.push(SharedExperienceMember {character:*character.as_bytes(),channel:scope.channel_bytes(),level,x:p.x,y:p.y,
                vocation:vocation(state.source_party_vocation()),activity:cells.iter().find(|a|a.character==*character.as_bytes())
                    .and_then(|a|oteryn_gameplay_social_rules::shared_experience::committed_shared_experience_activity(*a,character.as_bytes(),lease.generation(),now))});
        }
        local.push(XpParticipant {
            character,
            actor,
            session,
            share: 0,
            premium: state.hunting_premium_current(now_us),
            shared: false,
            party_id: source.party_id,
            party_revision: source.revision,
        });
    }
    if local.len() > 50 {
        return Err(SpellItemError::Rejected("XP principal bound"));
    }
    let total: u128 = entry
        .damage_shares
        .iter()
        .map(|d| u128::from(d.total))
        .sum();
    let base = entry.row.xp_amount.get();
    let contributions = entry
        .damage_shares
        .iter()
        .map(|d| (d.total, belongs(d.character.as_bytes())))
        .collect::<Vec<_>>();
    let party_xp = damage_experience_pool(base, &contributions)
        .map_err(|_| SpellItemError::Rejected("XP damage overflow"))?;
    let leader_pz = if let Some(leader) = source.leader {
        crate::durability::party_pz_fact::leader_pz_blocked_in_transaction(tx, scope, &leader)
            .await?
    } else {
        false
    };
    let missing_local = source.members.iter().any(|m| {
        m.channel == Some(scope.channel_bytes())
            && !rules.iter().any(|p| p.character == m.character)
    });
    let share = if rules.is_empty() {
        None
    } else {
        shared_experience_share(
            observed.enabled && !missing_local,
            source.leader.unwrap_or([0; 16]),
            scope.channel_bytes(),
            leader_pz,
            &rules,
            SharedExperiencePool {
                creature_xp: base,
                party_xp,
                summon_xp: 0,
            },
        )
        .map_err(|_| SpellItemError::Rejected("XP invalid planner inputs"))?
    };
    let shared = share.is_some();
    let mut members = Vec::new();
    for mut member in local {
        member.shared = shared;
        member.share = if let Some(share) = share {
            share
        } else if total > 0 {
            let Some(damage) = entry
                .damage_shares
                .iter()
                .find(|d| d.character.as_bytes() == member.character.as_bytes())
            else {
                continue;
            };
            let lease = runtime
                .bound_attacker_lease(member.actor, member.session)
                .map_err(|_| SpellItemError::Rejected("XP damage lease"))?;
            if damage.session != Some(member.session)
                || damage.lease_generation != Some(lease.generation())
            {
                continue;
            }
            i64::try_from(
                (base as u128 * u128::from(damage.total))
                    .checked_div(total)
                    .ok_or(SpellItemError::Rejected("XP zero damage"))?,
            )
            .map_err(|_| SpellItemError::Rejected("XP damage overflow"))?
        } else {
            if member.character.as_bytes() != entry.facts.principal.character.as_bytes() {
                continue;
            }
            base
        };
        members.push(member);
    }
    members.sort_by_key(|m| m.character);
    Ok(PartyXpPlan {
        source,
        shared,
        members,
    })
}

#[cfg(test)]
#[path = "party_xp_postgres_tests.rs"]
mod postgres_tests;

/// Freeze bounded committed event data; consuming authority is re-proved in SQL.
pub(super) fn capture_activity_at_death(
    runtime: &ChannelRuntimeV1,
    principal: crate::foundation::CharacterId,
    damage_shares: &[crate::foundation::RewardDamageShare],
    activity: &[oteryn_gameplay_social_rules::shared_experience::SharedExperienceActivity],
) -> std::result::Result<
    Option<Vec<oteryn_gameplay_social_rules::shared_experience::SharedExperienceActivity>>,
    crate::foundation::CarrierError,
> {
    // Cache data bounds event capture only. Every contributing party is independently
    // resolved again under the World owner at consumption, before any award.
    let mut captured = std::collections::BTreeSet::new();
    captured.insert(*principal.as_bytes());
    let census = runtime.positioned_actor_census()?;
    let local = census
        .iter()
        .filter_map(|(a, _, s)| {
            (*s).and_then(|s| runtime.bound_attacker_lease(*a, s).ok())
                .map(|l| *l.character_id().as_bytes())
        })
        .collect::<std::collections::BTreeSet<_>>();
    for (actor, _, session) in census {
        let Some(session) = session else { continue };
        let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
            continue;
        };
        if lease.character_id() != principal
            && !damage_shares
                .iter()
                .any(|d| d.character == lease.character_id())
        {
            continue;
        }
        captured.insert(*lease.character_id().as_bytes());
        if let Ok(Some(party)) = runtime.cached_world_party(actor, session)
            && party.is_well_formed()
        {
            captured.extend(
                party
                    .members
                    .iter()
                    .map(|m| m.character)
                    .filter(|id| local.contains(id)),
            );
        }
        if captured.len() > 50 {
            break;
        }
    }
    Ok(if captured.len() > 50 {
        None
    } else {
        let mut cells = std::collections::BTreeMap::new();
        for cell in activity
            .iter()
            .copied()
            .filter(|a| captured.contains(&a.character))
        {
            let old = cells.entry(cell.character).or_insert(cell);
            if (cell.lease_generation, cell.at) > (old.lease_generation, old.at) {
                *old = cell;
            }
        }
        Some(cells.into_values().collect())
    })
}
