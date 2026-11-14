use super::*;
use crate::play::play_channel;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
#[test]
fn status_details_owner_rejects_modal_closed_and_duplicate_queries() -> Result {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.status_details_test_selected();
    let closed = link.status_details_panel();
    assert!(!link.status_details_dispatch_bound(
        &closed,
        StatusDetailsAction::Refresh(StatusDetailsTab::Party)
    ));
    assert_eq!(link.status_details_panel(), closed);
    assert!(link.status_details_dispatch_bound(
        &closed,
        StatusDetailsAction::Open(StatusDetailsTab::Party)
    ));
    assert!(commands.try_recv().is_ok());
    let pending = link.status_details_panel();
    assert!(!link.status_details_dispatch_bound(
        &pending,
        StatusDetailsAction::Refresh(StatusDetailsTab::Party)
    ));
    assert!(!link.status_details_dispatch_bound(
        &closed,
        StatusDetailsAction::Open(StatusDetailsTab::Party)
    ));
    assert_eq!(link.status_details_panel(), pending);
    let reply = party_effects::PartyEffects {
        in_party: false,
        shared_enabled: false,
        shared_active: false,
        eligible: false,
        bonuses: [0; 5],
        party_spell_remaining_ms: [0; 5],
    }
    .encode()
    .ok_or("encode")?;
    outbox.status_details_test_reply(StatusDetailsTab::Party, true, &reply);
    let expected = link.status_details_panel();
    let _modal = link.bevy_server_modal_fixture();
    assert!(!link.status_details_dispatch_bound(
        &expected,
        StatusDetailsAction::Refresh(StatusDetailsTab::Party)
    ));
    assert_eq!(link.status_details_panel(), expected);
    assert!(commands.try_recv().is_err());
    let (link, commands, outbox) = play_channel();
    outbox.status_details_test_selected();
    let before = link.status_details_panel();
    drop(commands);
    assert!(!link.status_details_dispatch_bound(
        &before,
        StatusDetailsAction::Open(StatusDetailsTab::Party)
    ));
    assert_eq!(link.status_details_panel(), before);
    Ok(())
}
#[test]
fn status_details_owner_retains_pending_close_and_counts_down_confirmed_potions() -> Result {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.status_details_test_selected();
    assert!(link.status_details_dispatch_bound(
        &link.status_details_panel(),
        StatusDetailsAction::Open(StatusDetailsTab::Potions)
    ));
    assert!(commands.try_recv().is_ok());
    assert!(link.status_details_dispatch_bound(
        &link.status_details_panel(),
        StatusDetailsAction::Close(StatusDetailsTab::Potions)
    ));
    let potion = potion_durations::PotionDurations {
        active: vec![potion_durations::PotionDuration {
            source: 4,
            item_id: 0,
            percent: 25,
            remaining_ms: 2000,
        }],
    }
    .encode()
    .ok_or("encode")?;
    outbox.status_details_test_reply(StatusDetailsTab::Potions, true, &potion);
    let panel = link.status_details_panel();
    assert!(panel.active().is_none());
    assert!(
        panel
            .text_at(StatusDetailsTab::Potions, std::time::Instant::now())
            .contains("Daily XP boost +25%")
    );
    assert!(
        panel
            .text_at(
                StatusDetailsTab::Potions,
                std::time::Instant::now() + std::time::Duration::from_secs(3)
            )
            .contains("No active potion effects")
    );
    outbox.status_details_test_end();
    outbox.status_details_test_reply(StatusDetailsTab::Potions, true, &potion);
    let ended = link.status_details_panel();
    assert!(ended.active().is_none());
    assert!(!link.status_details_dispatch_bound(
        &panel,
        StatusDetailsAction::Open(StatusDetailsTab::Potions)
    ));
    assert_eq!(link.status_details_panel(), ended);
    assert!(commands.try_recv().is_err());
    Ok(())
}
