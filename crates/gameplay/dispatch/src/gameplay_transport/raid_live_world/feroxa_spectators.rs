//! Crystal9f5 grimvale.lua:72-92 snapshots players at30/45m and returns them60s later.
use super::*;
use crate::foundation::{ExactActorRef, GameSessionId, ScopeOwnershipGeneration};

const CAPTURE_AT: [u64; 2] = [1_800_000, 2_700_000];
const RETURN_AFTER_MS: u64 = 60_000;

struct SavedReturn {
    actor: ExactActorRef,
    session: GameSessionId,
    destination: MovementLocalPosition,
    due_ms: u64,
}

type SpectatorMove = (
    ExactActorRef,
    GameSessionId,
    MovementLocalPosition,
    MovementLocalPosition,
);

/// Volatile source callbacks belong to one actual Channel owner, just like Lua
/// addEvent. They cannot reconstruct positions lost during an owner restart.
#[derive(Default)]
pub(in crate::gameplay_transport) struct FeroxaSpectators {
    binding: Option<([u8; 16], ScopeOwnershipGeneration)>,
    captured: u8,
    pending: Vec<SavedReturn>,
    next_poll_ms: Option<i64>,
}

impl FeroxaSpectators {
    fn advance(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &crate::content::NativeEntryMovementCells,
        map: &crate::map::boot::BundleMap,
        firing: [u8; 16],
        elapsed_ms: u64,
        now_ms: i64,
    ) -> Result<Vec<SpectatorMove>, DurabilityError> {
        let binding = (firing, runtime.binding().scope_generation());
        if self.binding != Some(binding) {
            self.binding = Some(binding);
            self.pending.clear();
            // Recovery can resume upcoming callbacks, but has no authority to
            // invent the historical positions of already elapsed callbacks.
            self.captured = CAPTURE_AT.iter().enumerate().fold(0, |mask, (index, at)| {
                mask | if elapsed_ms > *at { 1 << index } else { 0 }
            });
        }
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let mut moves = Vec::new();
        let pending = std::mem::take(&mut self.pending);
        for saved in pending {
            if elapsed_ms < saved.due_ms {
                self.pending.push(saved);
                continue;
            }
            // A source callback attempts once. Logout, replaced actor/session,
            // or a blocked landing must not become a future surprise teleport.
            let Ok(before) = runtime.read_actor_position(saved.actor) else {
                continue;
            };
            if before.position() == saved.destination {
                continue;
            }
            let Ok(proof) = crate::map::floor::prepare_quest_encounter_ejection(
                runtime,
                cells,
                saved.actor,
                saved.session,
                before,
                saved.destination,
                &closed,
            ) else {
                continue;
            };
            let Ok(after) = runtime.commit_source_step(proof) else {
                continue;
            };
            moves.push((
                saved.actor,
                saved.session,
                before.position(),
                after.position(),
            ));
        }
        // Process due returns before a later capture: source31m precedes45m.
        for (index, at) in CAPTURE_AT.iter().enumerate() {
            let bit = 1 << index;
            if self.captured & bit != 0 || elapsed_ms < *at {
                continue;
            }
            let due_ms = elapsed_ms
                .checked_add(RETURN_AFTER_MS)
                .ok_or(DurabilityError::InvalidStoredState)?;
            let players = runtime
                .positioned_melee_census()
                .map_err(|_| DurabilityError::Unavailable)?;
            // The census is already bounded by the current Channel actor capacity;
            // at most two finite callbacks exist, separated by fifteen minutes.
            for (actor, position, session) in players {
                let at = position.position();
                if let Some(session) = session
                    && at.floor == 11
                    && (33412..=33448).contains(&at.x)
                    && (31519..=31555).contains(&at.y)
                {
                    self.pending.push(SavedReturn {
                        actor,
                        session,
                        destination: at,
                        due_ms,
                    });
                }
            }
            self.captured |= bit;
        }
        let next = self
            .pending
            .iter()
            .map(|saved| saved.due_ms)
            .chain(
                CAPTURE_AT
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| self.captured & (1 << index) == 0)
                    .map(|(_, at)| *at),
            )
            .min();
        self.next_poll_ms = next
            .and_then(|due| due.checked_sub(elapsed_ms))
            .and_then(|remaining| i64::try_from(remaining).ok())
            .and_then(|remaining| now_ms.checked_add(remaining));
        Ok(moves)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn apply_feroxa_spectators(
        &self,
        firing: [u8; 16],
        now_ms: i64,
        timers: &mut FeroxaSpectators,
    ) -> Result<(), ()> {
        // Denial-only cache: no current authority is needed to perform no action.
        // A due callback always reacquires every live fence below.
        if timers.binding.is_some_and(|binding| binding.0 == firing)
            && timers.next_poll_ms.is_none_or(|next| now_ms < next)
        {
            return Ok(());
        }
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let mut runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        if binding.world_id() != self.world_id || binding.channel_id() != Some(self.channel_id) {
            return Err(());
        }
        let identity = crate::durability::raid_runner::RaidStepIdentity {
            firing_id: firing,
            channel: self.channel_id,
            generation: binding.scope_generation(),
            ordinal: 0,
        };
        let pass = self.root.try_issue_semantic_pass().map_err(|_| ())?;
        let mut context = (self, &mut *runtime, &permit, timers);
        let moves = pass
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    use crate::durability::spell_item_transaction::{
                        assert_spell_item_scope_in_transaction, begin_spell_owner_transaction,
                    };
                    let (owner, runtime, permit, timers) = context;
                    let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                    let binding = runtime.binding();
                    let _authority = assert_spell_item_scope_in_transaction(
                        &mut tx,
                        permit,
                        owner.root,
                        owner.character,
                        owner.holder,
                        binding.runtime_scope(),
                        binding.scope_generation().get(),
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let map = owner.bundle_map().ok_or(DurabilityError::Unavailable)?;
                    let elapsed =
                        crate::durability::raid_runner::feroxa_map_elapsed_in_transaction(
                            &mut tx,
                            owner.holder,
                            identity,
                            owner.world_id,
                            owner.reset_epoch,
                            map.overlay.base().digest(),
                            now_ms,
                        )
                        .await?;
                    if !crate::durability::world_event::changes::callbacks::require(
                        &mut tx,
                        owner.world_id,
                        owner.reset_epoch,
                        "oteryn:quest-callback/grimvale-full-moon-entry",
                    )
                    .await?
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    let objects = owner.door.lock().await;
                    let moves = timers.advance(
                        runtime,
                        owner.movement_cells(),
                        map,
                        firing,
                        elapsed,
                        now_ms,
                    )?;
                    drop(objects);
                    tx.rollback().await?;
                    Ok(moves)
                })
            })
            .await
            .map_err(|_| ())?;
        if !moves.is_empty() {
            let map = self.bundle_map().ok_or(())?;
            let mut states = self.states.spell_states.lock().await;
            for (actor, session, from, to) in moves {
                states.stage_special_movement_step(
                    &runtime,
                    map,
                    actor,
                    session,
                    self.owner_now().get(),
                    Some(from),
                    to,
                    None,
                );
                states.stage_underwater_step(
                    &runtime,
                    actor,
                    session,
                    self.owner_now().get(),
                    super::super::paced_movement::underwater_drown::underwater(map, from),
                    super::super::paced_movement::underwater_drown::underwater(map, to),
                );
            }
        }
        Ok(())
    }
}
