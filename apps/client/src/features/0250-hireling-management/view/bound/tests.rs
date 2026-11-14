use super::*;
use crate::play::{Outbox, play_channel};
use oteryn_client_runtime::cui3::store;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    PlayLink,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn entry(byte: u8) -> ui::wire::Entry {
    let mut id = [byte; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    ui::wire::Entry {
        hireling: id,
        name: "Alice".into(),
        sex: 2,
        look_type: 1107,
        colors: [1, 2, 3, 4],
        skills: 0,
        outfits: 1,
        position: Some((7, 9, -1)),
    }
}
fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.hireling_management_initialize(true);
    link.hireling_management_open();
    let _ = commands.try_recv()?;
    outbox.bevy_hireling_management_reply(ui::wire::Reply::State(vec![entry(1), entry(2)]));
    let snapshot = link.hireling_management_snapshot();
    assert!(link.hireling_management_dispatch_bound(&snapshot, HirelingAction::Select(0)));
    {
        let mut m = lock(&link.mailbox);
        m.store.selected = true;
        m.store.catalogue_category = "houses.hirelings".into();
        m.store.catalogue = Some(store::StoreCataloguePage {
            catalog_revision: "r1".into(),
            total_coins: 100,
            transferable_coins: 100,
            next_page_cursor: String::new(),
            offers: vec![
                ("store:hireling:25438", "Rename"),
                ("store:hireling:25437", "Change sex"),
                ("store:hireling:1001", "Banker"),
                ("store:hireling:1002", "Cook"),
                ("store:hireling:1003", "Steward"),
                ("store:hireling:1004", "Trader"),
            ]
            .into_iter()
            .map(|(key, title)| store::StoreOffer {
                offer_key: key.into(),
                title: title.into(),
                price: 5,
                currency: "coins".into(),
            })
            .collect(),
        });
    }
    Ok((link, commands, outbox))
}
#[test]
fn hireling_management_paid_actions_bind_the_exact_identity_and_price() -> Result {
    for action in 131..=136 {
        let (link, mut commands, _) = fixture()?;
        let snapshot = link.hireling_management_snapshot();
        let review = link.hireling_management_review_bound(&snapshot, action, "Bella")?;
        assert!(review.text().contains("5 coins"));
        assert!(commands.try_recv().is_err());
        assert!(link.hireling_management_dispatch_bound(
            &snapshot,
            HirelingAction::Configure(Box::new(review.clone()))
        ));
        assert!(!link.hireling_management_dispatch_bound(
            &snapshot,
            HirelingAction::Configure(Box::new(review))
        ));
        let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
            return Err("purchase missing".into());
        };
        assert_eq!((command.capability, command.command_type), (154, 185));
        let (quote, custom) = store::decode_purchase_request_with_hireling(&command.payload)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(quote.price, 5);
        assert_eq!(quote.catalog_revision, "r1");
        if action <= 132 {
            assert_eq!(
                custom.as_ref().and_then(|c| c.hireling_id),
                Some(entry(1).hireling)
            );
        } else {
            assert!(custom.is_none());
        }
        if action == 131 {
            assert_eq!(custom.and_then(|c| c.name), Some("Bella".into()));
        }
        assert_eq!(
            link.hireling_management_panel().entries,
            snapshot.panel.entries
        );
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn hireling_management_bound_owner_rejects_replaced_session_target_quote_and_closed_window()
-> Result {
    let (link, mut commands, _) = fixture()?;
    let expected = link.hireling_management_snapshot();
    let review = link.hireling_management_review_bound(&expected, 131, "Bella")?;
    let (replacement, mut replacement_commands, _) = fixture()?;
    let before = replacement.hireling_management_snapshot();
    assert!(!replacement.hireling_management_dispatch_bound(&expected, HirelingAction::Refresh));
    assert_eq!(before, replacement.hireling_management_snapshot());
    assert!(replacement_commands.try_recv().is_err());
    assert!(link.hireling_management_dispatch_bound(&expected, HirelingAction::Select(1)));
    let changed = link.hireling_management_snapshot();
    assert!(!link.hireling_management_dispatch_bound(&expected, HirelingAction::Return));
    assert!(
        !link.hireling_management_dispatch_bound(
            &changed,
            HirelingAction::Configure(Box::new(review))
        )
    );
    assert_eq!(changed, link.hireling_management_snapshot());
    let review = link.hireling_management_review_bound(&changed, 131, "Bella")?;
    lock(&link.mailbox)
        .store
        .catalogue
        .as_mut()
        .ok_or("catalogue")?
        .offers[0]
        .price = 6;
    let repriced = link.hireling_management_snapshot();
    assert!(!link.hireling_management_dispatch_bound(
        &repriced,
        HirelingAction::Configure(Box::new(review))
    ));
    assert_eq!(repriced, link.hireling_management_snapshot());
    assert!(link.hireling_management_dispatch_bound(&repriced, HirelingAction::Close));
    let closed = link.hireling_management_snapshot();
    assert!(!link.hireling_management_dispatch_bound(&closed, HirelingAction::Refresh));
    assert_eq!(closed, link.hireling_management_snapshot());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn hireling_management_refresh_and_prices_use_the_existing_dispatcher_once() -> Result {
    let (link, mut commands, outbox) = fixture()?;
    let snapshot = link.hireling_management_snapshot();
    assert!(link.hireling_management_dispatch_bound(&snapshot, HirelingAction::Prices));
    assert!(!link.hireling_management_dispatch_bound(&snapshot, HirelingAction::Prices));
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("prices missing".into());
    };
    assert_eq!((command.capability, command.command_type), (154, 183));
    let request =
        store::decode_catalogue_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(request.category, "houses.hirelings");
    assert!(commands.try_recv().is_err());
    // A new fixture keeps the outstanding Store request bound to its original session.
    let (other, mut other_commands, _) = fixture()?;
    let snapshot = other.hireling_management_snapshot();
    assert!(other.hireling_management_dispatch_bound(&snapshot, HirelingAction::Refresh));
    assert!(!other.hireling_management_dispatch_bound(&snapshot, HirelingAction::Refresh));
    let PlayCommand::Feature(command) = other_commands.try_recv()? else {
        return Err("refresh missing".into());
    };
    assert_eq!(
        ui::wire::Request::decode(&command.payload),
        Some(ui::wire::Request::List)
    );
    outbox.bevy_hireling_management_reply(ui::wire::Reply::State(vec![]));
    assert_eq!(
        other.hireling_management_panel().entries,
        snapshot.panel.entries
    );
    assert!(other_commands.try_recv().is_err());
    Ok(())
}

#[test]
fn hireling_management_ended_session_and_closed_transport_reject_without_mutation() -> Result {
    let (link, mut commands, outbox) = fixture()?;
    let before = link.hireling_management_snapshot();
    outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    let ended = link.hireling_management_snapshot();
    assert!(!ended.live);
    assert!(!link.hireling_management_dispatch_bound(&before, HirelingAction::Return));
    assert_eq!(ended, link.hireling_management_snapshot());
    assert!(commands.try_recv().is_err());
    let (link, commands, _) = fixture()?;
    let before = link.hireling_management_snapshot();
    drop(commands);
    let closed = link.hireling_management_snapshot();
    assert!(!link.hireling_management_dispatch_bound(&before, HirelingAction::Refresh));
    assert_eq!(closed, link.hireling_management_snapshot());
    Ok(())
}

#[test]
fn hireling_management_retry_consent_is_single_use_and_binds_the_original_receipt() -> Result {
    for action in 131..=136 {
        let (link, mut commands, _) = fixture()?;
        let snapshot = link.hireling_management_snapshot();
        let review = link.hireling_management_review_bound(&snapshot, action, "Bella")?;
        assert!(link.hireling_management_dispatch_bound(
            &snapshot,
            HirelingAction::Configure(Box::new(review))
        ));
        let PlayCommand::ServerMechanic(original) = commands.try_recv()? else {
            return Err("purchase missing".into());
        };
        let pending = link.hireling_management_snapshot();
        let review = link.hireling_management_retry_review_bound(&pending)?;
        assert!(review.text().contains("5 coins"));
        assert!(link.hireling_management_dispatch_bound(
            &pending,
            HirelingAction::Retry(Box::new(review.clone()))
        ));
        let after = link.hireling_management_snapshot();
        assert!(!link.hireling_management_dispatch_bound(
            &after,
            HirelingAction::Retry(Box::new(review.clone()))
        ));
        assert_eq!(after, link.hireling_management_snapshot());
        let PlayCommand::ServerMechanic(replayed) = commands.try_recv()? else {
            return Err("retry missing".into());
        };
        assert_eq!(
            (replayed.capability, replayed.command_type, replayed.payload),
            (original.capability, original.command_type, original.payload)
        );
        assert!(commands.try_recv().is_err());
        assert_eq!(after.panel.entries, pending.panel.entries);
        // A different selected identity cannot reuse consent even with a new snapshot.
        lock(&link.mailbox).hireling_management.identity = Some(entry(2).hireling);
        let changed = link.hireling_management_snapshot();
        assert!(
            link.hireling_management_retry_review_bound(&changed)
                .is_err()
        );
        assert!(
            !link.hireling_management_dispatch_bound(
                &changed,
                HirelingAction::Retry(Box::new(review))
            )
        );
        assert_eq!(changed, link.hireling_management_snapshot());
    }
    Ok(())
}
