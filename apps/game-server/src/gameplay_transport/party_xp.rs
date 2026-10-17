//! PARTY-PVP-0 §5.1: bounded activity, eligibility and the final equal split.
//! Detached source component: no kill admission, XP receipt or current-authority producer.
//! Callers hold runtime/player/SQL owner locks, read the toggle in that transaction and supply
//! the actual leader PZ deadline. The native attack owner supplies its identity/lease-bound
//! activity predicate at the original kill instant. Supplied observations never attest current
//! authority. Missing inputs refuse a plan.
#![allow(
    dead_code,
    reason = "CP slot must compose the native kill/reward owners"
)]

use super::actor_spell::ChannelSpellStates;
use super::attack::ChannelAttackStates;
use super::party_spell_owner::{SourcePartyWorld, SourcePartyXpMember};
use crate::durability::world_party::WorldPartyXp;
use crate::foundation::{ChannelId, ChannelRuntimeV1, GameSessionId, PartySnapshot, WorldId};
use oteryn_simulation_determinism::{ExactI64, SemanticTimeMicros};

const MAX_MEMBERS: usize = 50;
const ACTIVITY_MICROS: u64 = 120_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PartyXpError {
    SourceChanged,
    InvalidRoster,
    ActivityScopeChanged,
    ClockRegressed,
    InvalidPool,
    PzObservationUnavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SharingOff {
    Solo,
    Disabled,
    LeaderAbsent,
    Range,
    Level,
    Inactive,
    LeaderPzBlock,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PartyXpShare {
    pub(crate) character: [u8; 16],
    pub(crate) session: GameSessionId,
    pub(crate) amount: ExactI64,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PartyXpDecision {
    /// The kill owner uses its existing damage-share path; this is not an XP award.
    Off(SharingOff),
    /// Before per-character Prey then stamina, with no redistribution of reduced shares.
    Shares(Vec<PartyXpShare>),
}

/// Data from the current leader's PZ owner, not the broader logout/battle lock. The outer
/// Option at the consumer distinguishes unavailable data from a known absent deadline.
/// Its producer must resolve the actual current CombatLock under the same owner turn.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LeaderPzObservation {
    pub(crate) character: [u8; 16],
    pub(crate) session: GameSessionId,
    pub(crate) observed_at: SemanticTimeMicros,
    pub(crate) pz_until: Option<SemanticTimeMicros>,
}

/// Event data only. Constructing this enum proves neither a committed event nor current
/// membership/aggressiveness; only the real combat/heal owner can qualify those facts.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PartyActivityEvent {
    /// The heal owner must have committed an actual heal of this member.
    HealedMember {
        healer: [u8; 16],
        session: GameSessionId,
        healed: [u8; 16],
    },
    /// The attack owner must have qualified an actual attack on an aggressive monster.
    AttackedAggressiveMonster {
        attacker: [u8; 16],
        session: GameSessionId,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActivityStamp {
    world: WorldId,
    channel: ChannelId,
    character: [u8; 16],
    session: GameSessionId,
    at: SemanticTimeMicros,
}
/// One bounded cell per Character/session on its World/Channel. Party changes never clear
/// a qualifying recent event; a solo aggressive attack can qualify after joining a party.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PartyXpActivity {
    stamp: Option<ActivityStamp>,
}
impl PartyXpActivity {
    /// Copies retained event DATA without renewing, clearing or restoring the cell.
    /// This observation grants no current authority or engagement qualification.
    pub(crate) fn recorded_activity(
        &self,
    ) -> Option<(
        WorldId,
        ChannelId,
        [u8; 16],
        GameSessionId,
        SemanticTimeMicros,
    )> {
        self.stamp.map(|stamp| {
            (
                stamp.world,
                stamp.channel,
                stamp.character,
                stamp.session,
                stamp.at,
            )
        })
    }

    /// The actual event owner retains the cell, with the event's original semantic timestamp.
    /// On recovery an empty cell grants nothing; old sessions/scopes cannot rebind this cell.
    pub(crate) fn record(
        &mut self,
        source: &SourcePartyWorld,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        event: PartyActivityEvent,
        at: SemanticTimeMicros,
    ) -> Result<(), PartyXpError> {
        source
            .validate_xp_physical(runtime, states)
            .map_err(|_| PartyXpError::SourceChanged)?;
        if source.source().world != *runtime.binding().world_id().as_bytes() {
            return Err(PartyXpError::ActivityScopeChanged);
        }
        self.record_members(
            source.source(),
            runtime.binding().channel_id(),
            &source.xp_members(),
            event,
            at,
        )
    }
    fn record_members(
        &mut self,
        source: &PartySnapshot,
        channel: ChannelId,
        members: &[SourcePartyXpMember],
        event: PartyActivityEvent,
        at: SemanticTimeMicros,
    ) -> Result<(), PartyXpError> {
        if source.party_id.is_some() {
            validate_roster(source, members)?;
        } else if !source.members.is_empty()
            || members.len() != 1
            || members[0].character != source.character
        {
            return Err(PartyXpError::InvalidRoster);
        }
        let (character, session) = match event {
            PartyActivityEvent::HealedMember {
                healer,
                session,
                healed,
            } => {
                if source.party_id.is_none() || !members.iter().any(|m| m.character == healed) {
                    return Err(PartyXpError::InvalidRoster);
                }
                (healer, session)
            }
            PartyActivityEvent::AttackedAggressiveMonster { attacker, session } => {
                (attacker, session)
            }
        };
        if !members
            .iter()
            .any(|m| m.character == character && m.session == session)
        {
            return Err(PartyXpError::InvalidRoster);
        }
        let world = WorldId::decode(&source.world).map_err(|_| PartyXpError::InvalidRoster)?;
        if let Some(previous) = self.stamp {
            if previous.world != world
                || previous.channel != channel
                || previous.character != character
                || previous.session != session
            {
                return Err(PartyXpError::ActivityScopeChanged);
            }
            if previous.at > at {
                return Err(PartyXpError::ClockRegressed);
            }
        }
        self.stamp = Some(ActivityStamp {
            world,
            channel,
            character,
            session,
            at,
        });
        Ok(())
    }
}

fn validate_roster(
    source: &PartySnapshot,
    members: &[SourcePartyXpMember],
) -> Result<(), PartyXpError> {
    if source.members.len() > MAX_MEMBERS || members.len() > MAX_MEMBERS {
        return Err(PartyXpError::InvalidRoster);
    }
    for (i, member) in members.iter().enumerate() {
        if member.level == 0
            || !source
                .members
                .iter()
                .any(|m| m.character == member.character)
            || members[..i]
                .iter()
                .any(|m| m.character == member.character || m.session == member.session)
        {
            return Err(PartyXpError::InvalidRoster);
        }
    }
    Ok(())
}

/// `pool_after_bonus_and_summon` is an already admitted integer XP pool after the PARTY bonus
/// and the summon share, before equal division. This module neither guesses their intermediate
/// rounding nor counts promoted vocations twice. The future owner must supply that pool from
/// the accepted native bonus/summon calculation; arbitrary caller numbers are not reward truth.
#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_shared_xp(
    source: &SourcePartyWorld,
    setting: &WorldPartyXp,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    activity: &ChannelAttackStates,
    pz: Option<LeaderPzObservation>,
    now: SemanticTimeMicros,
    pool_after_bonus_and_summon: ExactI64,
) -> Result<PartyXpDecision, PartyXpError> {
    source
        .validate_xp_physical(runtime, states)
        .map_err(|_| PartyXpError::SourceChanged)?;
    if !source.matches_current_world(&setting.source)
        || setting.source.world != *runtime.binding().world_id().as_bytes()
    {
        return Err(PartyXpError::SourceChanged);
    }
    evaluate_activity(
        &setting.source,
        runtime.binding().channel_id(),
        &source.xp_members(),
        setting.enabled,
        ActivityInput::Attack(activity, runtime, now),
        pz,
        pool_after_bonus_and_summon,
    )
}

#[allow(clippy::too_many_arguments)]
fn evaluate(
    source: &PartySnapshot,
    channel: ChannelId,
    members: &[SourcePartyXpMember],
    enabled: bool,
    activity: &[PartyXpActivity],
    pz: Option<LeaderPzObservation>,
    now: SemanticTimeMicros,
    pool: ExactI64,
) -> Result<PartyXpDecision, PartyXpError> {
    evaluate_activity(
        source,
        channel,
        members,
        enabled,
        ActivityInput::Recorded(activity, now),
        pz,
        pool,
    )
}

/// Private planner inputs: native E activity is queried at the ORIGINAL kill instant.
/// A true native predicate never manufactures or renews an engagement timestamp.
#[derive(Clone, Copy)]
enum ActivityInput<'a> {
    Recorded(&'a [PartyXpActivity], SemanticTimeMicros),
    Attack(
        &'a ChannelAttackStates,
        &'a ChannelRuntimeV1,
        SemanticTimeMicros,
    ),
}

fn evaluate_activity(
    source: &PartySnapshot,
    channel: ChannelId,
    members: &[SourcePartyXpMember],
    enabled: bool,
    activity: ActivityInput<'_>,
    pz: Option<LeaderPzObservation>,
    pool: ExactI64,
) -> Result<PartyXpDecision, PartyXpError> {
    let now = match activity {
        ActivityInput::Recorded(_, at) | ActivityInput::Attack(_, _, at) => at,
    };
    let off = |reason| Ok(PartyXpDecision::Off(reason));
    if source.party_id.is_none() {
        return off(SharingOff::Solo);
    }
    if !enabled {
        return off(SharingOff::Disabled);
    }
    validate_roster(source, members)?;
    let Some(leader) = members.iter().find(|m| Some(m.character) == source.leader) else {
        return off(SharingOff::LeaderAbsent);
    };
    if let ActivityInput::Recorded(activity, _) = activity {
        if activity.len() > MAX_MEMBERS {
            return Err(PartyXpError::InvalidRoster);
        }
        for (i, cell) in activity.iter().enumerate() {
            let Some(stamp) = cell.stamp else {
                continue;
            };
            if !members.iter().any(|m| m.character == stamp.character)
                || activity[..i]
                    .iter()
                    .any(|c| c.stamp.is_some_and(|s| s.character == stamp.character))
            {
                return Err(PartyXpError::InvalidRoster);
            }
            if *stamp.world.as_bytes() != source.world
                || stamp.channel != channel
                || !members
                    .iter()
                    .any(|m| m.character == stamp.character && m.session == stamp.session)
            {
                return Err(PartyXpError::ActivityScopeChanged);
            }
        }
    }
    let pz = pz.ok_or(PartyXpError::PzObservationUnavailable)?;
    if pz.character != leader.character || pz.session != leader.session || pz.observed_at != now {
        return Err(PartyXpError::PzObservationUnavailable);
    }
    if pz.pz_until.is_some_and(|until| until > now) {
        return off(SharingOff::LeaderPzBlock);
    }
    let mut low = u32::MAX;
    let mut high = 0;
    for member in members {
        // Any floor; floor distance never participates in this accepted Chebyshev rule.
        if member.position.x.abs_diff(leader.position.x) > 30
            || member.position.y.abs_diff(leader.position.y) > 30
        {
            return off(SharingOff::Range);
        }
        low = low.min(member.level);
        high = high.max(member.level);
        let active = match activity {
            ActivityInput::Recorded(cells, _) => {
                let Some(stamp) = cells.iter().filter_map(|cell| cell.stamp).find(|stamp| {
                    stamp.character == member.character && stamp.session == member.session
                }) else {
                    return off(SharingOff::Inactive);
                };
                let age = now
                    .get()
                    .checked_sub(stamp.at.get())
                    .ok_or(PartyXpError::ClockRegressed)?;
                age <= ACTIVITY_MICROS
            }
            ActivityInput::Attack(owner, runtime, _) => {
                // The preceding native source read resolved this Character/session;
                // lookup retains dead colocated XP members too, unlike spell targets.
                let (actor, position) = runtime
                    .positioned_player_for_session(member.session)
                    .map_err(|_| PartyXpError::SourceChanged)?
                    .ok_or(PartyXpError::SourceChanged)?;
                if runtime.binding().channel_id() != channel
                    || *runtime.binding().world_id().as_bytes() != source.world
                    || position.position() != member.position
                {
                    return Err(PartyXpError::SourceChanged);
                }
                owner.shared_experience_active(runtime, actor, member.session, now)
            }
        };
        if !active {
            return off(SharingOff::Inactive);
        }
    }
    if u64::from(low) * 3 < u64::from(high) * 2 {
        return off(SharingOff::Level);
    }
    let pool = pool.get();
    if pool < 0 {
        return Err(PartyXpError::InvalidPool);
    }
    let count = i64::try_from(members.len()).map_err(|_| PartyXpError::InvalidRoster)?;
    // Leader presence ensures a nonzero divisor. Quotient+remainder avoids pool+n-1 overflow.
    let amount = ExactI64::new(pool / count + i64::from(pool % count != 0));
    Ok(PartyXpDecision::Shares(
        members
            .iter()
            .map(|m| PartyXpShare {
                character: m.character,
                session: m.session,
                amount,
            })
            .collect(),
    ))
}

/// D1102 consumer DATA; E binds these values to its actual retained kill.
/// The death key proves no current Character authority or original-time provenance.
/// `pool` is after the native party bonus and summon share, before Prey/stamina.
pub(crate) struct PartyXpKillInputs<'a> {
    pub(crate) death: crate::foundation::CreatureDeathOccurrenceKey,
    pub(crate) at: SemanticTimeMicros,
    pub(crate) activity: &'a ChannelAttackStates,
    pub(crate) leader_pz: Option<LeaderPzObservation>,
    pub(crate) pool_after_bonus_and_summon: ExactI64,
}

