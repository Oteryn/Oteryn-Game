//! Context actions use the existing session commands and codecs.
use item_look::{ItemLookIntent, ItemLookSelection, ItemLookTarget};
pub use oteryn_session::EntityRef;
pub use oteryn_session::world_context::{item_look, world_object};
use oteryn_session::{ItemHandle, Session, SessionError, SessionStream};
use world_object::WorldObjectTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Item(ItemHandle),
    Creature(oteryn_session::EntityRef),
    Object(WorldObjectTarget),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Look,
    Use,
    Open,
    BedInteract,
    Bed(u8),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub target: Target,
    pub action: Action,
}

pub fn look_payload(target: &Target) -> Result<Vec<u8>, SessionError> {
    let target = match target {
        Target::Item(handle) => ItemLookTarget::Item(*handle),
        Target::Creature(entity) => ItemLookTarget::Creature(*entity),
        Target::Object(object) => ItemLookTarget::WorldObject(object.clone()),
    };
    item_look::encode_item_look_intent_for(
        ItemLookIntent { target },
        ItemLookSelection { extended: true },
    )
    .map_err(|_| SessionError::SessionUnusable)
}

pub fn bed_payload(handle: ItemHandle, skill: u8) -> Result<Vec<u8>, SessionError> {
    if !(1..=6).contains(&skill) {
        return Err(SessionError::SessionUnusable);
    }
    // Existing BedSleepIntentV1: nested ItemTargetV1 followed by training_skill.
    let target = oteryn_session::item_view::encode_item_target(handle);
    let mut payload = vec![
        10,
        u8::try_from(target.len()).map_err(|_| SessionError::SessionUnusable)?,
    ];
    payload.extend(target);
    payload.extend([16, skill]);
    Ok(payload)
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &Command,
) -> Result<String, SessionError> {
    if command.action == Action::Look {
        let result = session
            .gated_command(28, 30, &look_payload(&command.target)?)
            .await?;
        if !matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        ) {
            return Ok("Look refused".into());
        }
        return item_look::decode_item_look_result_for(
            &result.payload,
            ItemLookSelection { extended: true },
        )
        .map(|result| match result {
            item_look::ItemLookResult::Text(text) => text,
            item_look::ItemLookResult::Refused(reason) => format!("Look: {reason:?}"),
        })
        .map_err(|_| SessionError::SessionUnusable);
    }
    let disposition = match (&command.target, command.action) {
        (Target::Item(handle), Action::Use | Action::Open | Action::BedInteract) => {
            session.use_item(*handle).await?.disposition
        }
        (Target::Object(object), Action::Use | Action::Open | Action::BedInteract) => {
            session
                .use_object(&object.placement, object.expected_revision)
                .await?
                .disposition
        }
        (Target::Item(handle), Action::Bed(skill @ 1..=6)) => {
            let payload = bed_payload(*handle, skill)?;
            let result = session.gated_command(11, 31, &payload).await?;
            if result.payload.is_empty() {
                return Ok("Bed request refused".into());
            }
            world_object::decode_use_result_with_gate(&result.payload)
                .map_err(|_| SessionError::SessionUnusable)?
                .0
        }
        _ => return Ok("Action unavailable".into()),
    };
    use world_object::UseDisposition;
    Ok(match disposition {
        UseDisposition::Committed => "Interaction completed",
        UseDisposition::BedChooseSkill if !matches!(command.target, Target::Item(_)) => {
            "Training selection is unavailable for this bed"
        }
        UseDisposition::BedChooseSkill => "Choose a training skill from the Bed menu",
        UseDisposition::BedOccupied => "This bed is occupied",
        UseDisposition::BedWoken => "You woke the sleeper",
        UseDisposition::OfflineTraining => "Offline training started",
        UseDisposition::TooFar => "Move closer to interact",
        UseDisposition::StaleState => "The target changed; try again",
        UseDisposition::NothingToUse => "There is nothing to use here",
        _ => "Interaction refused",
    }
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bed_payload_keeps_handle_and_all_six_existing_skills() -> Result<(), SessionError> {
        for skill in 1..=6 {
            let intent = oteryn_session::world_context::bed::decode_bed_sleep_intent(&bed_payload(
                ItemHandle::MAX,
                skill,
            )?)
            .map_err(|_| SessionError::SessionUnusable)?;
            assert_eq!((intent.target, intent.skill), (ItemHandle::MAX, skill));
        }
        assert!(bed_payload(ItemHandle::MIN, 0).is_err());
        assert!(bed_payload(ItemHandle::MIN, 7).is_err());
        Ok(())
    }
    #[test]
    fn look_keeps_native_creature_identity_and_object_revision() -> Result<(), SessionError> {
        for target in [
            Target::Item(ItemHandle::MIN),
            Target::Creature(oteryn_session::EntityRef {
                identity: [9; 16],
                generation: 7,
            }),
            Target::Object(WorldObjectTarget {
                placement: vec![1; 40],
                expected_revision: 42,
            }),
        ] {
            let decoded = item_look::decode_item_look_intent_for(
                &look_payload(&target)?,
                ItemLookSelection { extended: true },
            )
            .map_err(|_| SessionError::SessionUnusable)?;
            assert_eq!(
                decoded.target,
                match target {
                    Target::Item(handle) => ItemLookTarget::Item(handle),
                    Target::Creature(entity) => ItemLookTarget::Creature(entity),
                    Target::Object(object) => ItemLookTarget::WorldObject(object),
                }
            );
        }
        Ok(())
    }
}
