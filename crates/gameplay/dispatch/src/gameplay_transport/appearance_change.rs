//! Native appearance commands. Retained intents never rebase at a newer Character root.
mod catalogue;
mod command_replay;
pub mod observation;
pub(super) mod offer;
use super::connection::UseCommand;
use super::*;
use crate::content::native_gameplay::NativeCosmeticsView;
use crate::domain::appearance::{AppearanceContent, AppearanceSelection, Sex};
use crate::durability::character_appearance::{
    AppearanceChangeOccurrence, AppearanceChangeOutcome, AppearanceChangeRequest,
    AppearanceValidationContext, AppearanceWriteError, CurrentCharacterAppearance,
    DurableAppearanceSelection,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_protocol_oteryn::appearance as wire;
use std::collections::BTreeMap;

pub(crate) struct PreparedChange {
    command: u64,
    change: wire::Change,
    fence: CurrentCharacterGameplayFence,
    request: AppearanceChangeRequest,
    sex: Sex,
    content: NativeCosmeticsView,
}
#[derive(Default)]
pub struct AppearanceCommands(std::sync::Mutex<BTreeMap<GameSessionId, PreparedChange>>);

impl AppearanceCommands {
    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut commands) = self.0.lock() {
            commands.remove(&session);
        }
    }

    fn settle(&self, session: GameSessionId, command: u64) {
        if let Ok(mut commands) = self.0.lock()
            && commands
                .get(&session)
                .is_some_and(|pending| pending.command == command)
        {
            commands.remove(&session);
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn appearance_context(&self) -> Option<(wire::Context, NativeCosmeticsView)> {
        let generation = self.active_generation?;
        let native = generation.native_gameplay()?;
        let view = native.cosmetics()?.clone();
        Some((
            wire::Context {
                content_generation: generation.identity().client_artifact_digest(),
                cosmetics_digest: view.document_sha256(),
            },
            view,
        ))
    }

    pub(super) async fn change_appearance(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        change: wire::Change,
    ) -> Option<wire::ChangeResult> {
        let request = command_replay::request(command, &change)?;
        let Some(admitted) = command.item_fence else {
            return Some(wire::ChangeResult::Rejected);
        };
        if admitted.game_session_id != command.game_session_id {
            return Some(wire::ChangeResult::Rejected);
        }
        match self.root.reconcile_character_appearance_request(
            self.character,
            admitted.character_id,
            crate::domain::CharacterRevision::new(change.expected_root_revision).ok()?,
            request,
        ).await {
            Ok(Some(receipt)) => {
                self.states.appearance_commands.settle(command.game_session_id, command.command_id);
                return Some(wire::ChangeResult::Committed(receipt.committed_character_revision.get()));
            },
            Ok(None) => {},
            Err(AppearanceWriteError::Unavailable(_) | AppearanceWriteError::Fence(crate::durability::character_progression::CharacterProgressionError::Unavailable(_))) => return None,
            Err(_) => return Some(wire::ChangeResult::Rejected),
        }
        let existing = self
            .states
            .appearance_commands
            .0
            .lock()
            .ok()?
            .get(&command.game_session_id)
            .map(|p| {
                (
                    p.command,
                    p.change.clone(),
                    p.fence,
                    p.request.clone(),
                    p.sex,
                    p.content.clone(),
                )
            });
        let (fence, request, sex, content) = if let Some((
            id,
            original,
            fence,
            request,
            sex,
            content,
        )) = existing
        {
            if id != command.command_id || original != change {
                return Some(wire::ChangeResult::Rejected);
            }
            (fence, request, sex, content)
        } else {
            let fence = crate::gameplay_transport::character_gameplay_fence::current(
                self,
                command.game_session_id,
                false,
            )
            .await?;
            if command.item_fence != Some(fence.item_fence()) {
                return Some(wire::ChangeResult::Rejected);
            }
            let Some((context, content)) = self.appearance_context() else {
                return Some(wire::ChangeResult::Rejected);
            };
            if context != change.context
                || fence.expected_character_revision.get() != change.expected_root_revision
                || content.content_revision() != change.content_revision
            {
                return Some(wire::ChangeResult::Rejected);
            }
            let state = match self
                    .root
                    .read_current_character_appearance_state(
                        self.character,
                        self.holder,
                        fence,
                        content.earned_grant_keys(),
                    )
                    .await
                {
                    Ok(state) => state,
                    Err(AppearanceWriteError::Unavailable(_) | AppearanceWriteError::Fence(crate::durability::character_progression::CharacterProgressionError::Unavailable(_))) => return None,
                    Err(_) => return Some(wire::ChangeResult::Rejected),
                };
            let request = command_replay::request(command, &change)?;
            if request.before != state.selected {
                return Some(wire::ChangeResult::Rejected);
            }
            let mut commands = self.states.appearance_commands.0.lock().ok()?;
            if commands.contains_key(&command.game_session_id) {
                return None;
            }
            commands.insert(
                command.game_session_id,
                PreparedChange {
                    command: command.command_id,
                    change,
                    fence,
                    request: request.clone(),
                    sex: state.sex,
                    content: content.clone(),
                },
            );
            (fence, request, state.sex, content)
        };
        // A failed reconciliation cannot be classified as a pre-commit refusal.
        match self
            .root
            .reconcile_character_appearance(self.character, request.occurrence)
            .await
        {
            Ok(Some(receipt)) => {
                let outcome = receipt.matches_request(&fence, &request, sex).then_some(
                    wire::ChangeResult::Committed(receipt.committed_character_revision.get()),
                );
                if outcome.is_some() {
                    self.states
                        .appearance_commands
                        .settle(command.game_session_id, command.command_id);
                }
                return outcome;
            }
            Ok(None) => {}
            Err(_) => return None,
        }
        {
            let runtime = self.runtime.lock().await;
            if !current_runtime(&runtime, actor, command.game_session_id, &fence, &content) {
                self.states
                    .appearance_commands
                    .settle(command.game_session_id, command.command_id);
                return Some(wire::ChangeResult::Rejected);
            }
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_appearance(
                self.root,
                self.character,
                self.holder,
                fence,
                request,
                AppearanceValidationContext {
                    sex,
                    content: content.clone(),
                    content_revision: content.content_revision().to_owned(),
                },
            )
            .await;
        let result = match outcome {
            Ok(
                AppearanceChangeOutcome::Committed(receipt)
                | AppearanceChangeOutcome::AlreadyCommitted(receipt),
            ) => Some(wire::ChangeResult::Committed(
                receipt.committed_character_revision.get(),
            )),
            Err(AppearanceWriteError::Unavailable(_)) => None,
            Err(AppearanceWriteError::Fence(
                crate::durability::character_progression::CharacterProgressionError::Unavailable(_),
            )) => None,
            Err(_) => Some(wire::ChangeResult::Rejected),
        };
        if result.is_some() {
            self.states
                .appearance_commands
                .settle(command.game_session_id, command.command_id);
        }
        result
    }
}
fn selection(selection: &wire::Selection) -> AppearanceSelection<String> {
    AppearanceSelection {
        outfit_key: selection.outfit_key.clone(),
        colours: selection.colours,
        addons: selection.addons,
        mount_key: selection.mount_key.clone(),
    }
}
fn current_runtime(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: &CurrentCharacterGameplayFence,
    content: &NativeCosmeticsView,
) -> bool {
    runtime.owner_fence().is_ok()
        && runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        && runtime
            .bound_attacker_lease(actor, session)
            .is_ok_and(|lease| {
                lease.generation() == fence.character_lease_generation
                    && lease.character_id().as_bytes() == fence.character_id.as_bytes()
            })
        && runtime.content_pin().server_artifact_digest() == content.source_digest()
        && fence.runtime_scope
            == RuntimeScopeRefV1::channel(
                runtime.binding().world_id(),
                runtime.binding().channel_id(),
            )
        && fence.scope_ownership_generation == runtime.binding().scope_generation()
}

macro_rules! authority_methods {
    () => {
        fn appearance_available(&self) -> bool { false }
        fn appearance_observation(&self, _actor: crate::foundation::ExactActorRef, _session: crate::foundation::GameSessionId, _entities: Vec<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>) -> impl std::future::Future<Output=Option<super::appearance_change::observation::Observation>> { async { None } }
        fn appearance_change(&self, _actor: crate::foundation::ExactActorRef, _command: super::connection::UseCommand, _change: oteryn_protocol_oteryn::appearance::Change) -> impl std::future::Future<Output=Option<oteryn_protocol_oteryn::appearance::ChangeResult>> { async { Some(oteryn_protocol_oteryn::appearance::ChangeResult::Rejected) } }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn appearance_available(&self) -> bool {
            self.appearance_context().is_some()
        }
        async fn appearance_observation(
            &self,
            actor: ExactActorRef,
            session: GameSessionId,
            entities: Vec<EntityRef>,
        ) -> Option<appearance_change::observation::Observation> {
            appearance_change::observation::observe(self, actor, session, entities).await
        }
        async fn appearance_change(
            &self,
            actor: ExactActorRef,
            command: connection::UseCommand,
            change: oteryn_protocol_oteryn::appearance::Change,
        ) -> Option<oteryn_protocol_oteryn::appearance::ChangeResult> {
            self.change_appearance(actor, command, change).await
        }
    };
}
pub(super) use composed_methods;
