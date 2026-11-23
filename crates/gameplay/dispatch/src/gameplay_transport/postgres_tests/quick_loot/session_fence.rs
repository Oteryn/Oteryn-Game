use super::*;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;

use super::super::quick_loot_fixtures::exchange;

#[test]
fn quick_loot_resolves_visible_corpse_and_recipient_under_session_fence() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let _ = admin;
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("quick-loot-visible", || {
        scenario_with_pouch(false, Some(FenceCase::Visible))
    })
}

#[test]
fn quick_loot_refuses_stale_handles_without_durable_writes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let _ = admin;
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("quick-loot-stale", || {
        scenario_with_pouch(false, Some(FenceCase::Stale))
    })
}

#[test]
fn quick_loot_click_accepts_durable_ground_corpse_without_runtime_projection() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("quick-loot-durable-ground", || {
        scenario_with_pouch(false, Some(FenceCase::DurableGround))
    })
}

pub(super) async fn check_durable_ground(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    corpse: [u8; 16],
    items: &[[u8; 16]],
) -> TestResult {
    use crate::gameplay_transport::item_view::{ItemKey, SessionItemView};

    assert!(owner.runtime.lock().await.visible_corpses().is_empty());
    let key = ItemKey::Instance(corpse);
    let mut view = SessionItemView::resume(admitted.continuity.item_view).with_map_view();
    view.tree_delta(&[key], |_| Ok(Vec::new())).map_err(debug)?;
    let handle = view.table().handle(&key).ok_or("corpse handle")?;
    let click = QuickLootRequest {
        corpse: Some(encode_target_handle(handle)),
        ..request(QuickLootAction::Click)
    };
    let command_id = admitted.continuity.next_command_id;
    let payload = encode_quick_loot_request(&click).map_err(debug)?;
    let wire = crate::gameplay_transport::session_ingress::QuickLootWire::new(
        COMMAND_TYPE_QUICK_LOOT_V2, &payload,
    ).ok_or("quick-loot binding")?;
    let wrong_view = owner
        .quick_loot(admitted, command_id + 1, click, wire, Some(&view))
        .await
        .ok_or("quick-loot dispatch refused to reply")?;
    assert_eq!(wrong_view.0, CommandStatus::Rejected);

    let map_handle = view
        .map_view(&[key], |table| Ok(table.handle(&key)))
        .map_err(debug)?;
    assert_eq!(map_handle, Some(handle));
    use crate::gameplay_transport::session_ingress::{CommandAction, SessionCommandStream};
    let mut ingress = SessionCommandStream::open(Some(&owner.states.command_ingress), &admitted)
        .ok_or("quick loot command stream")?;
    assert!(matches!(
        ingress.begin(command_id, admitted.continuity.connection_generation,
            admitted.continuity.server_sequence, false).map_err(debug)?,
        CommandAction::Execute
    ));
    let reply = owner
        .quick_loot(admitted, command_id, click, wire, Some(&view))
        .await
        .ok_or("quick-loot dispatch refused to reply")?;
    assert_eq!(reply.0, CommandStatus::Accepted);
    let replay = owner
        .quick_loot(admitted, command_id, click, wire, None)
        .await
        .ok_or("hidden-target replay")?;
    assert_eq!(replay, reply);
    // Lose the outbound acknowledgement after the real item transaction.
    drop(ingress);
    let mut resumed = admitted;
    assert!(owner.recover_pending_quick_loot(&resumed).await);
    owner.states.command_ingress.recover_trade_continuity(&mut resumed)
        .ok_or("quick loot reconnect continuity")?;
    assert_eq!(resumed.continuity.next_command_id, command_id + 1);
    let mut ingress = SessionCommandStream::open(Some(&owner.states.command_ingress), &resumed)
        .ok_or("recovered command stream")?;
    let CommandAction::Replay(frame) = ingress.begin(command_id, resumed.continuity.connection_generation,
        resumed.continuity.server_sequence, false).map_err(debug)?
    else { return Err("committed quick loot response was not retained".into()); };
    let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame).map_err(debug)?;
    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
    assert_eq!(result.status, CommandStatus::DuplicateReplay);
    assert_eq!(decode_quick_loot_result(result.payload).map_err(debug)?, reply.1);
    assert_eq!(
        reply.1.items,
        items
            .iter()
            .map(|&item| QuickLootItemResult { item, accepted: true })
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) enum FenceCase {
    Visible,
    Stale,
    DurableGround,
}

impl FenceCase {
    pub(super) async fn check(
        self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        admitted: &mut AdmittedSession,
        pool: &sqlx::PgPool,
        corpse_identity: [u8; 16],
        corpse_id: [u8; 16],
        items: &[[u8; 16]],
    ) -> TestResult {
        let assign = |handle| QuickLootRequest {
            recipient: Some(QuickLootRecipient::Container),
            recipient_id: Some(handle),
            ..request(QuickLootAction::Assign)
        };
        let click = |handle| QuickLootRequest {
            corpse: Some(handle),
            ..request(QuickLootAction::Click)
        };
        match self {
            Self::DurableGround => return Err("durable Ground case uses its own item view".into()),
            Self::Visible => {
                let (_, replies) = exchange(owner, admitted, corpse_identity, |shown| {
                    Ok(vec![
                        assign(shown.bag.ok_or("shown bag")?),
                        click(shown.corpse.ok_or("shown corpse")?),
                    ])
                })
                .await?;
                assert_eq!(replies[0].0, CommandStatus::Accepted);
                assert_eq!(replies[1].0, CommandStatus::Accepted);
                assert_eq!(
                    replies[1].1.items,
                    items
                        .iter()
                        .map(|&item| QuickLootItemResult {
                            item,
                            accepted: true
                        })
                        .collect::<Vec<_>>()
                );
                let quantities: Vec<i64> = sqlx::query_scalar("SELECT i.quantity FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY i.quantity")
                    .bind(id(71).as_slice()).fetch_all(pool).await?;
                assert_eq!(quantities, [20, 100]);
            }
            Self::Stale => {
                let (previous, _) =
                    exchange(owner, admitted, corpse_identity, |_| Ok(Vec::new())).await?;
                let corpse = previous.corpse.ok_or("previously shown corpse")?;
                let bag = previous.bag.ok_or("previously shown bag")?;
                let (_, replies) = exchange(owner, admitted, corpse_identity, |shown| {
                    let current_corpse = shown.corpse.ok_or("current corpse handle")?;
                    let current_bag = shown.bag.ok_or("current bag handle")?;
                    assert_ne!(current_corpse, corpse);
                    assert_ne!(current_bag, bag);
                    Ok(vec![
                        click(corpse),
                        assign(bag),
                        click(corpse_id),
                        assign(id(71)),
                        click(current_bag),
                        assign(current_corpse),
                    ])
                })
                .await?;
                let reads: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_quick_loot_receipts), (SELECT count(*) FROM game_character_quick_loot_settings), (SELECT count(*) FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)")
                    .bind(corpse_id.as_slice()).fetch_one(pool).await?;
                assert_eq!(reads, (0, 0, i64::try_from(items.len())?));
                assert_eq!(
                    replies,
                    vec![
                        (
                            CommandStatus::Rejected,
                            QuickLootResult { items: Vec::new() }
                        );
                        6
                    ]
                );
            }
        }
        Ok(())
    }
}
