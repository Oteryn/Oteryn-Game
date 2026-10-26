//! Provisional Canary 47dfd51f45280a59a1d3e50ba7edd573d7234446 rules, not verified against Global. Sources: src/creatures/players/player.cpp:5513-5625,
//! src/utils/tools.cpp (normal_random), config.lua.dist, and
//! data-otservbr-global/scripts/systems/hazard_primal.lua at that exact revision.
//! Decision 22 permits this fallback after the Global evidence search. No extrapolation
//! beyond the pinned qualification ceiling and no removed loot bonus are installed.
use super::*;
use crate::durability::{
    DurabilityError,
    spell_item_transaction::{self as item_tx, SpellItemError},
    spell_owner_commit::SpellLanePermit,
    world_party,
};
use crate::foundation::{ChannelRuntimeV1, PartySnapshot};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sqlx::{Postgres, Row, Transaction};

// Canary uses a truncated N(0.5, 0.25) draw rounded onto [1, 10000], rather than a
// uniform percentage roll. These integer CDF thresholds preserve that distribution
// without host floating-point arithmetic in the simulation. Threshold for k is
// round(2^64 * (Phi(((k-.5)/9999-.5)/.25)-Phi(-2))/(Phi(2)-Phi(-2))).
const CRITICAL_THRESHOLD: u64 = 421_295_983_000_151_936;
const DODGE_THRESHOLDS: [u64; 12] = [
    36_484_161_335_091_296,
    75_713_787_214_711_840,
    117_607_134_293_348_704,
    162_293_378_720_273_536,
    209_903_639_156_960_320,
    260_570_635_738_160_832,
    314_428_319_087_183_040,
    371_611_469_656_202_688,
    432_255_267_891_930_176,
    496_494_835_966_872_896,
    564_464_752_065_106_368,
    636_298_538_467_120_512,
];

/// Eligibility is fixed by the actual spawn origin, not where a monster was lured.
/// Canary's zone event includes reward bosses; only pod/Plunder death rolls exclude them.
/// A summoned companion has no native spawn source and is therefore ineligible here.
pub(in crate::gameplay_transport) fn eligible(
    runtime: &ChannelRuntimeV1,
    source: ExactActorRef,
) -> bool {
    runtime
        .creature_spawn_cell(source)
        .is_some_and(|p| hazard::contains(p.x, p.y, p.floor))
        && runtime
            .companion_snapshot(source)
            .is_ok_and(|s| s.state.master.is_none())
}

pub(in crate::gameplay_transport) fn in_area(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
) -> bool {
    runtime.read_actor_position(actor).is_ok_and(|p| {
        let p = p.position();
        hazard::contains(p.x, p.y, p.floor)
    })
}

/// This read is consumed before releasing the transaction's World membership and session
/// locks or the Channel runtime guard. Non-damaging party members participate too.
pub(in crate::gameplay_transport) async fn effective_level(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    scope: &item_tx::SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
) -> Result<u8, SpellItemError> {
    let party = world_party::read_world_party_for_scope_actor_in_transaction(
        tx, root, scope, runtime, actor, session,
    )
    .await?;
    level_from_party(tx, root, scope, runtime, actor, session, &party).await
}