/// A bound planner observation, never a reservation, receipt or commit authority.
#[derive(Debug)]
pub(crate) struct PartyXpKillPlan {
    pub(crate) death: crate::foundation::CreatureDeathOccurrenceKey,
    pub(crate) at: SemanticTimeMicros,
    pub(crate) party_id: Option<[u8; 16]>,
    pub(crate) party_revision: Option<u64>,
    pub(crate) decision: PartyXpDecision,
}

/// Keep unavailable native source facts separate from a valid sharing-Off decision.
#[derive(Debug)]
pub(crate) enum PartyXpConsumerError {
    Source(crate::durability::spell_item_transaction::SpellItemError),
    Plan(PartyXpError),
}

/// The owned read/planner consumer; E's atomic settlement caller is separate.
/// Reads both actual native sources through the caller's SAME SQL transaction.
/// E retains responsibility for original kill-time provenance, at-kill capture,
/// lawful reservation/lock ordering and every recipient's independent current fence.
/// Returning this DATA does not let E hold a runtime lock across a sequencer/write,
/// reuse it as current authority, or resample a changed party on occurrence replay.
pub(crate) async fn plan_shared_xp_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &crate::durability::spell_item_transaction::SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    caster: (crate::foundation::ExactActorRef, GameSessionId),
    input: PartyXpKillInputs<'_>,
) -> Result<PartyXpKillPlan, PartyXpConsumerError> {
    use crate::durability::spell_item_transaction::SpellItemError;
    if input.death.world_id() != runtime.binding().world_id()
        || input.death.channel_id() != runtime.binding().channel_id()
        || input.death.scope_ownership_generation() != runtime.binding().scope_generation()
    {
        return Err(PartyXpConsumerError::Source(SpellItemError::Rejected(
            "party XP death scope changed",
        )));
    }
    let setting =
        crate::durability::world_party::read_world_party_xp_for_scope_actor_in_transaction(
            tx, root, authority, runtime, caster.0, caster.1,
        )
        .await
        .map_err(PartyXpConsumerError::Source)?;
    let physical = super::party_spell_owner::read_source_party_world_for_scope_in_transaction(
        tx, root, authority, runtime, states, caster.0, caster.1,
    )
    .await
    .map_err(PartyXpConsumerError::Source)?;
    let decision = plan_shared_xp(
        &physical,
        &setting,
        runtime,
        states,
        input.activity,
        input.leader_pz,
        input.at,
        input.pool_after_bonus_and_summon,
    )
    .map_err(PartyXpConsumerError::Plan)?;
    Ok(PartyXpKillPlan {
        death: input.death,
        at: input.at,
        party_id: setting.source.party_id,
        party_revision: setting.source.revision,
        decision,
    })
}

#[cfg(test)]
#[path = "party_xp_tests.rs"]
mod tests;
