//! Context actions use the existing session commands and codecs.
use item_look::{ItemLookIntent, ItemLookSelection, ItemLookTarget};
pub use oteryn_session::EntityRef;
pub use oteryn_session::world_context::{item_look, world_object};
use oteryn_session::{ItemHandle, Session, SessionError, SessionStream};
use world_object::WorldObjectTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Position(oteryn_session::ActorPosition),
    Item(ItemHandle),
    Creature(oteryn_session::EntityRef),
    Object(WorldObjectTarget),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    UseWith,
    UseRune,
    UseOn(ItemHandle),
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
        Target::Position(_) => return Err(SessionError::SessionUnusable),
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

pub use crate::item_use::use_with_payload;

#[path = "state.rs"]
mod state;
pub use state::item_current;

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
        Action::UseWith | Action::UseRune | Action::UseOn(_) => "Use with",
        Action::Look => "Look",
        Action::Use => "Use",
        Action::Open => "Open",
        Action::BedInteract | Action::Bed(_) => "Bed",
    };
    let result = match (&command.target, command.action) {
        (_, Action::UseOn(source)) => {
            if !session.is_selected(15)
                || matches!(command.target, Target::Position(_)) && !session.is_selected(22)
                || matches!(command.target, Target::Item(_) | Target::Object(_))
                    && !session.is_selected(11)
            {
                return Ok("Use with unavailable".into());
            }
            let Ok(payload) = use_with_payload(source, &command.target) else {
                return Ok("Use with: invalid target".into());
            };
            session.world_context_use(&payload).await?
        }
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
    if command.action != Action::Look
        && matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::Rejected
                | oteryn_session::CommandStatus::DuplicateReplay
        )
        && let Some(message) = refusal_message(&result.payload, session.is_selected(66))
    {
        return Ok(message.into());
    }
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
        UseDisposition::BedChooseSkill => "choose an offline training skill in the server dialog",
        UseDisposition::BedOccupied => "this bed is occupied",
        UseDisposition::BedWoken => "sleeper woke",
        UseDisposition::OfflineTraining => "offline training started",
        UseDisposition::TooFar => "move closer",
        UseDisposition::StaleState => "target changed",
        UseDisposition::NothingToUse => "nothing to use here",
        UseDisposition::RequirementNotMet => "requirements not met",
        UseDisposition::Exhausted => "exhausted",
        UseDisposition::Full => "already full",
        UseDisposition::NoTarget => "no target",
        UseDisposition::TargetIllegal => "illegal target or blocked line of sight",
        UseDisposition::ProtectionZone | UseDisposition::PzBlocked => "protection zone",
        UseDisposition::Locked => "locked",
        UseDisposition::KeyMismatch => "wrong key",
        UseDisposition::Sealed => "sealed",
        UseDisposition::NotMovable => "item cannot move",
        UseDisposition::NoRoom => "no room",
        UseDisposition::NotPossible => "not possible",
        UseDisposition::Occupied => "occupied",
        UseDisposition::Rejected => "refused",
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
                    Target::Position(_) => return Err(SessionError::SessionUnusable),
                    Target::Item(handle) => ItemLookTarget::Item(handle),
                    Target::Creature(entity) => ItemLookTarget::Creature(entity),
                    Target::Object(object) => ItemLookTarget::WorldObject(object),
                }
            );
        }
        Ok(())
    }
}

fn refusal_message(payload: &[u8], selected: bool) -> Option<&'static str> {
    if !selected {
        return None;
    }
    let (payload, detail) = oteryn_session::refusal_detail::split_refusal_detail(payload).ok()?;
    let (disposition, _) = world_object::decode_use_result_with_gate(payload).ok()?;
    if matches!(
        disposition,
        world_object::UseDisposition::Committed
            | world_object::UseDisposition::BedWoken
            | world_object::UseDisposition::OfflineTraining
    ) {
        return None;
    }
    detail.map(|detail| detail.reference_message())
}
#[cfg(test)]
mod refusal_tests {
    use super::*;
    #[test]
    fn use_flow_presents_every_negotiated_refusal_detail() {
        for detail in oteryn_session::refusal_detail::RefusalDetail::ALL {
            let mut payload = vec![8, 5];
            oteryn_session::refusal_detail::append_refusal_detail(
                &mut payload,
                &[66],
                Some(detail),
            );
            assert_eq!(
                refusal_message(&payload, true),
                Some(detail.reference_message())
            );
            assert_eq!(refusal_message(&payload, false), None);
        }
    }
}