async fn level_from_party(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    scope: &item_tx::SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    party: &PartySnapshot,
) -> Result<u8, SpellItemError> {
    let reject = || SpellItemError::Rejected("hazard current party member unavailable");
    let mut members = if party.party_id.is_some() {
        if party.members.len() > 50 || party.members.is_empty() {
            return Err(reject());
        }
        party
            .members
            .iter()
            .map(|m| m.character)
            .collect::<Vec<_>>()
    } else {
        vec![party.character]
    };
    members.sort();
    members.dedup();
    let census = runtime.positioned_actor_census().map_err(|_| reject())?;
    let mut effective = hazard::MAX_LEVEL;
    for character in members {
        // Presence/channel visibility is presentation data. A hidden local member still
        // contributes; the independent current session/lease read below resolves its scope.
        let local = census.iter().find_map(|(a, _, s)| {
            let s = (*s)?;
            runtime
                .bound_attacker_lease(*a, s)
                .ok()
                .filter(|l| l.character_id().as_bytes() == &character)
                .map(|_| (*a, s))
        });
        let Some((member, member_session)) = local else {
            // PARTY-PVP-0 §5: benefits are Channel-local. World membership survives
            // a Channel switch; privacy/presence hints never decide participation.
            if crate::durability::admission_journal::party_target_binding::current_remote_party_character_for_scope_in_transaction(
                tx, root, scope, character,
            ).await? {
                continue;
            }
            return Err(reject());
        };
        if crate::durability::admission_journal::party_target_binding::current_colocated_party_character_for_scope_in_transaction(
            tx, root, scope, runtime, member, member_session,
        ).await? != Some(character) { return Err(reject()); }
        // Exactly two primary-key rows; new local selections cannot race the held runtime
        // guard. Existing rows additionally remain share-locked until the physical effect.
        let rows = sqlx::query("SELECT track_key, value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key IN ($2,$3) ORDER BY track_key LIMIT 2 FOR SHARE")
            .bind(character.as_slice()).bind(hazard::CURRENT).bind(hazard::UNLOCKED)
            .fetch_all(&mut **tx).await?;
        let mut tracks = std::collections::BTreeMap::new();
        for row in rows {
            tracks.insert(row.try_get("track_key")?, row.try_get("value")?);
        }
        let level = hazard::level(&tracks).ok_or_else(reject)?.current;
        effective = effective.min(if in_area(runtime, member) { level } else { 0 });
    }
    if !runtime
        .player_control_facts(actor, session)
        .is_ok_and(|f| f.control_loss.is_none())
    {
        return Err(reject());
    }
    Ok(effective)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Run a synchronous physical effect while authoritative party/lease/quest locks are
    /// still held. An unavailable party is never silently interpreted as solo membership.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gameplay_transport) async fn with_hazard_level<T, F>(
        &self,
        permit: &SpellLanePermit,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        effect: F,
    ) -> Option<T>
    where
        T: Send,
        F: FnOnce(&mut ChannelRuntimeV1, &mut ChannelSpellStates, u8) -> T + Send,
    {
        if !in_area(runtime, actor) {
            return Some(scoped_effect(runtime, states, actor, session, 0, effect));
        }
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = (self, runtime, states, Some(effect), permit);
        pass.run_with_context(&mut context, move |holder, deadline, ctx| {
            Box::pin(async move {
                let (owner, runtime, states, effect, permit) = ctx;
                let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                let b = runtime.binding();
                let scope = item_tx::assert_spell_item_scope_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    crate::foundation::RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
                    b.scope_generation().get(),
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                let level = effective_level(&mut tx, owner.root, &scope, runtime, actor, session)
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                // Moving the closure out of the context is unnecessary: Option::take gives this
                // pass exactly one physical consumer, even when the bounded SQL pass fails.
                let result = scoped_effect(
                    runtime,
                    states,
                    actor,
                    session,
                    level,
                    effect.take().ok_or(DurabilityError::Unavailable)?,
                );
                tx.rollback().await?;
                Ok(result)
            })
        })
        .await
        .ok()
    }
}

fn scoped_effect<T>(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    level: u8,
    effect: impl FnOnce(&mut ChannelRuntimeV1, &mut ChannelSpellStates, u8) -> T,
) -> T {
    let previous = states.hazard_level.replace((actor, session, level));
    let result = effect(runtime, states, level);
    states.hazard_level = previous;
    result
}

impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn hazard_condition_label(
        runtime: &ChannelRuntimeV1,
        source: ExactActorRef,
    ) -> String {
        // Keep the admitted origin with the condition after its monster retires.
        let kind = if eligible(runtime, source) {
            "creature:hazard:"
        } else {
            "creature:"
        };
        format!(
            "{kind}{}",
            source
                .placement_identity()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::gameplay_transport) fn hazard_condition_damage(
        &mut self,
        runtime: &ChannelRuntimeV1,
        source: &str,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        now: u64,
        critical_used: bool,
    ) -> Option<(u32, bool)> {
        let Some(identity) = source.strip_prefix("creature:hazard:") else {
            return Some((magnitude, false));
        };
        if !in_area(runtime, target) {
            return Some((magnitude, false));
        }
        if identity.len() != 32 || !identity.is_ascii() {
            return None;
        }
        let mut bytes = [0; 16];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&identity[i * 2..i * 2 + 2], 16).ok()?;
        }
        let (bound, bound_session, level) = self.hazard_level?;
        if bound != target || bound_session != session {
            return None;
        }
        let ready = !critical_used && self.hazard_criticals.ready(runtime, target, session, now);
        incoming(
            magnitude,
            level,
            ready,
            draw_identity(runtime, bytes, target, now, "hazard.critical")?,
        )
    }

    /// The value exists only during the synchronous consumer of the locked World read.
    pub(in crate::gameplay_transport) fn hazard_damage(
        &mut self,
        runtime: &ChannelRuntimeV1,
        source: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        magnitude: u32,
        now: u64,
    ) -> Option<(u32, bool)> {
        if !eligible(runtime, source) || !in_area(runtime, target) {
            return Some((magnitude, false));
        }
        let (bound, bound_session, level) = self.hazard_level?;
        if bound != target || bound_session != session {
            return None;
        }
        let ready = self.hazard_criticals.ready(runtime, target, session, now);
        incoming(
            magnitude,
            level,
            ready,
            draw(runtime, source, target, now, "hazard.critical")?,
        )
    }
}

#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct Criticals(Vec<(ExactActorRef, GameSessionId, u64)>);
impl Criticals {
    pub(in crate::gameplay_transport) fn ready(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        now: u64,
    ) -> bool {
        self.0
            .retain(|(a, s, _)| runtime.player_control_facts(*a, *s).is_ok());
        self.0
            .iter()
            .find(|(a, s, _)| *a == actor && *s == session)
            .is_none_or(|(_, _, last)| now.checked_sub(*last).is_some_and(|age| age >= 2_000_000))
    }
    pub(in crate::gameplay_transport) fn record(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
        now: u64,
    ) {
        if let Some((_, _, last)) = self
            .0
            .iter_mut()
            .find(|(a, s, _)| *a == actor && *s == session)
        {
            *last = now;
        } else {
            self.0.push((actor, session, now));
        }
    }
}

pub(in crate::gameplay_transport) fn draw(
    runtime: &ChannelRuntimeV1,
    source: ExactActorRef,
    target: ExactActorRef,
    now: u64,
    purpose: &str,
) -> Option<u64> {
    draw_identity(runtime, source.placement_identity(), target, now, purpose)
}

/// A swing keeps its draw across owner scheduling delays. The exact target also
/// separates the primary hit from each cleave recipient of the same swing.
pub(in crate::gameplay_transport) fn swing_dodge_roll(
    runtime: &ChannelRuntimeV1,
    source: ExactActorRef,
    target: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
) -> Option<u64> {
    let occurrence = oteryn_gameplay_combat_rules::resolution::swing_occurrence(
        source.placement_identity(),
        (
            *lineage.game_session_id().as_bytes(),
            lineage.command_id().get(),
        ),
        ordinal,
        &target.placement_identity(),
    );
    deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest()),
        occurrence,
        "hazard.dodge",
        0,
    )
    .ok()
}

/// Retain a zero-damage effect so the physical receipt, payment and cooldowns
/// still commit. Healing and condition installation keep their source semantics.
pub(in crate::gameplay_transport) fn dodge_spell_effects(
    runtime: &ChannelRuntimeV1,
    caster: ExactActorRef,
    command: CommandRef,
    level: u8,
    effects: &mut [crate::spell::combat_batch::OwnerCombatEffect],
) -> Option<()> {
    use crate::spell::combat_batch::OwnerCombatChange;
    for effect in effects {
        if !eligible(runtime, effect.target) || level == 0 {
            continue;
        }
        let damage = match &mut effect.change {
            OwnerCombatChange::Damage { magnitude, .. } => magnitude,
            OwnerCombatChange::DamageWithHealing { damage, .. } => damage,
            _ => continue,
        };
        if *damage > 0
            && dodges(
                level,
                spell_dodge_roll(runtime, caster, effect.target, command, effect.sub_ordinal)?,
            )
        {
            *damage = 0;
        }
    }
    Some(())
}

