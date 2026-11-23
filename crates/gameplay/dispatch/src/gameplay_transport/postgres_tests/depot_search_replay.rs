use super::full_world_support::{TestResult, world_scenario};
use super::kill_reward_e2e_fixtures::scenario_thread;
use crate::durability::DurabilityError;
use oteryn_protocol_oteryn::depot_search::{Outcome, Request};
use oteryn_protocol_oteryn::stash::{StashAction, StashIntent, StashOutcome};

const CLIENT_ID: u32 = 3028;
const ITEM: &str = "oteryn:item.tibia.i3028";
const COMMAND: u64 = 42;

#[test]
fn depot_search_retrieve_all_replays_stash_result_after_balance_reaches_zero() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario_thread::run("depot-search-stash-replay", || scenario(false, None))
}

#[test]
fn depot_search_retrieve_rejects_changed_request_under_same_command() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario_thread::run("depot-search-changed-request", || scenario(true, None))
}

#[test]
fn depot_search_unknown_withdrawal_resumes_the_original_wire_command() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario_thread::run("depot-search-unknown-ack", || scenario(false, Some(DurabilityError::CommitOutcomeUnknown)))
}

#[test]
fn depot_search_deadline_with_committed_withdrawal_replays_retrieved() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario_thread::run("depot-search-deadline-ack", || {
        scenario(false, Some(DurabilityError::RootPassDeadlineExceeded))
    })
}

fn scenario(change_request: bool, lost_ack: Option<DurabilityError>) -> TestResult {
    world_scenario!(
        "depot_search_stash_replay",
        super::house_transfer_fixtures::prepare_owned_session_request,
        |h,
         _request,
         seal,
         authority,
         world_id,
         native,
         world,
         cells,
         room,
         pin,
         deps,
         actor,
         session,
         owner,
         fence,
         admitted| {
            let recovery = if lost_ack.is_some() {
                Some(super::house_transfer_fixtures::RecoverySource::start(&h).await?)
            } else {
                None
            };
            if let Some(recovery) = &recovery {
                owner.evidence = &recovery.evidence;
            }
            while position(&owner, actor).await?.x > LOCKER_CELL.x {
                step_west(&mut owner, actor, session).await?;
            }
            assert_eq!(position(&owner, actor).await?, LOCKER_CELL);
            let stashed = seed_carried_stack(&h.pool, 37).await?;
            let deposited = seed_carried_stack(&h.pool, 20).await?;
            let command = |command_id| UseCommand {
                game_session_id: session,
                command_id,
                item_fence: Some(fence),
            };
            let handle = std::num::NonZeroU64::new(1).ok_or("item handle")?;
            let stow = StashIntent {
                page: 0,
                action: StashAction::Stow(handle),
            };
            let stowed = owner
                .stash(
                    actor,
                    command(40),
                    stow,
                    Some(ItemKey::Instance(stashed)),
                    true,
                )
                .await
                .ok_or("stow reply")?;
            assert_eq!(stowed.outcome, StashOutcome::Committed);
            assert_eq!(stowed.stowed_quantity, 37);
            let deposit = DepotIntent {
                town: Some(0),
                box_number: DEPOT_BOX,
                page: 0,
                action: DepotAction::Deposit(handle),
            };
            let moved = owner
                .depot(
                    actor,
                    command(41),
                    deposit,
                    Some(ItemKey::Instance(deposited)),
                )
                .await
                .ok_or("depot reply")?;
            assert_eq!(moved.reply.outcome, DepotOutcome::Moved);

            let retrieve = |client_id, all| {
                owner.depot_search_command(
                    actor,
                    admitted,
                    COMMAND,
                    Request::Retrieve { client_id, all },
                )
            };
            if let Some(lost_ack) = lost_ack {
                reconcile_wire_withdrawal(&owner, admitted, lost_ack).await?;
                assert_eq!(balances(&h.pool).await?, (0, 37, 20));
                return Ok(());
            }
            let first = retrieve(CLIENT_ID, true).await;
            assert_eq!((first.outcome, first.retrieved), (Outcome::Retrieved, 37));
            assert_eq!(balances(&h.pool).await?, (0, 37, 20));
            if change_request {
                for (client_id, all) in [(CLIENT_ID, false), (3031, true)] {
                    let changed = retrieve(client_id, all).await;
                    assert_eq!((changed.outcome, changed.retrieved), (Outcome::Rejected, 0));
                }
            }
            assert_eq!(retrieve(CLIENT_ID, true).await, first);
            assert_eq!(balances(&h.pool).await?, (0, 37, 20));

            let receipts: Vec<(Vec<u8>, bool, i64, bool)> = sqlx::query_as(
                "SELECT r.request,r.found,r.retrieved,r.created_xact_id=s.created_xact_id \
             FROM game_depot_search_retrieve_receipts r JOIN game_character_stash_operations s \
             USING(game_session_id,command_id,character_id) WHERE r.command_id=$1::bigint",
            )
            .bind(i64::try_from(COMMAND)?)
            .fetch_all(&h.pool)
            .await?;
            let mut canonical = CLIENT_ID.to_be_bytes().to_vec();
            canonical.push(1);
            assert_eq!(receipts, [(canonical, true, 37, true)]);
            let children: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_item_loop_move_receipts WHERE quick_loot_ordinal>0",
            )
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(children, 0);
            Ok(())
        }
    )
}

