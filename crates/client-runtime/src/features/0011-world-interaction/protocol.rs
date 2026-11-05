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
pub struct Selection {
    pub position: oteryn_session::ActorPosition,
    pub binding: Option<([u8; 32], [u8; 32], u64)>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub target: Target,
    pub action: Action,
    pub selection: Option<Selection>,
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

#[path = "state.rs"]
mod state;

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &Command,
) -> Result<String, SessionError> {
    // The session task continues receiving while the native popup owns the UI loop.
    // Drain already-readable pushes before consulting the session's current stores.
    session
        .service_liveness(std::time::Duration::from_millis(1))
        .await?;
    if !state::current(session, command) {
        return Ok("Target changed".into());
    }
    let label = match command.action {
        Action::Look => "Look",
        Action::Use => "Use",
        Action::Open => "Open",
        Action::BedInteract | Action::Bed(_) => "Bed",
    };
    let result = match (&command.target, command.action) {
        (_, Action::Look) => {
            let Ok(payload) = look_payload(&command.target) else {
                return Ok("Look unavailable".into());
            };
            session.gated_command(28, 30, &payload).await?
        }
        (Target::Item(handle), Action::Use | Action::Open | Action::BedInteract) => {
            if !session.is_selected(4) {
                return Ok(format!("{label} unavailable"));
            }
            session
                .world_context_use(&world_object::encode_use_item_intent(*handle))
                .await?
        }
        (Target::Object(object), Action::Use | Action::Open | Action::BedInteract) => {
            let Ok(payload) = world_object::encode_use_intent(object) else {
                return Ok(format!("{label} unavailable"));
            };
            session.world_context_use(&payload).await?
        }
        (Target::Item(handle), Action::Bed(skill @ 1..=6)) => {
            let Ok(payload) = bed_payload(*handle, skill) else {
                return Ok("Bed unavailable".into());
            };
            session.gated_command(11, 31, &payload).await?
        }
        _ => return Ok(format!("{label} unavailable")),
    };
    if !matches!(
        result.status,
        oteryn_session::CommandStatus::Accepted | oteryn_session::CommandStatus::DuplicateReplay
    ) {
        return Ok(format!("{label} refused"));
    }
    if command.action == Action::Look {
        return Ok(
            match item_look::decode_item_look_result_for(
                &result.payload,
                ItemLookSelection { extended: true },
            ) {
                Ok(item_look::ItemLookResult::Text(text)) => text,
                Ok(item_look::ItemLookResult::Refused(_)) => "Look refused".into(),
                Err(_) => "Look: unknown result".into(),
            },
        );
    }
    use world_object::UseDisposition;
    let Ok((disposition, _)) = world_object::decode_use_result_with_gate(&result.payload) else {
        return Ok(format!("{label}: unknown result"));
    };
    let text = match disposition {
        UseDisposition::Committed => "completed",
        UseDisposition::BedChooseSkill => "training selection unavailable for this bed",
        UseDisposition::BedOccupied => "this bed is occupied",
        UseDisposition::BedWoken => "sleeper woke",
        UseDisposition::OfflineTraining => "offline training started",
        UseDisposition::TooFar => "move closer",
        UseDisposition::StaleState => "target changed",
        UseDisposition::NothingToUse => "nothing to use here",
        _ => "refused",
    };
    Ok(format!("{label}: {text}"))
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