pub(in crate::gameplay_transport) fn spell_dodge_roll(
    runtime: &ChannelRuntimeV1,
    source: ExactActorRef,
    target: ExactActorRef,
    lineage: CommandRef,
    ordinal: u16,
) -> Option<u64> {
    let occurrence = oteryn_gameplay_combat_rules::resolution::swing_occurrence(
        source.placement_identity(),
        (
            *lineage.game_session_id().as_bytes(),
            lineage.command_id().get(),
        ),
        ordinal,
        &target.placement_identity(),
    );
    deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest()),
        occurrence,
        "hazard.spell.dodge",
        0,
    )
    .ok()
}

fn draw_identity(
    runtime: &ChannelRuntimeV1,
    source: [u8; 16],
    target: ExactActorRef,
    now: u64,
    purpose: &str,
) -> Option<u64> {
    use sha2::{Digest, Sha256};
    let bytes: [u8; 32] = Sha256::new()
        .chain_update(source)
        .chain_update(target.placement_identity())
        .chain_update(now.to_be_bytes())
        .finalize()
        .into();
    deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest()),
        DecisionOccurrenceId::from_bytes(bytes[..16].try_into().ok()?),
        purpose,
        0,
    )
    .ok()
}

pub(in crate::gameplay_transport) fn incoming(
    damage: u32,
    level: u8,
    ready: bool,
    roll: u64,
) -> Option<(u32, bool)> {
    if level == 0 || damage == 0 {
        return Some((damage, false));
    }
    if level > hazard::MAX_LEVEL {
        return None;
    }
    let critical = ready && roll < CRITICAL_THRESHOLD;
    // C++ adds ceil(negative_damage * rate), equivalent to flooring the positive bonus.
    let damage = u64::from(damage);
    let damage = if critical {
        damage + damage * (5000 + u64::from(level - 1) * 25) / 10000
    } else {
        damage
    };
    u32::try_from(damage + damage * u64::from(level) * 200 / 10000)
        .ok()
        .map(|d| (d, critical))
}

pub(in crate::gameplay_transport) fn dodges(level: u8, roll: u64) -> bool {
    level
        .checked_sub(1)
        .and_then(|i| DODGE_THRESHOLDS.get(usize::from(i)))
        .is_some_and(|threshold| roll < *threshold)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hazard_pinned_damage_rounding_critical_threshold_and_overflow() {
        assert_eq!(incoming(100, 0, true, 0), Some((100, false)));
        assert_eq!(incoming(100, 1, false, 0), Some((102, false)));
        assert_eq!(incoming(100, 12, false, 0), Some((124, false)));
        assert_eq!(incoming(10_000, 1, true, 0), Some((15_300, true)));
        assert_eq!(incoming(10_000, 12, true, 0), Some((18_941, true)));
        assert_eq!(
            incoming(100, 1, true, CRITICAL_THRESHOLD),
            Some((102, false))
        );
        assert_eq!(incoming(1, 1, true, 0), Some((1, true)));
        assert_eq!(incoming(0, 12, true, 0), Some((0, false)));
        assert_eq!(incoming(u32::MAX, 12, true, 0), None);
        assert_eq!(incoming(100, 13, false, 0), None);
    }
    #[test]
    fn hazard_pinned_dodge_has_no_level_zero_roll_and_monotonic_thresholds() {
        assert!(!dodges(0, 0));
        for level in 1..=hazard::MAX_LEVEL {
            let threshold = DODGE_THRESHOLDS[usize::from(level - 1)];
            assert!(dodges(level, threshold - 1));
            assert!(!dodges(level, threshold));
            assert!(!dodges(level, u64::MAX));
        }
        assert!(DODGE_THRESHOLDS.windows(2).all(|w| w[0] < w[1]));
    }
    #[test]
    fn hazard_critical_cooldown_is_per_victim_not_monster_and_prunes_stale_sessions() {
        let (runtime, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(81);
        let mut criticals = Criticals::default();
        assert!(criticals.ready(&runtime, actor, session, 10));
        criticals.record(actor, session, 10);
        assert!(!criticals.ready(&runtime, actor, session, 9));
        assert!(!criticals.ready(&runtime, actor, session, 2_000_009));
        assert!(criticals.ready(&runtime, actor, session, 2_000_010));
        let (other_runtime, other, other_session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(82);
        assert!(criticals.ready(&other_runtime, other, other_session, 11));
        assert!(criticals.0.is_empty());
    }
}
