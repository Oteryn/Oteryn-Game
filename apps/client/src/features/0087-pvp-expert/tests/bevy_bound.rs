use super::ExpertPvpModesAction as Action;
use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_runtime::pvp_expert as wire;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
#[test]
fn expert_pvp_modes_owner_rejects_stale_closed_duplicate_and_modal() -> Result {
    let (link, mut incoming, outbox) = play_channel();
    outbox.expert_pvp_modes_test_state(oteryn_client_ui::pvp_expert::Panel {
        selected: true,
        open: true,
        ..Default::default()
    });
    outbox.expert_pvp_modes_test_push(wire::ExpertFightModes::default());
    let snapshot = link.expert_pvp_modes_snapshot();
    let before = link.pvp_expert_panel();
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.expert_pvp_modes_dispatch_bound(&snapshot, Action::Apply));
    assert_eq!(link.pvp_expert_panel(), before);
    assert!(incoming.try_recv().is_err());
    drop(modal);
    assert!(link.expert_pvp_modes_dispatch_bound(&snapshot, Action::Field(3, 3)));
    let changed = link.pvp_expert_panel();
    assert!(!link.expert_pvp_modes_dispatch_bound(&snapshot, Action::Apply));
    assert_eq!(link.pvp_expert_panel(), changed);
    let snapshot = link.expert_pvp_modes_snapshot();
    assert!(!link.expert_pvp_modes_dispatch_bound(&snapshot, Action::Field(4, 0)));
    assert!(!link.expert_pvp_modes_dispatch_bound(&snapshot, Action::Field(3, 4)));
    assert_eq!(link.pvp_expert_panel(), changed);
    assert!(link.expert_pvp_modes_dispatch_bound(&snapshot, Action::Apply));
    let pending = link.pvp_expert_panel();
    assert!(!link.expert_pvp_modes_dispatch_bound(&snapshot, Action::Apply));
    assert!(
        !link.expert_pvp_modes_dispatch_bound(&link.expert_pvp_modes_snapshot(), Action::Apply)
    );
    assert_eq!(link.pvp_expert_panel(), pending);
    assert!(matches!(incoming.try_recv()?, PlayCommand::Feature(_)));
    assert!(incoming.try_recv().is_err());
    assert!(link.expert_pvp_modes_dispatch_bound(&link.expert_pvp_modes_snapshot(), Action::Close));
    let closed = link.pvp_expert_panel();
    assert!(
        !link.expert_pvp_modes_dispatch_bound(&link.expert_pvp_modes_snapshot(), Action::Apply)
    );
    assert_eq!(link.pvp_expert_panel(), closed);
    drop(incoming);
    assert!(!link.expert_pvp_modes_dispatch_bound(&link.expert_pvp_modes_snapshot(), Action::Open));
    assert_eq!(link.pvp_expert_panel(), closed);
    Ok(())
}
