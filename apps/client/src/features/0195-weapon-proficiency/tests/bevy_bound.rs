use super::*;
use crate::play::{mailbox::lock, play_channel};
use oteryn_client_runtime::weapon_proficiency::{
    ProficiencyCatalogueEntry, ProficiencyCatalogueLevel, ProficiencyCataloguePage,
};
use oteryn_client_runtime::weapon_proficiency_state::{Slot, State};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn observed() -> Panel {
    Panel {
        selected: true,
        state_selected: true,
        open: true,
        weapon: 258,
        slot: 1,
        revision: 7,
        catalogue: Some(ProficiencyCataloguePage {
            offset: 0,
            total: 1,
            entries: vec![ProficiencyCatalogueEntry {
                definition: 258,
                weapon: "server.sword".into(),
                name: "Server sword".into(),
                experience: 30,
                thresholds: [10, 20, 30, 40, 50, 60, 70, 80, 90],
                levels: vec![ProficiencyCatalogueLevel {
                    selected: Some(0),
                    perks: vec!["Server perk".into()],
                }],
            }],
        }),
        shaping: Some(State {
            definition: 258,
            revision: 7,
            slots: [
                Some(Slot {
                    level: 0,
                    rank: 1,
                    current: "Current".into(),
                    next: "Next".into(),
                    maximum: "Maximum".into(),
                    offers: vec!["Offer 1".into(), "Offer 2".into(), "Offer 3".into()],
                }),
                None,
            ],
            modify_dust: [250, 1000],
            rank_dust: [25; 10],
            reshape_dust: 100,
            clear_dust: 10,
            orbs: 1,
        }),
        ..Default::default()
    }
}
#[test]
fn weapon_proficiency_all_default_actions_keep_the_default_requests() -> TestResult {
    for id in 101..=108 {
        let (link, mut commands, outbox) = play_channel();
        link.use_shared_frontend();
        let panel = observed();
        lock(&outbox.0).weapon_proficiency = panel.clone();
        let (session, expected, _) = link.weapon_proficiency_snapshot();
        assert!(link.weapon_proficiency_dispatch_bound(session, &expected, Action::Confirmed(id)));
        let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("command".into());
        };
        let mut default = panel.clone();
        let expected = if id == 108 {
            assert!(native::visual_submit(&mut default, 108, false).is_none());
            native::visual_submit(&mut default, 117, false)
        } else {
            native::visual_submit(&mut default, id, false)
        }
        .ok_or("default command")?;
        assert_eq!(command, expected, "action {id}");
        assert_eq!(link.weapon_proficiency_panel().catalogue, panel.catalogue);
        assert_eq!(link.weapon_proficiency_panel().shaping, panel.shaping);
        let pending = link.weapon_proficiency_panel();
        assert!(!link.weapon_proficiency_dispatch_bound(session, &pending, Action::Confirmed(id)));
        assert_eq!(link.weapon_proficiency_panel(), pending);
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn weapon_proficiency_owner_rejects_closed_stale_target_and_unavailable_without_changes()
-> TestResult {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    lock(&outbox.0).weapon_proficiency = observed();
    let (session, expected, _) = link.weapon_proficiency_snapshot();
    assert!(link.weapon_proficiency_dispatch_bound(session, &expected, Action::Choose(3, 1)));
    let changed = link.weapon_proficiency_panel();
    assert!(!link.weapon_proficiency_dispatch_bound(session, &expected, Action::Confirmed(102)));
    assert_eq!(link.weapon_proficiency_panel(), changed);
    let (replacement, _receiver, replacement_outbox) = play_channel();
    replacement.use_shared_frontend();
    lock(&replacement_outbox.0).weapon_proficiency = expected.clone();
    assert!(!replacement.weapon_proficiency_dispatch_bound(
        session,
        &expected,
        Action::Confirmed(102)
    ));
    assert_eq!(replacement.weapon_proficiency_panel(), expected);
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.weapon_proficiency_dispatch_bound(session, &changed, Action::Confirmed(102)));
    assert_eq!(link.weapon_proficiency_panel(), changed);
    drop(modal);
    assert!(link.weapon_proficiency_dispatch_bound(session, &changed, Action::Close));
    let closed = link.weapon_proficiency_panel();
    assert!(!link.weapon_proficiency_dispatch_bound(session, &closed, Action::Confirmed(102)));
    assert_eq!(link.weapon_proficiency_panel(), closed);
    outbox.bevy_weapon_proficiency_end();
    let ended = link.weapon_proficiency_panel();
    assert!(!link.weapon_proficiency_dispatch_bound(session, &expected, Action::Confirmed(102)));
    assert_eq!(link.weapon_proficiency_panel(), ended);
    assert!(commands.try_recv().is_err());
    Ok(())
}
