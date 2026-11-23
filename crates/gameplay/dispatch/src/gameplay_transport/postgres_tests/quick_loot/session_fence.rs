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
fn quick_loot_postgres_clicks_a_ground_corpse_without_a_runtime_projection() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let _ = admin;
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("quick-loot-ground", || {
        scenario_with_pouch(false, Some(FenceCase::GroundOnly))
    })
}

#[derive(Clone, Copy)]
pub(super) enum FenceCase {
    Visible,
    Stale,
    GroundOnly,
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
            Self::GroundOnly => {
                use crate::gameplay_transport::item_view::{
                    ItemKey, ItemViewContinuity, SessionItemView,
                };

                {
                    let mut runtime = owner.runtime.lock().await;
                    runtime.retain_live_corpse_items(&[]).map_err(debug)?;
                    assert!(runtime.visible_corpses().is_empty());
                }
                let ground = ItemKey::Instance(corpse_id);
                let mut view =
                    SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
                let handle = view
                    .map_view(&[ground], |handles| Ok(handles.handle(&ground)))
                    .map_err(debug)?
                    .ok_or("shown Ground corpse")?;
                let (status, result) = owner
                    .quick_loot(
                        *admitted,
                        admitted.continuity.next_command_id,
                        click(encode_target_handle(handle)),
                        Some(&view),
                        true,
                    )
                    .await
                    .ok_or("Quick Loot outcome")?;
                assert_eq!(status, CommandStatus::Accepted);
                assert_eq!(result.items.len(), items.len());
                assert!(result.items.iter().all(|item| item.accepted));
                let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid")
                    .bind(corpse_id.as_slice()).fetch_one(pool).await?;
                assert_eq!(remaining, 0);
            }
        }
        Ok(())
    }
}
