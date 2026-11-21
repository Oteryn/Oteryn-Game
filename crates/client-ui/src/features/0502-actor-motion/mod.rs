pub use oteryn_client_domain::movement::StepDir;
use oteryn_client_runtime::raw_feature::{FeatureCommand, actor_motion};
use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};

pub fn forecast(
    entity: &oteryn_client_runtime::WorldSpatialEntity,
    direction: StepDir,
) -> Option<([i32; 2], u32)> {
    use oteryn_client_domain::movement::StepDir::*;
    let (index, dx, dy) = match direction {
        North => (0, 0, -1),
        East => (1, 1, 0),
        South => (2, 0, 1),
        West => (3, -1, 0),
        NorthEast => (4, 1, -1),
        SouthEast => (5, 1, 1),
        SouthWest => (6, -1, 1),
        NorthWest => (7, -1, -1),
    };
    let duration = entity.motion?.prewalk_ms[index];
    (duration != 0).then_some((
        [
            entity.position.x.checked_add(dx)?,
            entity.position.y.checked_add(dy)?,
        ],
        duration,
    ))
}

pub fn walking_phase(phases: u32, elapsed_ms: u64, step_duration_ms: u32) -> u32 {
    let duration = u64::from(step_duration_ms.max(1));
    ((elapsed_ms % duration) * u64::from(phases) / duration) as u32
}

pub fn mount_command(selected: bool, riding: bool) -> Option<FeatureCommand> {
    selected.then(|| FeatureCommand {
        capability: actor_motion::CAPABILITY_ACTOR_MOTION_V1,
        kind: actor_motion::COMMAND_TYPE_MOUNT,
        payload: actor_motion::encode_mount(riding),
    })
}

pub fn mount_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event,
        NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false }
        if code.get() == 21 && *modifiers == Modifiers::one(Modifier::Control)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mount_chord_is_control_r() -> Result<(), Box<dyn std::error::Error>> {
        let event = NormalizedInputEvent::Key {
            code: oteryn_input_actions::KeyCode::new(21)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control),
            repeat: false,
        };
        assert!(mount_key(&[event]));
        assert!(mount_command(false, true).is_none());
        assert_eq!(mount_command(true, true).ok_or("command")?.payload, [8, 1]);
        Ok(())
    }
}
