//! PvP joins the existing exact-actor combat batch and source SQL owners.
use super::{
    ComposedFreshAdmission,
    actor_spell::{
        ChannelSpellStates, PlayerBatchPreflight, check_owner_batch, install_owner_batch,
        stage_player_batch,
    },
};
use crate::content::QualifiedNativeEntryRoom;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::durability::{DurabilityError, pvp_live as storage, spell_item_transaction as item_tx};
use crate::foundation::{ChannelRuntimeV1, CommandRef, ExactActorRef, StagedSpellBatch};
use crate::spell::combat_batch::OwnerCombatBatch;
use crate::world_runtime::LocalObjectRuntime;
use oteryn_gameplay_combat_rules::pvp::{ExpertMode, PvpActor, PvpTarget, check_pvp_legality};
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};

#[allow(clippy::too_many_arguments)]
pub(super) async fn legal_player(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &item_tx::SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    actor: ExactActorRef,
    target: ExactActorRef,
    secure: bool,
) -> Result<storage::LivePvpFacts, SpellCastDisposition> {
    let reject = || SpellCastDisposition::TargetIllegal;
    let source = states
        .get(runtime, actor, authority.game_session_id())
        .ok_or_else(reject)?;
    let (_, session, state) = states
        .actors
        .iter()
        .find(|(a, s, _)| *a == target && states.get(runtime, *a, *s).is_some())
        .ok_or_else(reject)?;
    if actor == target
        || states.is_dead(actor)
        || states.is_dead(target)
        || state.vitals().health == 0
    {
        return Err(reject());
    }
    let from = runtime
        .read_actor_position(actor)
        .map_err(|_| reject())?
        .position();
    let to = runtime
        .read_actor_position(target)
        .map_err(|_| reject())?
        .position();
    if !super::attack::sees(from, to) {
        return Err(reject());
    }
    let source_tile = crate::spell::world_execution::qualified_combat_tile_in_transaction(
        tx, authority, room, runtime, objects, from,
    )
    .await
    .map_err(|_| reject())?;
    let target_tile = crate::spell::world_execution::qualified_combat_tile_in_transaction(
        tx, authority, room, runtime, objects, to,
    )
    .await
    .map_err(|_| reject())?;
    let facts = storage::read_live_pvp_in_transaction(
        tx, root, authority, runtime, actor, target, *session,
    )
    .await
    .map_err(|_| reject())?;
    let a = PvpActor {
        level: source.character_facts().level,
        in_protection_zone: source_tile.protection_zone(),
        on_no_pvp_tile: source_tile.no_pvp_zone(),
        post_login_immune: facts.post_login_immune,
        black_skull: facts.actor_black,
        secure_mode: secure,
        // ATTACK_V1 predates expert mode. Its secure toggle permits ordinary aggression;
        // the four-mode extension remains a separate wire feature.
        expert_mode: ExpertMode::RedFist,
    };
    let t = PvpTarget {
        level: state.character_facts().level,
        in_protection_zone: target_tile.protection_zone(),
        on_no_pvp_tile: target_tile.no_pvp_zone(),
        marked: facts.marked,
    };
    check_pvp_legality(facts.world, &a, &t, &facts.relation).map_err(|_| reject())?;
    if !storage::unarmoured_pvp_target_in_transaction(tx, &facts)
        .await
        .map_err(|_| reject())?
    {
        return Err(reject());
    }
    Ok(facts)
}

