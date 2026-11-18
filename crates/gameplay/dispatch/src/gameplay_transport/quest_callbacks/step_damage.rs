//! Crystal 201189f8 data-global/scripts/quests/the_pits_of_inferno_quest/movements_fire_tile.lua:1-31,47-54.
use super::*;
use crate::{
    content::{ReferenceResistanceKind, spell_data::Vocation},
    durability::{
        DurabilityError,
        item_transfer::CurrentCharacterItemFence,
        pvp_live,
        spell_item_transaction::{
            begin_field_owner_transaction, read_standing_player_tile_in_transaction,
        },
        spell_items_abi::SpellGroundTarget,
    },
    foundation::owner_timer::SemanticTimeMicros,
    player_lethal::PlayerHitKind,
};
use oteryn_gameplay_quests::quest::world_callbacks::{
    StepDamage, StepDamageKind, StepDamageVocation, VocationStepDamage,
};
use oteryn_protocol_oteryn::analyser::AnalyserDamageSource;
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub(in crate::gameplay_transport) struct PendingStepDamage {
    actor: ExactActorRef,
    session: GameSessionId,
    entered: MovementPositionSnapshot,
    vitals_revision: u64,
    source_key: u64,
    source_revision: u64,
    fence: CurrentCharacterItemFence,
    target: SpellGroundTarget,
    content: [u8; 32],
    occurrence: [u8; 32],
    hit: StepDamage,
    raw: u32,
    reduced: u32,
}

impl PendingStepDamage {
    fn park(self) -> Box<dyn std::any::Any + Send> {
        Box::new(
            crate::gameplay_transport::actor_spell::native_combat_cast::UnresolvedSpellCommit::QuestStepDamage(
                Box::new(self),
            ),
        )
    }