async fn reconcile_wire_withdrawal(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    lost_ack: DurabilityError,
) -> TestResult {
    use super::wave2_replay_wire::{exchange, reconnect};
    use crate::foundation::CommandStatus;
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use oteryn_protocol_oteryn::depot_search as wire;

    let supported = [4, 6, 18, 37, 62, 153];
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &supported)
            .filter(|selected| selected.contains(wire::CAPABILITY_DEPOT_SEARCH_V1))
            .ok_or("depot search capability")?;
    let payload = wire::encode_request(Request::Retrieve {
        client_id: CLIENT_ID,
        all: true,
    })
    .map_err(super::fixture::debug)?;
    let command = admitted.continuity.next_command_id;
    let kind = wire::COMMAND_TYPE_DEPOT_SEARCH;
    let lost = crate::gameplay_transport::depot_search::LOSE_DEPOT_SEARCH_ACK
        .scope(
            std::cell::Cell::new(Some(lost_ack)),
            exchange(owner, admitted, command, kind, &payload, None),
        )
        .await?;
    assert!(
        lost.command_result.is_none(),
        "an unknown retrieval sent a terminal reply"
    );
    let resumed = reconnect(owner, lost.retained, &supported).await?;
    assert!(owner.states.command_ingress.resumable(&resumed));
    let replay = exchange(owner, resumed, command, kind, &payload, None).await?;
    let (status, reply) = replay.command_result.ok_or("replayed depot retrieval")?;
    assert_eq!(status, CommandStatus::DuplicateReplay);
    let reply = wire::decode_reply(&reply).map_err(super::fixture::debug)?;
    assert_eq!((reply.outcome, reply.retrieved), (Outcome::Retrieved, 37));
    Ok(())
}

async fn balances(pool: &sqlx::PgPool) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT quantity FROM game_character_stash_balances WHERE definition_key=$1),
          (SELECT sum(i.quantity)::bigint FROM game_item_container_entries e
             JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1),
          (SELECT sum(i.quantity)::bigint FROM game_item_character_depot_locations d
             JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1)",
    )
    .bind(ITEM)
    .fetch_one(pool)
    .await?)
}

async fn seed_carried_stack(pool: &sqlx::PgPool, quantity: i32) -> TestResult<[u8; 16]> {
    use super::fixture::id;

    // Seed previously acquired small diamonds. The stow, depot deposit and retrieval use the live owners.
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let instance: Vec<u8> = sqlx::query_scalar(
        "WITH item AS (\
           INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) \
           SELECT game_character_uuid_v7(),world_id,'Item',$2,'definition-r1',$3,1,game_character_uuid_v7() \
           FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid RETURNING item_instance_id,world_id\
         ) INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) \
           SELECT i.item_instance_id,i.world_id,s.character_id,s.item_instance_id,\
                  (SELECT coalesce(max(e.placement_ordinal),0)+1 FROM game_item_container_entries e WHERE e.parent_item_instance_id=s.item_instance_id),game_character_uuid_v7() \
           FROM item i JOIN game_item_container_slots s USING(world_id) WHERE s.character_id=encode($1,'hex')::uuid RETURNING uuid_send(item_instance_id)",
    ).bind(id(41).as_slice()).bind(ITEM).bind(quantity).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    instance.try_into().map_err(|_| "item UUID".into())
}
