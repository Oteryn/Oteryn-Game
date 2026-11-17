use super::*;
use crate::durability::DurabilityError;
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_entities::carrier::{NativeRaidSpawnSpec, RaidSpawnIdentity};
use oteryn_gameplay_world_change::thornfire::WOLF;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn project_thornfire_wolf(&self, command: UseCommand) {
        let (Some(fence), Ok(id)) = (command.item_fence, CommandId::new(command.command_id)) else {
            return;
        };
        let mut context = (self, fence);
        let command = CommandRef::new(command.game_session_id, id);
        for attempt in 0..3 {
            let result = self
                .root
                .project_thornfire_wolf(
                    self.character,
                    self.holder,
                    fence,
                    command,
                    &mut context,
                    |context, firing_id, (x, y, floor)| {
                        let (owner, fence) = *context;
                        let mut runtime = owner
                            .runtime
                            .try_lock()
                            .map_err(|_| DurabilityError::Unavailable)?;
                        if runtime.binding().runtime_scope() != fence.runtime_scope
                            || runtime.binding().scope_generation()
                                != fence.scope_ownership_generation
                        {
                            return Err(DurabilityError::Unavailable);
                        }
                        let policy = runtime
                            .companion_policy(WOLF)
                            .map_err(|_| DurabilityError::Unavailable)?;
                        let spec = NativeRaidSpawnSpec::qualified(
                            WOLF,
                            &policy.definition_revision,
                            runtime.content_pin().server_artifact_digest(),
                            false,
                        )
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                        let stamp = runtime
                            .issue_owner_work()
                            .map_err(|_| DurabilityError::Unavailable)?;
                        // Reuse the native one-occurrence creature ledger for retries after COMMIT.
                        runtime
                            .realize_raid_spawn(
                                stamp,
                                RaidSpawnIdentity {
                                    firing_id,
                                    wave: 0,
                                    entry: 0,
                                    index: 0,
                                },
                                &spec,
                                Some(MovementLocalPosition { x, y, floor }),
                                |runtime, at| {
                                    Some(super::super::arena_match::walkable(owner, runtime, at))
                                },
                            )
                            .map_err(|_| DurabilityError::Unavailable)?;
                        Ok(())
                    },
                )
                .await;
            match result {
                Ok(()) => return,
                Err(DurabilityError::Unavailable) if attempt < 2 => tokio::task::yield_now().await,
                Err(_) => break,
            }
        }
        super::super::operator_event("thornfire_wolf_projection_deferred");
    }
}
