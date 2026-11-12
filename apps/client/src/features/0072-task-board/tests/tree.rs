use super::*;
use crate::combat_input::PlayCommand;
use oteryn_client_ui::task_board::{FullIntent, talisman, tree_nodes};

#[test]
fn task_tree_uses_server_quotes_and_submits_the_selected_path_and_reward() -> Result<(), Error> {
    let (link, mut incoming, outbox) = play_channel();
    outbox.task_board_initialize(true);
    link.task_board_full_request(FullIntent::Refresh);
    assert!(matches!(incoming.try_recv(), Ok(PlayCommand::Feature(_))));
    let mut reply = full_reply();
    reply.bounty.state = 3;
    reply.bounty.selected = 1;
    reply.bounty.current = 10;
    let bounty = task_board::full::BountyOffer {
        elite: false,
        dust: 0,
        race: "oteryn:creature.rat".into(),
        required: 10,
        experience: 1250,
        points: 25,
        grade: 1,
    };
    reply.bounty.offers = vec![bounty.clone()];
    reply.bounty.active = Some(bounty);
    let response =
        |reply: &task_board::full::Reply| -> Result<oteryn_session::GatedCommandResult, Error> {
            Ok(oteryn_session::GatedCommandResult {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload: task_board::full::encode_reply(reply).map_err(|_| "full reply")?,
            })
        };
    outbox.task_board_full_result(&response(&reply)?);
    let nodes = tree_nodes(&link.task_board_panel());
    assert!(nodes.iter().any(|n| n.label.contains("25 task points")));
    assert!(nodes.iter().any(|n| n.label.contains("automatic")));
    assert!(nodes.iter().any(|n| n.label.contains("ready to claim")));
    assert!(nodes.iter().any(|n| n.label.contains("Loot")));
    let path = nodes
        .iter()
        .find(|n| n.path == Some(2) && n.label.starts_with("Next"))
        .and_then(|n| n.path)
        .ok_or("quoted Loot path")?;
    assert!(link.task_board_talisman_request(talisman::Intent::Upgrade(path)));
    let Ok(PlayCommand::Feature(command)) = incoming.try_recv() else {
        return Err("upgrade command".into());
    };
    let request =
        task_board::full::decode_request(&command.payload).map_err(|_| "upgrade payload")?;
    assert_eq!(
        (
            command.capability,
            command.kind,
            request.action,
            request.selection
        ),
        (72, 89, 18, 2)
    );
    assert_eq!(
        link.task_board_panel()
            .weekly
            .as_ref()
            .map(|r| r.talisman[2].level),
        Some(1)
    );
    reply.talisman[2].level = 2;
    reply.talisman[2].bonus = 350;
    reply.talisman[2].next_cost = 29;
    reply.bounty_points = 3;
    outbox.task_board_full_result(&response(&reply)?);
    assert!(
        tree_nodes(&link.task_board_panel())
            .iter()
            .any(|n| n.path == Some(2) && n.label.contains("29 bounty points"))
    );
    link.task_board_full_request(FullIntent::ClaimBounty);
    let Ok(PlayCommand::Feature(command)) = incoming.try_recv() else {
        return Err("claim command".into());
    };
    assert_eq!(
        task_board::full::decode_request(&command.payload)
            .map_err(|_| "claim payload")?
            .action,
        9
    );
    reply.bounty.state = 0;
    reply.bounty.selected = 0;
    reply.bounty.current = 0;
    reply.bounty.active = None;
    reply.bounty_points = 28;
    outbox.task_board_full_result(&response(&reply)?);
    assert_eq!(
        link.task_board_panel()
            .weekly
            .as_ref()
            .map(|r| r.weekly.pending_points),
        Some(25)
    );
    assert!(
        !tree_nodes(&link.task_board_panel())
            .iter()
            .any(|n| n.label.contains("ready to claim"))
    );
    assert!(
        oteryn_client_ui::task_board::shop_offer_text(&reply.shop[0], reply.points)
            .contains("Feral Trapper")
    );
    Ok(())
}