/// Exact staged swing retained across a cancelled or unknown SQL COMMIT.
#[derive(Debug)]
pub struct PendingPvpHit {
    batch: OwnerCombatBatch,
    physical: StagedSpellBatch,
    player: PlayerBatchPreflight,
    target_character: [u8; 16],
    ordinal: u16,
    binding: [u8; 32],
}
fn park(hit: PendingPvpHit) -> Box<dyn std::any::Any + Send> {
    Box::new(super::actor_spell::native_combat_cast::UnresolvedSpellCommit::Pvp(Box::new(hit)))
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn pvp_selection_allowed(
        &self,
        permit: &SpellLanePermit,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        room: Option<&QualifiedNativeEntryRoom>,
        actor: ExactActorRef,
        session: crate::foundation::GameSessionId,
        command_id: u64,
        target: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    ) -> bool {
        let Some(named) = target else { return true };
        let visible = runtime.visible_entities();
        let Some(player) = visible.players.iter().find(|p| {
            p.actor.placement_identity() == named.identity && p.generation == named.generation
        }) else {
            return true;
        };
        if player.actor == actor {
            return true;
        }
        let Some(room) = room else {
            return false;
        };
        let modes = self
            .states
            .attack
            .lock()
            .await
            .combat_state(actor, session, self.owner_now())
            .modes;
        let Ok(id) = crate::foundation::CommandId::new(command_id) else {
            return false;
        };
        let objects = self.door.lock().await;
        self.pvp_target_legal(
            permit,
            runtime,
            states,
            room,
            &objects,
            actor,
            player.actor,
            CommandRef::new(session, id),
            modes.secure,
        )
        .await
        .is_some()
    }

    /// A source-fenced transaction is also used for target selection: no receipt or damage there.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn pvp_target_legal(
        &self,
        permit: &SpellLanePermit,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        room: &QualifiedNativeEntryRoom,
        objects: &LocalObjectRuntime,
        actor: ExactActorRef,
        target: ExactActorRef,
        command: CommandRef,
        secure: bool,
    ) -> Option<bool> {
        let Ok(Some(fence)) = self.current_quest_fence(command.game_session_id()).await else {
            return None;
        };
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            return None;
        };
        let mut context = (self, runtime, states, objects);
        pass.run_with_context(&mut context, move |holder, deadline, ctx| {
            Box::pin(async move {
                let (owner, runtime, states, objects) = ctx;
                let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = item_tx::assert_spell_item_authority_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    &fence.item_fence(),
                    command,
                    runtime.content_pin().server_artifact_digest(),
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                let result = legal_player(
                    &mut tx, owner.root, &authority, runtime, states, room, objects, actor, target,
                    secure,
                )
                .await
                .ok()
                .map(|f| f.target_black);
                tx.rollback().await?;
                Ok(result)
            })
        })
        .await
        .ok()
        .flatten()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn pvp_swing(
        &self,
        permit: &mut SpellLanePermit,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        room: &QualifiedNativeEntryRoom,
        objects: &LocalObjectRuntime,
        batch: OwnerCombatBatch,
        ordinal: u16,
        secure: bool,
        expected_black: bool,
    ) -> bool {
        let Some(effect) = batch.effects.first() else {
            return false;
        };
        let target = effect.target;
        let Some((_, target_session, _)) = states.actors.iter().find(|(a, _, _)| *a == target)
        else {
            return false;
        };
        let Ok(lease) = runtime.bound_attacker_lease(target, *target_session) else {
            return false;
        };
        let target_character = *lease.character_id().as_bytes();
        let Ok(mut physical) = runtime.stage_spell_batch(&batch) else {
            return false;
        };
        let Ok(player) = stage_player_batch(runtime, states, &batch, None) else {
            return false;
        };
        if states.deaths.try_reserve(batch.effects.len()).is_err()
            || states.pvp_locks.try_reserve(2).is_err()
            || runtime.reserve_spell_batch(&mut physical).is_err()
        {
            return false;
        }
        let binding: [u8; 32] = Sha256::digest(format!("{batch:?}").as_bytes()).into();
        let hit = PendingPvpHit {
            batch,
            physical,
            player,
            target_character,
            ordinal,
            binding,
        };
        let mut window = permit.open_commit_window(hit, park);
        // Even cancellation before COMMIT retains enough state to prove rollback and release.
        window.mark_already_committed();
        let Ok(Some(fence)) = self
            .current_quest_fence(window.attempt().batch.command.game_session_id())
            .await
        else {
            return false;
        };
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            return false;
        };
        let mut context = (self, &mut *runtime, &mut *states, objects, &mut window);
        let result = pass
            .run_with_context(&mut context, move |holder, deadline, ctx| {
                Box::pin(async move {
                    let (owner, runtime, states, objects, window) = ctx;
                    let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                    let hit = window.attempt();
                    let authority = item_tx::assert_spell_item_authority_in_transaction(
                        &mut tx,
                        window.permit(),
                        owner.root,
                        owner.character,
                        owner.holder,
                        &fence.item_fence(),
                        hit.batch.command,
                        runtime.content_pin().server_artifact_digest(),
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let facts = legal_player(
                        &mut tx,
                        owner.root,
                        &authority,
                        runtime,
                        states,
                        room,
                        objects,
                        hit.batch.caster,
                        target,
                        secure,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    if facts.target_black != expected_black {
                        return Err(DurabilityError::Unavailable);
                    }
                    let source = hit.batch.caster;
                    let source_session = hit.batch.command.game_session_id();
                    let target_session = states
                        .actors
                        .iter()
                        .find(|(a, _, _)| *a == target)
                        .ok_or(DurabilityError::Unavailable)?
                        .1;
                    window.attempt_mut().player.pvp_locks = vec![
                        (
                            source,
                            source_session,
                            !facts.relation.target_aggressor_of_actor,
                        ),
                        (target, target_session, false),
                    ];
                    let hit = window.attempt();
                    check_owner_batch(runtime, states, &hit.physical, Some(&hit.player))
                        .map_err(|_| DurabilityError::Unavailable)?;
                    use crate::foundation::runtime_actor_spell_types::PlayerBatchProof;
                    let damage = hit
                        .player
                        .health_receipt(target, ordinal)
                        .ok_or(DurabilityError::Unavailable)?;
                    let damage = u32::try_from(damage.health_before - damage.health_after)
                        .map_err(|_| DurabilityError::Unavailable)?;
                    storage::record_live_pvp_harm_in_transaction(
                        &mut tx,
                        &authority,
                        &facts,
                        ordinal,
                        hit.binding,
                        damage,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    #[cfg(test)]
                    if faults::take(faults::Point::BeforeCommit) {
                        return Err(DurabilityError::Unavailable);
                    }
                    storage::commit_live_pvp_transaction(tx, deadline, window).await?;
                    Ok(())
                })
            })
            .await;
        if result.is_err() {
            return false;
        }
        #[cfg(test)]
        if faults::take(faults::Point::AfterCommit) {
            return false;
        }
        let Ok(checked) = check_owner_batch(
            runtime,
            states,
            &window.attempt().physical,
            Some(&window.attempt().player),
        ) else {
            return false;
        };
        let hit = window.install();
        install_owner_batch(runtime, states, hit.physical, Some(hit.player), checked).applied
    }

    pub(super) async fn resolve_pvp_hit(&self, permit: &mut SpellLanePermit, hit: PendingPvpHit) {
        let mut window = permit.open_commit_window(hit, park);
        window.mark_already_committed();
        let command = window.attempt().batch.command;
        let Ok(Some(fence)) = self.current_quest_fence(command.game_session_id()).await else {
            return;
        };
        let Ok(pass) = self.root.try_issue_semantic_pass() else {
            return;
        };
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let mut context = (self, &mut *runtime, &mut *states, &mut window);
        let result = pass
            .run_with_context(&mut context, move |holder, deadline, ctx| {
                Box::pin(async move {
                    let (owner, runtime, _, window) = ctx;
                    let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                    let hit = window.attempt();
                    let authority = item_tx::assert_spell_item_authority_in_transaction(
                        &mut tx,
                        window.permit(),
                        owner.root,
                        owner.character,
                        owner.holder,
                        &fence.item_fence(),
                        command,
                        runtime.content_pin().server_artifact_digest(),
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let committed = storage::live_pvp_receipt_matches_in_transaction(
                        &mut tx,
                        &authority,
                        hit.ordinal,
                        hit.target_character,
                        hit.binding,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    tx.rollback().await?;
                    Ok(committed)
                })
            })
            .await;
        match result {
            Ok(true) => {
                let Ok(checked) = check_owner_batch(
                    &runtime,
                    &states,
                    &window.attempt().physical,
                    Some(&window.attempt().player),
                ) else {
                    return;
                };
                let hit = window.install();
                install_owner_batch(
                    &mut runtime,
                    &mut states,
                    hit.physical,
                    Some(hit.player),
                    checked,
                );
            }
            Ok(false) => {
                if runtime
                    .release_definitely_uncommitted_spell_batch(&window.attempt().physical)
                    .is_ok()
                {
                    window.release();
                }
            }
            Err(_) => {}
        }
    }
}
/// Native spells write harm in their existing cost/training SQL transaction.
#[allow(clippy::too_many_arguments)]
pub(super) async fn record_spell_harm(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &item_tx::SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    batch: &OwnerCombatBatch,
    player: &mut PlayerBatchPreflight,
    secure: bool,
) -> Result<(), SpellCastDisposition> {
    use crate::foundation::runtime_actor_spell_types::PlayerBatchProof;
    let mut pvp_locks = Vec::new();
    for effect in &batch.effects {
        if runtime.contains_live_creature(effect.target)
            || !matches!(
                effect.change,
                crate::spell::combat_batch::OwnerCombatChange::Damage { .. }
            )
        {
            continue;
        }
        let facts = legal_player(
            tx,
            root,
            authority,
            runtime,
            states,
            room,
            objects,
            batch.caster,
            effect.target,
            secure,
        )
        .await?;
        let capture: serde_json::Value =
            serde_json::from_slice(&batch.binding).map_err(|_| SpellCastDisposition::Rejected)?;
        let expected = serde_json::json!([
            crate::spell::combat_execution::actor_atom(effect.target),
            facts.target_black
        ]);
        if !capture["pvp_targets"]
            .as_array()
            .is_some_and(|targets| targets.contains(&expected))
        {
            return Err(SpellCastDisposition::TargetIllegal);
        }
        let receipt = player
            .health_receipt(effect.target, effect.sub_ordinal)
            .ok_or(SpellCastDisposition::Rejected)?;
        let damage = u32::try_from(receipt.health_before - receipt.health_after)
            .map_err(|_| SpellCastDisposition::Rejected)?;
        let binding = Sha256::digest(format!("{batch:?}").as_bytes()).into();
        storage::record_live_pvp_harm_in_transaction(
            tx,
            authority,
            &facts,
            effect.sub_ordinal,
            binding,
            damage,
        )
        .await
        .map_err(|_| SpellCastDisposition::Rejected)?;
        let target_session = states
            .actors
            .iter()
            .find(|(a, _, _)| *a == effect.target)
            .ok_or(SpellCastDisposition::Rejected)?
            .1;
        pvp_locks.push((
            batch.caster,
            batch.command.game_session_id(),
            !facts.relation.target_aggressor_of_actor,
        ));
        pvp_locks.push((effect.target, target_session, false));
    }
    player.pvp_locks = pvp_locks;
    Ok(())
}

#[path = "pvp_live_locks.rs"]
mod locks;
pub(super) use locks::{PvpBlocker, lock_mut, logout_blocked, pz_blocked, tracked};

#[cfg(test)]
pub(super) mod faults {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(in crate::gameplay_transport) enum Point {
        BeforeCommit,
        AfterCommit,
    }
    thread_local! {static ARMED:std::cell::Cell<Option<Point>>=const{std::cell::Cell::new(None)};}
    pub(in crate::gameplay_transport) fn arm(point: Point) {
        ARMED.set(Some(point));
    }
    pub(super) fn take(point: Point) -> bool {
        if ARMED.get() == Some(point) {
            eprintln!("PVP_FAULT consumed={point:?}");
            ARMED.set(None);
            true
        } else {
            false
        }
    }
}
