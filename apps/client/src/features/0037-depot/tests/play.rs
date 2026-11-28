use crate::play::play_channel;
use oteryn_client_ui::depot::{DepotPanel, Intent, request};
#[test]
fn depot_commands_require_authoritative_town_and_one_pending_request() {
    let mut p = DepotPanel {
        selected: true,
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Withdraw(0)).is_none());
    assert!(
        request(
            &mut p,
            Intent::Browse {
                box_number: 1,
                page: 0
            }
        )
        .is_some()
    );
    assert!(
        request(
            &mut p,
            Intent::Browse {
                box_number: 2,
                page: 0
            }
        )
        .is_none()
    );
}
#[test]
fn depot_visible_panel_receives_authoritative_refusal() {
    let (link, _commands, outbox) = play_channel();
    outbox.depot_initialize(true, false);
    link.open_depot();
    assert!(link.depot_panel().pending);
    outbox.depot_result(None);
    assert!(!link.depot_panel().pending);
    assert!(link.depot_panel().feedback.contains("refused"));
}

use crate::play::feature_0037_depot::protocol::test_wire::Error;
// Thais and Carlin use these IDs in the authored town catalogue.
const THAIS: u16 = 8;
const CARLIN: u16 = 6;
const LOCAL: Intent = Intent::Browse {
    box_number: 1,
    page: 0,
};
const LEGACY_OPEN: &[u8] = &[8, 1, 16, 1, 24, 0, 40, 0];
const CROSS_TOWN_OPEN: &[u8] = &[8, 1, 16, 1, 24, 0, 40, 0, 32, 6, 56, 1];

#[test]
fn depot_town_selector_sends_field_7_when_627_selected() -> Result<(), Error> {
    depot_session(
        true,
        &[(Intent::SelectTown(Some(CARLIN)), CROSS_TOWN_OPEN, CARLIN)],
    )
}

#[test]
fn depot_without_town_keeps_legacy_bytes() -> Result<(), Error> {
    depot_session(true, &[(LOCAL, LEGACY_OPEN, THAIS)])?;
    depot_session(
        false,
        &[(Intent::SelectTown(Some(CARLIN)), LEGACY_OPEN, THAIS)],
    )
}

#[test]
fn cross_town_depot_round_trip_through_session() -> Result<(), Error> {
    depot_session(
        true,
        &[
            (LOCAL, LEGACY_OPEN, THAIS),
            (Intent::SelectTown(Some(CARLIN)), CROSS_TOWN_OPEN, CARLIN),
            (
                Intent::Browse {
                    box_number: 2,
                    page: 0,
                },
                &[8, 1, 16, 2, 24, 0, 40, 0, 32, 6, 56, 1],
                CARLIN,
            ),
            (
                Intent::SelectTown(None),
                &[8, 1, 16, 2, 24, 0, 40, 0],
                THAIS,
            ),
        ],
    )
}

fn depot_session(
    cross_town: bool,
    exchanges: &'static [(Intent, &'static [u8], u16)],
) -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    depot_exchanges(cross_town, exchanges),
                )
                .await?
            })
        })?
        .join()
        .map_err(|_| "depot session thread panicked")?
}

async fn depot_exchanges(
    cross_town: bool,
    exchanges: &[(Intent, &[u8], u16)],
) -> Result<(), Error> {
    use crate::play::feature_0037_depot::protocol::test_wire;
    let (session, mut peer) = test_wire::admit(cross_town).await?;
    let selected = session.selected_capabilities().to_vec();
    let (link, commands, outbox) = play_channel();
    let task = tokio::spawn(crate::play::run_session(session, commands, outbox));
    while !link.depot_panel().selected {
        tokio::task::yield_now().await;
    }
    assert_eq!(link.depot_panel().cross_town_selected, cross_town);
    for (index, &(intent, expected, town)) in exchanges.iter().enumerate() {
        link.depot_request(intent);
        let sequence = u64::try_from(index)? + 1;
        let reply = test_wire::reply(&mut peer, &selected, sequence, expected, town).await?;
        while link.depot_panel().pending {
            tokio::task::yield_now().await;
        }
        assert_eq!(link.depot_panel().reply, Some(reply));
    }
    drop(link);
    task.await?;
    Ok(())
}
