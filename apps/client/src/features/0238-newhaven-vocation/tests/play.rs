use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
use oteryn_protocol_oteryn::newhaven_vocation::NewhavenChoiceResult;
use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;
use oteryn_session::GatedCommandResult;

#[test]
fn portal_hint_drives_five_native_choices_once_until_authoritative_answer()
-> Result<(), &'static str> {
    for vocation in 1..=5 {
        let (link, mut commands, outbox) = play_channel();
        outbox.vocation_initialize(true);
        outbox.push(vec![SessionEvent::TutorialNotice(AppliedDelta {
            server_sequence: 1,
            base_revision: 1,
            new_revision: 2,
            value: TutorialNotice::Hint(2),
        })]);
        assert!(link.newhaven_vocation_status().is_some());
        let key = NormalizedInputEvent::Key {
            code: KeyCode::new(29 + u16::from(vocation)).map_err(|_| "key")?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        assert!(link.newhaven_vocation_input(&[key.clone()]));
        assert_eq!(
            commands.try_recv(),
            Ok(crate::combat_input::PlayCommand::NewhavenVocation(vec![
                8, vocation
            ]))
        );
        assert!(link.newhaven_vocation_input(&[key]));
        assert!(commands.try_recv().is_err());
        assert_eq!(link.newhaven_vocation_status(), Some("Choosing vocation…"));
        outbox.vocation_result(&GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 2,
            payload: NewhavenChoiceResult::Chosen.encode(),
        });
        assert_eq!(link.newhaven_vocation_status(), None);
    }
    Ok(())
}

#[test]
fn portal_hint_without_choice_capability_never_intercepts_spell_keys() {
    let (link, _commands, outbox) = play_channel();
    outbox.push(vec![SessionEvent::TutorialNotice(AppliedDelta {
        server_sequence: 1,
        base_revision: 1,
        new_revision: 2,
        value: TutorialNotice::Hint(2),
    })]);
    assert_eq!(link.newhaven_vocation_status(), None);
    assert!(!link.newhaven_vocation_input(&[]));
}
