//! Server rules retain exact callback inputs; answers never supply gameplay authority.
use super::{ComposedFreshAdmission, ServerModalOutcome, actor_is_current, owner::Root};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::gameplay_transport::{
    connection::{AdmittedSession, UseCommand, UseOutcome},
    item_view::ItemKey,
};
use crate::map::view::MapFacts as _;
use oteryn_protocol_oteryn::{
    server_modal::{Answer, CAPABILITY_SERVER_MODAL_V1, Dialog, Reply},
    world_object::{CAPABILITY_WORLD_INTERACTION_V1, UseDisposition, UseTarget},
};

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Selection {
    HirelingStore(super::store::Selection),
    Bed(BedSelection),
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct BedSelection {
    target: ItemKey,
    revision: u64,
    fence: CurrentCharacterItemFence,
}

/// Crystal 201189f game.cpp:261-272,10912-10990: native bed skill choice.
fn bed_dialog() -> Dialog {
    Dialog {
        id: 1,
        title: "Choose a Skill".into(),
        message: "Please choose a skill for offline training:".into(),
        choices: [
            (4, "Axe"),
            (2, "Club"),
            (5, "Distance"),
            (1, "Fist"),
            (6, "Magic Level"),
            (3, "Sword"),
        ]
        .into_iter()
        .map(|(skill, name)| {
            (
                skill,
                if skill == 6 {
                    format!("{name} and Shielding")
                } else {
                    format!("{name} Fighting and Shielding")
                },
            )
        })
        .collect(),
        buttons: vec![(1, "Okay".into()), (2, "Cancel".into())],
        cancel_button: Some(2),
        image: None,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn offer_bed_modal(
        &self,
        admitted: AdmittedSession,
        target: UseTarget,
        resolved: Option<ItemKey>,
    ) -> Option<UseOutcome> {
        if ![CAPABILITY_SERVER_MODAL_V1, CAPABILITY_WORLD_INTERACTION_V1]
            .iter()
            .all(|id| admitted.continuity.selected_capabilities.contains(*id))
        {
            return None;
        }
        if matches!(&target, UseTarget::Item(_))
            && !matches!(resolved, Some(ItemKey::MapBase { .. }))
        {
            return None;
        }
        let root = self.states.server_messages.connection_root(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        )?;
        if !self.states.server_modals.accepts_terminal_reply(root) {
            return None;
        }
        let map = self.bundle_map()?;
        let context = {
            let runtime = self.runtime.lock().await;
            if !actor_is_current(&runtime, admitted) {
                return None;
            }
            crate::durability::consume_transform::HouseActionContext::from_native_owner(
                &runtime,
                admitted.runtime_actor?,
                admitted.game_session_id,
                admitted.item_fence?,
            )
            .ok()?
        };
        let changes = self
            .house_base_decoration_facts(&context, map.overlay.base().digest())
            .await?;
        let runtime = self.runtime.lock().await;
        let base = map.facts.base_only();
        let facts = crate::gameplay_transport::world_map::DecoratedFacts {
            base: &base,
            changes: &changes,
        };
        let target = match target {
            UseTarget::Item(_) => resolved?,
            UseTarget::WorldObject(target) => {
                let key = crate::gameplay_transport::world_map::resolve_map_target(
                    &crate::gameplay_transport::world_map::MapViewSource {
                        overlay: &map.overlay,
                        facts: &facts,
                        content_generation: runtime.content_pin().client_artifact_digest(),
                        reset_epoch: self.reset_epoch,
                    },
                    &target,
                    |_| None::<()>,
                )
                .ok()?;
                ItemKey::MapBase {
                    bundle_digest: map.overlay.base().digest(),
                    placement_key: key.placement_key,
                    reset_epoch: self.reset_epoch,
                }
            }
            _ => return None,
        };
        if !actor_is_current(&runtime, admitted)
            || !self.bed_modal_target(&runtime, admitted, target, &changes)
            || self.states.server_messages.connection_root(
                admitted.game_session_id,
                admitted.continuity.connection_generation,
            ) != Some(root)
        {
            return None;
        }
        let ItemKey::MapBase { placement_key, .. } = target else {
            return None;
        };
        let offered = self.states.server_modals.offer_rule(
            root,
            bed_dialog(),
            Selection::Bed(BedSelection {
                target,
                revision: facts.object_revision(placement_key),
                fence: admitted.item_fence?,
            }),
        );
        Some(UseOutcome {
            disposition: if offered.is_some() {
                UseDisposition::BedChooseSkill
            } else {
                UseDisposition::Rejected
            },
            ..UseOutcome::rejected()
        })
    }
}

pub(super) async fn answer(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    root: Root,
    command_id: u64,
    payload: &[u8],
) -> Option<Option<ServerModalOutcome>> {
    let answer = Answer::decode(payload).ok()?;
    let Selection::Bed(selection) = owner.states.server_modals.rule_selection(root, answer)? else {
        return None;
    };
    Some(complete(owner, admitted, root, command_id, answer, selection).await)
}

async fn complete(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    root: Root,
    command_id: u64,
    answer: Answer,
    selection: BedSelection,
) -> Option<ServerModalOutcome> {
    if answer.button_id == 2 {
        return owner
            .states
            .server_modals
            .finish_rule(root, answer)
            .map(ServerModalOutcome::Reply);
    }
    if admitted.item_fence != Some(selection.fence)
        || !owner.states.server_modals.accepts_terminal_reply(root)
    {
        owner.states.server_modals.refused_rule(root, answer)?;
        return None;
    }
    let actor = admitted.runtime_actor?;
    let acknowledgement = Reply::AnsweredAndLoggedOut(answer).encode().ok()?;
    // The durable owner future is deliberately kept off the connection's stack,
    // as on the existing command-31 path.
    let outcome = Box::pin(owner.production_bed_skill(
        actor,
        UseCommand {
            game_session_id: admitted.game_session_id,
            command_id,
            item_fence: admitted.item_fence,
        },
        answer.choice_id?,
        selection.target,
        Some(selection.revision),
    ))
    .await;
    match outcome {
        Ok(outcome) if outcome.disposition == UseDisposition::OfflineTraining => {
            // The native owner committed the final release. Send its exact modal ACK
            // and end the connection; do not attempt another current-session mutation.
            let _ = owner
                .states
                .server_modals
                .finish_terminal_rule(root, answer);
            Some(ServerModalOutcome::LoggedOut(acknowledgement))
        }
        Ok(_) => {
            owner.states.server_modals.refused_rule(root, answer)?;
            None
        }
        Err(_) => Some(ServerModalOutcome::Unavailable),
    }
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod tests;
