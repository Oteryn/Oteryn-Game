use super::*;
use crate::{combat_input::PlayCommand, play::mailbox::lock};
#[test]
fn returner_real_caller_requires_server_offer_and_confirms_claim() -> Result<(), Error> {
    use daily_reward::returner::{CLAIM, QUERY, Reply};
    let (link, mut incoming, outbox) = play_channel();
    outbox.daily_reward_initialize(true);
    lock(&link.mailbox).daily_reward.wall = Some(daily_reward::DailyRewardWall {
        lane_day: 30,
        streak: 10000,
        jokers: 3,
        claimed: false,
        claim_enabled: true,
    });
    link.returner_reward_request(true);
    assert!(incoming.try_recv().is_err());
    link.returner_reward_request(false);
    assert!(
        matches!(incoming.try_recv(),Ok(PlayCommand::Feature(c)) if c.capability==182 && c.kind==QUERY && c.payload.is_empty())
    );
    let id = [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1];
    outbox.daily_reward_returner_result(
        QUERY,
        &oteryn_session::GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: Reply {
                state: 1,
                id: Some(id),
            }
            .encode()?,
        },
    );
    assert!(
        link.daily_reward_panel()
            .returner
            .notification()
            .contains("available")
    );
    link.returner_reward_request(true);
    assert!(
        matches!(incoming.try_recv(),Ok(PlayCommand::Feature(c)) if c.kind==CLAIM && c.payload==id)
    );
    assert!(
        !link
            .daily_reward_panel()
            .returner
            .notification()
            .contains("Claimed")
    );
    outbox.daily_reward_returner_result(
        CLAIM,
        &oteryn_session::GatedCommandResult {
            command_id: 2,
            status: CommandStatus::Accepted,
            server_sequence: 2,
            payload: daily_reward::DailyRewardResult {
                disposition: 1,
                transaction_id: Some(id),
            }
            .encode()
            .map_err(|_| "result")?,
        },
    );
    assert!(
        link.daily_reward_panel()
            .returner
            .notification()
            .contains("Claimed")
    );
    assert!(
        !link.daily_reward_panel().wall.ok_or("daily wall")?.claimed,
        "returner claim must not consume the ordinary daily claim"
    );
    link.returner_reward_request(true);
    assert!(incoming.try_recv().is_err());
    Ok(())
}

#[tokio::test]
async fn play_entry_queries_returner_entitlement_without_opening_daily_wall() -> Result<(), Error> {
    let id = [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2];
    let (link, incoming, outbox) = play_channel();
    let peer = Peer {
        commands: Arc::new(Mutex::new(vec![])),
        claimed: false,
        returner_offer: Some(id),
        activity_scope: None,
    };
    let task = tokio::spawn(run_session(peer, incoming, outbox));
    tokio::time::timeout(Duration::from_secs(2), async {
        while link
            .daily_reward_panel()
            .returner
            .offer
            .as_ref()
            .map(|o| o.id)
            != Some(id)
        {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    let panel = link.daily_reward_panel();
    assert!(!panel.open);
    assert_eq!(panel.returner_notified, Some(id));
    drop(link);
    task.await?;
    Ok(())
}

#[tokio::test]
async fn review_recovered_play_entry_queries_returner_after_scope_presentation() -> Result<(), Error>
{
    use oteryn_session::activity_scope::{
        ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
    };
    let id = |value| {
        let mut id = [value; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    };
    let session = id(3);
    let offer = id(6);
    let scope = encode_activity_scope(&ActivityScopeSnapshot {
        handoff: id(1),
        source_session: id(2),
        session,
        scope: ActivityScope::Channel {
            world: id(4),
            channel: id(5),
        },
        scope_generation: 1,
        connection_generation: 1,
        lease_generation: 1,
        selection: vec![117, 182],
        requires_ack: false,
    })
    .map_err(|_| "invalid recovery receipt")?;
    let (link, incoming, outbox) = play_channel();
    let mut task = tokio::spawn(run_session(
        Peer {
            commands: Arc::new(Mutex::new(vec![])),
            claimed: false,
            returner_offer: Some(offer),
            activity_scope: Some(scope),
        },
        incoming,
        outbox,
    ));
    tokio::time::timeout(Duration::from_secs(2), async {
        while !crate::play::mailbox::lock(&link.mailbox).scope_pending {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    link.scope_presented(session);
    tokio::time::timeout(Duration::from_secs(2), async {
        while link.daily_reward_panel().returner_notified != Some(offer) {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    drop(link);
    match tokio::time::timeout(Duration::from_secs(2), &mut task).await {
        Ok(result) => result?,
        Err(_) => {
            task.abort();
            return Err("Returner session shutdown timed out".into());
        }
    }
    Ok(())
}
