use super::*;

#[test]
fn gap_0602_managed_container_move() -> Result {
    use crate::bevy_shell::game_ui::stash::tests::wire;
    use oteryn_protocol_oteryn::container_batch::{Reply, Request};

    let (mut peer, link) = wire::Peer::new(Some(602))?;
    let mut app = bulk_app(link)?;
    wire::wait(&mut app, |link| !link.stash_panel().carried.is_empty())?;
    click(&mut app, "inventory.bulk.open")?;
    app.update();
    click(&mut app, "inventory.bulk.managed")?;
    assert_eq!(
        Request::decode(&peer.command(6365)?),
        Some(Request::Preview {
            source: 10.try_into()?,
            recursive: true,
        })
    );
    let quote = peer.result(
        oteryn_session::CommandStatus::Accepted,
        &Reply::Preview(1).encode(),
    )?;
    wire::wait_text(&mut app, "Move 1 stack")?;
    click(&mut app, "inventory.bulk.confirm")?;
    assert_eq!(
        Request::decode(&peer.command(6365)?),
        Some(Request::Confirm {
            preview: quote.try_into()?
        })
    );
    peer.result(
        oteryn_session::CommandStatus::Accepted,
        &Reply::Committed(1).encode(),
    )?;
    peer.clear_container()?;
    wire::wait(&mut app, |link| {
        link.container_panel()
            .views
            .is_some_and(|views| views.views[0].entries.is_empty())
    })?;
    wire::wait_text(&mut app, "Moved 1 stack")?;
    Ok(())
}