    fn matches_current(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        map: &BundleMap,
    ) -> bool {
        runtime.read_actor_position(self.actor) == Ok(self.entered)
            && !map.facts.base_hidden(self.source_key)
            && map.facts.object_revision(self.source_key) == self.source_revision
            && states
                .get(runtime, self.actor, self.session)
                .is_some_and(|state| state.revision() == self.vitals_revision)
            && !states.is_dead(self.actor)
            && !states.has_pending_spell_commit(self.actor, self.session)
            && runtime.assert_actor_spell_unreserved(self.actor).is_ok()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "rechecks the exact movement and map source under the Channel turn"
    )]
    pub(super) async fn apply_vocation_step_damage(
        &self,
        permit: &mut SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        damage: VocationStepDamage,
    ) {
        let Some(room) = self.rooms().gate_periodic_damage() else {
            return;
        };
        let Some(content) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.read_actor_position(actor) != Ok(entered)
            || map.facts.base_hidden(key)
            || !matches(map, callback, key)
            || states.is_dead(actor)
        {
            return;
        }
        let Some(state) = states.get(&runtime, actor, session) else {
            return;
        };
        let base = match state.source_party_vocation() {
            Vocation::Sorcerer | Vocation::MasterSorcerer => Some(StepDamageVocation::Sorcerer),
            Vocation::Druid | Vocation::ElderDruid => Some(StepDamageVocation::Druid),
            Vocation::Paladin | Vocation::RoyalPaladin => Some(StepDamageVocation::Paladin),
            Vocation::Knight | Vocation::EliteKnight => Some(StepDamageVocation::Knight),
            Vocation::Monk | Vocation::ExaltedMonk => Some(StepDamageVocation::Monk),
            Vocation::None => None,
        };
        let hit = if base == Some(damage.vocation) {
            damage.matching
        } else {
            damage.other
        };
        let Ok(target) =
            SpellGroundTarget::for_native_tile_read(room, &runtime, entered.position())
        else {
            return;
        };
        let occurrence = Sha256::new()
            .chain_update(b"oteryn:quest-step-damage:v1")
            .chain_update(actor.placement_identity())
            .chain_update(session.as_bytes())
            .chain_update(entered.revision().to_be_bytes())
            .chain_update(key.to_be_bytes())
            .finalize()
            .into();
        let raw = passive_damage(state.equipped_element_protection(resistance(hit.kind)), hit);
        let pending = PendingStepDamage {
            actor,
            session,
            entered,
            vitals_revision: state.revision(),
            source_key: key,
            source_revision: map.facts.object_revision(key),
            fence: fence.item_fence(),
            target,
            content: content.source_digest(),
            occurrence,
            hit,
            raw,
            reduced: raw,
        };
        self.settle_quest_step_damage(permit, pending, false, &runtime, &mut states, map)
            .await;
    }

    pub(in crate::gameplay_transport) async fn resolve_quest_step_damage(
        &self,
        permit: &mut SpellLanePermit,
        pending: PendingStepDamage,
    ) {
        let Some(map) = self.bundle_map() else {
            permit
                .open_commit_window(pending, PendingStepDamage::park)
                .park();
            return;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        self.settle_quest_step_damage(permit, pending, true, &runtime, &mut states, map)
            .await;
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "retained hit and equipment receipt share the current Channel owner"
    )]
    async fn settle_quest_step_damage(
        &self,
        permit: &mut SpellLanePermit,
        pending: PendingStepDamage,
        recovering: bool,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
    ) {
        let current = pending.matches_current(runtime, states, map);
        if !current && !recovering {
            return;
        }
        let mut window = permit.open_commit_window(pending, PendingStepDamage::park);
        if recovering {
            window.mark_already_committed();
        }
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            if !recovering {
                window.release();
            }
            return;
        };
        let result = pass
            .run_with_context(&mut (self, &mut window), |holder, deadline, context| {
                Box::pin(async move {
                    let (owner, window) = context;
                    let mut tx = begin_field_owner_transaction(holder, deadline).await?;
                    let hit = window.attempt();
                    let read = read_standing_player_tile_in_transaction(
                        &mut tx,
                        window.permit(),
                        owner.root,
                        owner.character,
                        owner.holder,
                        &hit.fence,
                        hit.content,
                        &hit.target,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let binding = Sha256::new()
                        .chain_update(hit.content)
                        .chain_update(hit.vitals_revision.to_be_bytes())
                        .chain_update(hit.source_revision.to_be_bytes())
                        .chain_update(hit.raw.to_be_bytes())
                        .chain_update(hit.hit.effect.to_be_bytes())
                        .finalize()
                        .into();
                    if recovering
                        && pvp_live::field_cycle_matches_in_transaction(
                            &mut tx,
                            &read,
                            hit.occurrence,
                            binding,
                        )
                        .await
                        .map_err(|_| DurabilityError::Unavailable)?
                    {
                        tx.into_inner().rollback().await?;
                        return Ok(true);
                    }
                    if !current {
                        tx.into_inner().rollback().await?;
                        return Ok(false);
                    }
                    // Crystal Player::blockHit (player.cpp:3920-3950) consumes a
                    // charge on ordinary absorption; field-only resistance is excluded.
                    let reduced = owner
                        .equipped_step_damage(
                            &mut tx,
                            &read,
                            resistance(hit.hit.kind),
                            hit.raw,
                            hit.occurrence,
                        )
                        .await?;
                    pvp_live::record_field_cycle_in_transaction(
                        &mut tx,
                        &read,
                        hit.occurrence,
                        binding,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    window.attempt_mut().reduced = reduced;
                    pvp_live::commit_live_pvp_transaction(tx.into_inner(), deadline, window)
                        .await?;
                    Ok(true)
                })
            })
            .await;
        match result {
            Ok(true) if current => {
                let hit = window.install();
                self.apply_quest_amount(
                    runtime,
                    states,
                    hit.actor,
                    hit.session,
                    hit.entered,
                    map,
                    hit.hit,
                    hit.reduced,
                );
            }
            Ok(_) => {
                window.release();
            }
            Err(_) => {
                let _ = window.reclaim_uncommitted();
            }
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "damage and presentation use the same current actor snapshot"
    )]
    pub(super) fn apply_quest_hit(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
        map: &BundleMap,
        hit: StepDamage,
    ) {
        let Some(state) = states.get(runtime, actor, session) else {
            return;
        };
        let amount = passive_damage(state.equipped_element_protection(resistance(hit.kind)), hit);
        self.apply_quest_amount(runtime, states, actor, session, entered, map, hit, amount);
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the combat owner and cue use the same hit"
    )]
    fn apply_quest_amount(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
        map: &BundleMap,
        hit: StepDamage,
        amount: u32,
    ) {
        let kind = match hit.kind {
            StepDamageKind::Fire => PlayerHitKind::Fire,
            StepDamageKind::Physical => PlayerHitKind::Physical,
        };
        if states
            .apply_creature_damage_kind(
                runtime,
                actor,
                session,
                amount,
                kind,
                SemanticTimeMicros::from_micros(self.owner_now().get()),
                AnalyserDamageSource::None,
            )
            .is_some()
            && let Some(position) = native(entered.position())
        {
            map.facts
                .emit_movement_effect(position, u16::from(hit.effect));
        }
    }
}

fn resistance(kind: StepDamageKind) -> ReferenceResistanceKind {
    match kind {
        StepDamageKind::Fire => ReferenceResistanceKind::Fire,
        StepDamageKind::Physical => ReferenceResistanceKind::Physical,
    }
}

fn passive_damage(
    protection: Option<crate::content::ReferenceRationalPercent>,
    hit: StepDamage,
) -> u32 {
    protection
        .and_then(|protection| {
            oteryn_gameplay_items::equipment_effects::apply_protection(
                i64::from(hit.amount),
                protection,
            )
        })
        .and_then(|amount| u32::try_from(amount).ok())
        .unwrap_or(u32::from(hit.amount))
}
