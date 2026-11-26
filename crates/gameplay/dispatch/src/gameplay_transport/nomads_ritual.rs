//! The source drop and pour gestures enter the common fenced World callback owner.
use super::{ComposedFreshAdmission, connection::UseCommand};
use crate::durability::nomads_ritual::{NomadsRitualGesture, NomadsRitualTarget};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::map::boot::BundleMap;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Position, nomads_ritual};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) fn accepts_callback(key: &str, ritual: Option<&str>) -> bool {
    match ritual {
        Some(gesture) => key == gesture,
        None => !matches!(
            key,
            nomads_ritual::PLACE_SCIMITAR | nomads_ritual::OPEN_ENTRANCE
        ),
    }
}

fn target_key(position: Position) -> Option<u64> {
    oteryn_world_bundle::bundle::placement_key(position.floor, position.x, position.y, 1)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prepare_nomads_ritual(
        &self,
        map: &BundleMap,
        gesture: NomadsRitualGesture,
    ) -> Option<NomadsRitualTarget> {
        let (position, callback_key) = effect(gesture);
        let key = target_key(position)?;
        let callback = self
            .callback_content()
            .iter()
            .find(|row| row.key == callback_key)?;
        (super::quest_callbacks::matches(map, callback, key)
            && super::quest_callbacks::world_requirements_match(map, callback))
        .then_some(NomadsRitualTarget {
            map_digest: map.overlay.base().digest(),
            reset_epoch: self.reset_epoch,
            object_revision: map.facts.object_revision(key),
        })
    }

    /// Reconcile the exact physical command before resolving the current item
    /// view: the scimitar has moved and the flask is already empty.
    pub(super) async fn resume_nomads_ritual(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        gesture: NomadsRitualGesture,
    ) -> Result<bool, ()> {
        let fence = command.item_fence.ok_or(())?;
        let reference = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let budget = super::AdmissionRetryBudget::for_command(self.root);
        let Some(receipt) = budget
            .retry_command_guard(async || {
                self.root
                    .nomads_ritual_gesture(
                        self.character,
                        self.holder,
                        fence,
                        reference,
                        gesture,
                        false,
                    )
                    .await
            })
            .await
            .map_err(|_| ())?
        else {
            return Ok(false);
        };
        if receipt.settled {
            return Ok(true);
        }
        let map = self.bundle_map().ok_or(())?;
        let (position, callback) = effect(gesture);
        let key = target_key(position).ok_or(())?;
        // A rebuilt map starts at revision zero. A revised live target has
        // already published this gesture or superseded its original placement.
        let revision = map.facts.object_revision(key);
        if receipt.target.reset_epoch == self.reset_epoch
            && receipt.target.map_digest == map.overlay.base().digest()
            && (revision == 0 || revision == receipt.target.object_revision)
        {
            let mut placement = receipt.target.map_digest.to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let target = WorldObjectTarget {
                placement,
                expected_revision: revision,
            };
            if let Some(outcome) = Box::pin(self.use_quest_callback_from(
                actor,
                command,
                map,
                &target,
                None,
                Some(callback),
            ))
            .await
                && !matches!(
                    outcome.disposition,
                    UseDisposition::Committed | UseDisposition::NothingToUse
                )
            {
                return Err(());
            }
        }
        budget
            .retry_command_guard(async || {
                self.root
                    .nomads_ritual_gesture(
                        self.character,
                        self.holder,
                        fence,
                        reference,
                        gesture,
                        true,
                    )
                    .await
            })
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        Ok(true)
    }
}

fn effect(gesture: NomadsRitualGesture) -> (Position, &'static str) {
    match gesture {
        NomadsRitualGesture::Drop => (nomads_ritual::CRACK, nomads_ritual::PLACE_SCIMITAR),
        NomadsRitualGesture::Pour => (nomads_ritual::ENTRANCE, nomads_ritual::OPEN_ENTRANCE),
    }
}

#[cfg(test)]
#[test]
fn nomads_ritual_accepts_only_the_matching_native_gesture() {
    let place = nomads_ritual::PLACE_SCIMITAR;
    let open = nomads_ritual::OPEN_ENTRANCE;
    assert!(!accepts_callback(place, None));
    assert!(!accepts_callback(open, None));
    assert!(accepts_callback(place, Some(place)));
    assert!(accepts_callback(open, Some(open)));
    assert!(!accepts_callback(open, Some(place)));
    assert!(!accepts_callback(place, Some(open)));
    assert!(accepts_callback("ordinary-use", None));
}

#[cfg(test)]
thread_local! {
    static INTERRUPT_AFTER_COMMIT: std::cell::Cell<Option<(crate::foundation::GameSessionId, u64)>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn fixture_interrupt_nomads_ritual(
        &self,
        command: UseCommand,
    ) -> Result<(), &'static str> {
        INTERRUPT_AFTER_COMMIT.with(|fault| {
            if fault.get().is_some() {
                return Err("ritual fault already armed");
            }
            fault.set(Some((command.game_session_id, command.command_id)));
            Ok(())
        })
    }
}

#[cfg(test)]
pub(super) fn interrupted_after_commit(command: UseCommand) -> bool {
    INTERRUPT_AFTER_COMMIT.with(|fault| {
        if fault.get() == Some((command.game_session_id, command.command_id)) {
            fault.set(None);
            true
        } else {
            false
        }
    })
}
