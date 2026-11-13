//! Crystal201189f soul_war_mechanics.lua364-398: one original33876 STEP,
//! any-outfit early return, optional named Spite heal, then Item/outfit/message.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::creature_source_items::{SourceGroundStep, SourceGroundStepKind};
use crate::foundation::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues,
};
use crate::spell::cast::PlayerSpellState;
use oteryn_gameplay_encounters::bosses::goshnars_spite;
use oteryn_gameplay_entities::{CompanionSnapshot, carrier::soul_war_spite::SpiteChange};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

#[derive(Debug, Clone)]
pub(super) struct PendingWeeping {
    input: SourceGroundStep,
    before: Arc<PlayerSpellState>,
    after: Arc<PlayerSpellState>,
    boss: Option<(CompanionSnapshot, u32)>,
    heal: bool,
}

fn has_outfit(player: &PlayerSpellState, now: u64) -> bool {
    player
        .conditions
        .active_at(crate::domain::condition::ConflictKey::Outfit, now)
        .is_some()
}

fn prepare(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
    player: &PlayerSpellState,
    input: SourceGroundStep,
    boss: Option<(CompanionSnapshot, u32)>,
    now: u64,
) -> Result<PendingWeeping, ()> {
    if input.kind() != SourceGroundStepKind::Weeping
        || !input.current_for(runtime)
        || has_outfit(player, now)
    {
        return Err(());
    }
    let root = GameplayDecisionRoot::from_bytes(native.source_digest());
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&input.binding()[..16]);
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
    let draw =
        deterministic_decision_u64(&root, occurrence, "spite_weeping_heal", 0).map_err(|_| ())?;
    let draw = u8::try_from(crate::spell::uniform_draw(draw, 1, 100)).map_err(|_| ())?;
    // luaCreatureAddHealth calls combatChangeHealth(nullptr,...), which returns
    // false for HP0 (game.cpp7802-7812). Lua ignores that result and continues.
    let maximum = boss.as_ref().and_then(|(boss, _)| {
        (boss.health > 0)
            .then(|| u64::try_from(boss.maximum_health).ok())
            .flatten()
    });
    let plan = goshnars_spite::corpse_step(false, maximum, draw).ok_or(())?;
    let (look_type, duration_ms) = plan.outfit.ok_or(())?;
    if !plan.consume {
        return Err(());
    }
    let appearance = native
        .spell_appearances()
        .and_then(|index| index.for_look_type(u32::from(look_type)))
        .ok_or(())?;
    let definition = ConditionDefinition::new(
        "srx3:soul-war:weeping-outfit",
        1,
        ConditionValues::Outfit {
            look_type: u32::from(look_type),
            duration_ms: u32::try_from(duration_ms).map_err(|_| ())?,
        },
    )
    .and_then(|definition| definition.with_appearance(appearance.selection()))
    .ok_or(())?;
    let facts = ApplicationFacts {
        now,
        base_speed: u16::try_from(player.owned_base_speed()).map_err(|_| ())?,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence,
    };
    let mut after = player.clone();
    after
        .conditions
        .apply(
            &definition,
            Some("source:weeping-soul-step".to_owned()),
            ConditionSourceKind::Field,
            &[],
            &facts,
        )
        .map_err(|_| ())?;
    after.source_set_batch_revision(player.revision().checked_add(1).ok_or(())?);
    Ok(PendingWeeping {
        input,
        before: Arc::new(player.clone()),
        after: Arc::new(after),
        boss,
        heal: plan.heal > 0,
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::expect_used,
        reason = "exact reserved slots and the complete Player predecessor are checked before synchronous publication"
    )]
    pub(super) async fn resolve_one_weeping_step(
        &self,
        permit: &SpellLanePermit,
        mut pending: PendingStep,
    ) -> Result<(), ()> {
        if pending.weeping.is_none() {
            let input = pending
                .movement
                .source_ground_steps()
                .iter()
                .find(|input| !pending.completed_items.contains(&input.item_instance_id()))
                .cloned();
            let names = {
                let mut runtime = self.runtime.lock().await;
                let mut states = self.states.spell_states.try_lock().map_err(|_| ())?;
                runtime
                    .validate_source_actor_reservation(&pending.reservation, &pending.movement)
                    .map_err(|_| ())?;
                let player = states
                    .get(
                        &runtime,
                        pending.movement.actor(),
                        pending.movement.session(),
                    )
                    .ok_or(())?;
                if input.is_none() {
                    return finish(&mut runtime, &mut states, &pending);
                }
                if has_outfit(player, self.owner_now().get()) {
                    // Lua returns nil. The original MoveEvents dispatcher
                    // stops this movement's callback vector, including later
                    //33949/33984 originals; no Item or effect is published.
                    return finish(&mut runtime, &mut states, &pending);
                }
                self.prepare_source_names(&runtime, &["Goshnar's Spite"])?
            };
            let names = self.read_source_names(permit, &names).await?;
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.try_lock().map_err(|_| ())?;
            runtime
                .validate_source_actor_reservation(&pending.reservation, &pending.movement)
                .map_err(|_| ())?;
            let names = self.seal_source_names(&runtime, names)?;
            let native = self
                .active_generation
                .and_then(|generation| generation.native_gameplay())
                .ok_or(())?;
            let target = if let Some(named) =
                super::super::source_names::creature_named(&names, "Goshnar's Spite")?
            {
                if named.role != super::super::source_names::SourceNameRole::Creature
                    || super::super::soul_war_callbacks::source_actor(&runtime, native, named.actor)
                        .as_deref()
                        != Some(oteryn_gameplay_entities::carrier::soul_war_spite::SPITE)
                {
                    return Err(());
                }
                let snapshot = runtime
                    .companion_snapshot_including_dead(named.actor)
                    .map_err(|_| ())?;
                let defense = runtime
                    .effective_creature_defense(&snapshot)
                    .map_err(|_| ())?;
                if snapshot.health < 0
                    || snapshot.maximum_health <= 0
                    || snapshot.health > snapshot.maximum_health
                {
                    return Err(());
                }
                Some((snapshot, defense))
            } else {
                None
            };
            let player = states
                .get(
                    &runtime,
                    pending.movement.actor(),
                    pending.movement.session(),
                )
                .ok_or(())?;
            let now = self.owner_now().get();
            if has_outfit(player, now) {
                return finish(&mut runtime, &mut states, &pending);
            }
            let effect = prepare(&runtime, native, player, input.ok_or(())?, target, now)?;
            let index = states
                .spite_steps
                .pending
                .iter()
                .position(|entry| {
                    entry.movement.event_identity() == pending.movement.event_identity()
                })
                .ok_or(())?;
            if states.spite_steps.pending[index].weeping.is_some() {
                return Err(());
            }
            let affected = effect.boss.as_ref().map(|(boss, _)| boss.actor);
            runtime
                .extend_source_actor_reservation(
                    &mut pending.reservation,
                    &pending.movement,
                    affected.as_slice(),
                )
                .map_err(|_| ())?;
            pending.weeping = Some(effect);
            // The same reservation now contains the original Player and the
            // actual selected Spite, before the first physical Item SQL await.
            states.spite_steps.pending[index] = pending.clone();
        }
        let effect = pending.weeping.as_ref().ok_or(())?;
        let outcome = self
            .root
            .commit_source_ground_step(permit, self.character, self.holder, &effect.input)
            .await
            .map_err(|_| ())?;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.try_lock().map_err(|_| ())?;
        runtime
            .validate_source_actor_reservation(&pending.reservation, &pending.movement)
            .map_err(|_| ())?;
        let index = states
            .spite_steps
            .pending
            .iter()
            .position(|entry| entry.movement.event_identity() == pending.movement.event_identity())
            .ok_or(())?;
        if states.spite_steps.pending[index]
            .weeping
            .as_ref()
            .is_none_or(|retained| retained.input.binding() != effect.input.binding())
        {
            return Err(());
        }
        if !outcome.removed {
            let target = effect.boss.as_ref().map(|(boss, _)| boss.actor);
            runtime
                .release_source_actor_targets_after_observed_outcome(
                    &mut pending.reservation,
                    &pending.movement,
                    target.as_slice(),
                )
                .map_err(|_| ())?;
            pending
                .completed_items
                .push(effect.input.item_instance_id());
            pending.weeping = None;
            states.spite_steps.pending[index] = pending;
            return Ok(());
        }
        if states.get(
            &runtime,
            pending.movement.actor(),
            pending.movement.session(),
        ) != Some(effect.before.as_ref())
        {
            return Err(());
        }
        if let Some((expected, defense)) = &effect.boss {
            runtime
                .validate_companion_snapshot(expected)
                .map_err(|_| ())?;
            if !runtime
                .source_creature_is_registered(expected.actor)
                .map_err(|_| ())?
                || runtime
                    .effective_creature_defense(expected)
                    .map_err(|_| ())?
                    != *defense
            {
                return Err(());
            }
        }
        // Release only the independently added Spite target after its Item ACK.
        // The same original Player reservation protects every remaining Item
        // callback, including33949/33984 interleaved with other33876 originals.
        let target = effect.boss.as_ref().map(|(boss, _)| boss.actor);
        runtime
            .release_source_actor_targets_after_observed_outcome(
                &mut pending.reservation,
                &pending.movement,
                target.as_slice(),
            )
            .map_err(|_| ())?;
        if effect.heal {
            let (boss, defense) = effect.boss.as_ref().expect("prepared positive Spite heal");
            runtime
                .change_soul_war_spite(boss, *defense, SpiteChange::WeepingHeal)
                .expect("current reserved qualified live Spite heal");
        }
        states
            .commit(
                &runtime,
                pending.movement.actor(),
                pending.movement.session(),
                effect.after.as_ref().clone(),
            )
            .then_some(())
            .expect("prepared direct Player outfit successor");
        self.states.loop_ground.invalidate();
        self.states.server_messages.send_current(
            pending.movement.session(),
            ServerMessage::Advance("You are soaked by tears of the weeping soul!".into()),
        );
        pending
            .completed_items
            .push(effect.input.item_instance_id());
        pending.weeping = None;
        states.spite_steps.pending[index] = pending;
        Ok(())
    }
}
