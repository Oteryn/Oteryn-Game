//! Stateful production-owner cases called by the admitted-connection scenario.
use super::*;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::quest_state::QuestCause;
use crate::durability::task_board::{TaskBoardCatalogue, TaskBoardCommand};
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration};
use oteryn_gameplay_quests::quest::task_board::full::TaskBoardAction;
use std::sync::Arc;

pub(super) async fn inactive_death_without_calendar(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &AdmittedSession,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    let occurrence =
        crate::durability::character_progression::ExperienceRewardOccurrence::from_bytes(
            death_occurrence(65_000),
        )
        .map_err(debug)?;
    let catalogue = Arc::new(TaskBoardCatalogue {
        creatures: Vec::new(),
        items: std::collections::BTreeMap::new(),
        gameplay_content_revision: String::new(),
        progression: None,
        content_revision: String::new(),
        cosmetics: None,
    });
    let outcome = owner
        .root
        .commit_task_board(
            owner.character,
            owner.holder,
            fence,
            TaskBoardCommand {
                cause: QuestCause::CreatureDeath(occurrence),
                action: TaskBoardAction::CreditKill {
                    race: "oteryn:creature.rat".into(),
                },
                payload_binding: Vec::new(),
                fee_change: None,
            },
            Arc::clone(&catalogue),
            None,
        )
        .await
        .map_err(debug)?;
    assert!(outcome.refusal.is_none() && outcome.transition.is_none());
    assert_eq!(outcome.view.period, 0);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_task_board_receipts WHERE cause_kind='creature_death'"
        )
        .fetch_one(&harness.pool)
        .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_task_boards")
            .fetch_one(&harness.pool)
            .await?,
        0
    );
    assert!(
        owner
            .root
            .commit_task_board(
                owner.character,
                owner.holder,
                fence,
                TaskBoardCommand {
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(900_000).map_err(debug)?
                    )),
                    action: TaskBoardAction::Query,
                    payload_binding: full_request(4, 0, ""),
                    fee_change: None
                },
                catalogue,
                None
            )
            .await
            .is_err(),
        "UI/read owner still requires the existing World calendar"
    );
    Ok(())
}

pub(super) fn full_request(action: u8, selection: u16, race: &str) -> Vec<u8> {
    wire::full::encode_request(&wire::full::Request {
        action,
        selection,
        race: race.to_owned(),
    })
    .expect("full request")
}

pub(super) async fn full_board(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    action: u8,
    selection: u16,
    race: &str,
) -> TestResult<wire::full::Reply> {
    let first = admitted.continuity.next_command_id;
    let replies = serve_from(
        owner,
        admitted,
        first,
        &[full_request(action, selection, race)],
    )
    .await?;
    assert_eq!(replies[0].1, CommandStatus::Accepted, "{replies:?}");
    wire::full::decode_reply(&replies[0].2).map_err(debug)
}

pub(super) async fn fixture_level(harness: &Harness, level: i64, experience: i64) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=$1,total_experience=$2 WHERE character_id=encode($3,'hex')::uuid")
        .bind(level).bind(experience).bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn fixture_board_data(
    harness: &Harness,
) -> TestResult<oteryn_gameplay_quests::quest::task_board::full::TaskBoardData> {
    let body: serde_json::Value = sqlx::query_scalar(
        "SELECT body FROM game_character_task_boards WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    Ok(serde_json::from_value(body)?)
}

pub(super) async fn fixture_track(harness: &Harness, key: &str, value: i64) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value")
        .bind(id(41).as_slice()).bind(key).bind(task_board::QUEST).bind(value).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// The full reset pipeline has independent owner tests. This fixture supplies its
/// next committed epoch, then exercises Task Board's production period reader.
pub(super) async fn fixture_next_reset(harness: &Harness) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) SELECT e.world_id,e.reset_epoch+1,e.target_digest,e.map_revision,e.server_digest,e.client_digest,e.frame_digest,e.calendar,e.calendar_revision,e.activated_at+interval '7 days',to_char((e.activated_at+interval '7 days' AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM') FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid")
        .bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_world_reset_active SET reset_epoch=reset_epoch+1 WHERE world_id=encode($1,'hex')::uuid")
        .bind(id(42).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn expansion_case(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    let catalogue = Arc::new(
        owner
            .task_board_catalogue()
            .ok_or("native task catalogue")?,
    );
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let command = CommandRef::new(session, CommandId::new(1_000_000).map_err(debug)?);
    let grant = slot
        .grant_task_board_expansion(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            command,
            "store-delivery:test-expansion-1",
            Arc::clone(&catalogue),
        )
        .await
        .map_err(debug)?;
    assert!(grant.refusal.is_none());
    assert!(grant.view.data.expansion);
    let weekly = grant.view.data.weekly.as_ref().expect("weekly board");
    assert_eq!((weekly.kills.len() + 1, weekly.deliveries.len()), (9, 9));
    let replay = slot
        .grant_task_board_expansion(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            CommandRef::new(session, CommandId::new(1_000_001).map_err(debug)?),
            "store-delivery:test-expansion-1",
            Arc::clone(&catalogue),
        )
        .await
        .map_err(debug)?;
    assert!(replay.replayed);
    assert_eq!(replay.view, grant.view);
    let refused = slot
        .commit_task_board(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            TaskBoardCommand {
                cause: QuestCause::Command(command),
                action: TaskBoardAction::GrantExpansion,
                payload_binding: Vec::new(),
                fee_change: None,
            },
            Arc::clone(&catalogue),
            None,
        )
        .await;
    assert!(matches!(
        refused,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    drop(slot);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_expansion_deliveries")
            .fetch_one(&harness.pool)
            .await?,
        1
    );
    let query = full_board(owner, admitted, 4, 0, "").await?;
    assert_eq!(
        (query.weekly.kills.len(), query.weekly.deliveries.len()),
        (9, 9)
    );
    println!("<TASK_BOARD> POSTGRES_CASE_COMPLETED: trusted_expansion_delivery_once");

    // New mutations cannot use an old connection generation. Same semantic query
    // under altered original bytes cannot borrow the original command receipt.
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    let mut stale = fence;
    stale.connection_generation =
        ConnectionGeneration::new(fence.connection_generation.get() + 1).map_err(debug)?;
    let command = CommandRef::new(session, CommandId::new(1_000_002).map_err(debug)?);
    let request = TaskBoardCommand {
        cause: QuestCause::Command(command),
        action: TaskBoardAction::Query,
        payload_binding: full_request(4, 0, ""),
        fee_change: None,
    };
    assert!(matches!(
        owner
            .root
            .commit_task_board(
                owner.character,
                owner.holder,
                stale,
                request.clone(),
                Arc::clone(&catalogue),
                None
            )
            .await,
        Err(CharacterProgressionError::AuthorityRejected)
    ));
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let committed = slot
        .commit_task_board(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request.clone(),
            Arc::clone(&catalogue),
            None,
        )
        .await
        .map_err(debug)?;
    assert!(committed.refusal.is_none());
    let mut altered = request;
    altered.payload_binding = super::request(0, 0);
    assert!(matches!(
        slot.commit_task_board(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            altered,
            catalogue,
            None
        )
        .await,
        Err(CharacterProgressionError::ConflictingOccurrence)
    ));
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: current_generation_and_original_payload_binding"
    );
    Ok(())
}

pub(super) async fn delivery_cases(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    super::depot_case::fixture_stackable_deliveries(owner, harness).await?;
    let board = full_board(owner, admitted, 4, 0, "").await?;
    let catalogue = owner
        .task_board_catalogue()
        .ok_or("native task catalogue")?;
    let delivery = &board.weekly.deliveries[0];
    let item = catalogue
        .items
        .get(&delivery.item)
        .ok_or("delivery item mapping")?;
    fixture_delivery_units(owner, harness, item, delivery.required, 0).await?;
    super::depot_case::fixture_large_nested_depot(owner, harness, item).await?;
    super::depot_case::availability_snapshot_case(
        owner,
        harness,
        admitted,
        item,
        delivery.required,
    )
    .await?;
    let before_xp = experience(harness).await?;
    let before_units = owned_units(harness, &item.key).await?;
    for fault in [
        "CREATE FUNCTION task_board_test_stash_audit_fault() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RETURN NULL; END $$;",
        "CREATE FUNCTION task_board_test_stash_audit_fault() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.event_id:=game_character_uuid_v7(); RETURN NEW; END $$;",
        "CREATE FUNCTION task_board_test_stash_audit_fault() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.envelope:=NEW.envelope||decode('00','hex'); NEW.envelope_sha256:=sha256(NEW.envelope); RETURN NEW; END $$;",
    ] {
        sqlx::raw_sql(fault).execute(&harness.pool).await?;
        sqlx::raw_sql("CREATE TRIGGER task_board_test_stash_audit_fault BEFORE INSERT ON game_character_stash_audit_outbox FOR EACH ROW WHEN(NEW.schema_revision=2) EXECUTE FUNCTION task_board_test_stash_audit_fault();")
            .execute(&harness.pool).await?;
        let command = admitted.continuity.next_command_id;
        let attempted = serve_from(owner, admitted, command, &[full_request(6, 0, "")]).await;
        sqlx::raw_sql("DROP TRIGGER task_board_test_stash_audit_fault ON game_character_stash_audit_outbox; DROP FUNCTION task_board_test_stash_audit_fault();").execute(&harness.pool).await?;
        let refused = attempted?;
        assert_ne!(refused[0].1, CommandStatus::Accepted);
        assert_eq!(
            (
                experience(harness).await?,
                owned_units(harness, &item.key).await?
            ),
            (before_xp, before_units),
            "missing or mismatched stash audit rolls back all delivery participants"
        );
    }
    let command = admitted.continuity.next_command_id;
    let delivered = full_board(owner, admitted, 6, 0, "").await?;
    assert_eq!(delivered.refusal, 0);
    assert!(delivered.weekly.deliveries[0].delivered);
    assert_eq!(delivered.weekly.deliveries[0].current, delivery.required);
    assert_eq!(
        owned_units(harness, &item.key).await?,
        before_units - i64::from(delivery.required)
    );
    let available_after:i64=sqlx::query_scalar("SELECT (reply->'delivery_available'->>$4)::bigint FROM game_task_board_receipts WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric")
        .bind(id(41).as_slice()).bind(admitted.game_session_id.as_bytes().as_slice()).bind(command.to_string())
        .bind(delivery.item.to_string()).fetch_one(&harness.pool).await?;
    assert_eq!(
        available_after, 0,
        "receipt captures remaining ownership after consumption; completed wire row still shows required"
    );
    assert_eq!(
        experience(harness).await?,
        before_xp + i64::try_from(board.weekly.delivery_experience)?
    );
    let source_counts: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_callback_consumptions WHERE location_kind='container'),(SELECT count(*) FROM game_task_board_stash_sources),(SELECT count(*) FROM game_quest_callback_consumptions WHERE location_kind='depot')")
        .fetch_one(&harness.pool).await?;
    assert!(
        source_counts.0 > 0 && source_counts.1 > 0 && source_counts.2 > 0,
        "{source_counts:?}"
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_quest_callback_consumptions WHERE location_kind='depot_nested'").fetch_one(&harness.pool).await?,1);
    println!("<TASK_BOARD> POSTGRES_CASE_COMPLETED: large_depot_and_nested_delivery_source");
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_task_board_stash_sources s JOIN game_character_stash_audit_outbox a USING(transaction_id,event_id,world_id,occurred_at) WHERE a.event_type_id=9 AND a.schema_revision=2 AND a.retention_profile_id='ECONOMY_LEDGER_RETENTION_V1'").fetch_one(&harness.pool).await?,source_counts.1);
    println!("<TASK_BOARD> POSTGRES_CASE_COMPLETED: stash_debit_requires_exact_economy_audit");
    let replay = serve_from(owner, admitted, command, &[full_request(6, 0, "")]).await?;
    assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
    assert_eq!(full_board(owner, admitted, 6, 0, "").await?.refusal, 2);
    let delivery = &board.weekly.deliveries[1];
    let item = catalogue
        .items
        .get(&delivery.item)
        .ok_or("second delivery item")?;
    fixture_delivery_units(owner, harness, item, delivery.required, 1).await?;
    let available = full_board(owner, admitted, 4, 0, "").await?;
    assert_eq!(
        available.weekly.deliveries[1].current,
        delivery.required - 1,
        "the missing unit in Inbox does not enter the delivery snapshot"
    );
    let before = (
        experience(harness).await?,
        owned_units(harness, &item.key).await?,
    );
    let refused = full_board(owner, admitted, 6, 1, "").await?;
    assert_eq!(refused.refusal, 13);
    assert_eq!(refused.weekly.deliveries[1].current, delivery.required - 1);
    assert_eq!(
        (
            experience(harness).await?,
            owned_units(harness, &item.key).await?
        ),
        before,
        "Inbox does not make an otherwise incomplete delivery affordable; no partial consumption"
    );
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: inventory_stash_depot_delivery_and_inbox_exclusion"
    );
    Ok(())
}

async fn owned_units(harness: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT sum(quantity) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1),0)::bigint+COALESCE((SELECT sum(quantity) FROM game_character_stash_balances WHERE definition_key=$1),0)::bigint")
        .bind(key).fetch_one(&harness.pool).await?)
}

pub(super) async fn item_shop_and_talisman_cases(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let initial = full_board(owner, admitted, 4, 0, "").await?;
    fixture_track(harness, task_board::POINTS, 200_000).await?;
    let reservations = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM game_reward_claim_inbox_mint_reservations",
    )
    .fetch_one(&harness.pool)
    .await?;
    let kit_offers: Vec<_> = oteryn_gameplay_quests::quest::task_board::full::SHOP_OFFERS
        .iter()
        .filter(|item| item.requires_decoration_kit())
        .collect();
    assert_eq!(
        kit_offers.iter().map(|offer| offer.id).collect::<Vec<_>>(),
        (10..=22).collect::<Vec<_>>(),
        "all thirteen item offers, including non-pickupable Cerberus 11, need the separate kit owner"
    );
    for offer in kit_offers {
        let refused = full_board(owner, admitted, 16, offer.id, "").await?;
        assert_eq!(
            (refused.refusal, refused.points),
            (15, 200_000),
            "{}",
            offer.name
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_reward_claim_inbox_mint_reservations"
        )
        .fetch_one(&harness.pool)
        .await?,
        reservations
    );
    fixture_track(harness, task_board::POINTS, i64::try_from(initial.points)?).await?;
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: decoration_kit_offers_refuse_without_debit_or_mint_reservation"
    );

    assert_eq!(full_board(owner, admitted, 17, 0, "").await?.refusal, 4);
    let native = owner
        .active_generation
        .ok_or("active generation")?
        .native_gameplay()
        .ok_or("native gameplay")?;
    let policy = native
        .item_policy_for_source_id(3035)
        .ok_or("platinum policy")?;
    let definition = &policy.record().production_definition;
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let coin:String=sqlx::query_scalar("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,$3,50,1,game_character_uuid_v7()) RETURNING item_instance_id::text")
        .bind(id(42).as_slice()).bind(&definition.production_key).bind(&definition.revision_ref).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT $1::uuid,s.world_id,s.character_id,s.item_instance_id,coalesce((SELECT max(placement_ordinal) FROM game_item_container_entries e WHERE e.parent_item_instance_id=s.item_instance_id),0)+1,game_character_uuid_v7() FROM game_item_container_slots s WHERE s.character_id=encode($2,'hex')::uuid")
        .bind(&coin).bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    let before_audit = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_audit_outbox")
        .fetch_one(&harness.pool)
        .await?;
    let before_items =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_character_inbox_locations")
            .fetch_one(&harness.pool)
            .await?;
    let before_grants =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_item_grants")
            .fetch_one(&harness.pool)
            .await?;
    let before_fees =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_fee_proofs")
            .fetch_one(&harness.pool)
            .await?;
    let before_receipts =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_receipts")
            .fetch_one(&harness.pool)
            .await?;
    // Fail the final source receipt after the existing gold, Item MINT, Inbox and
    // DUR-03 participants run. Their value writes must all roll back together.
    // The existing mint preparation may retain its value-free reservation.
    sqlx::raw_sql("CREATE FUNCTION task_board_test_refuse_talisman() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'task board test atomicity fault' USING ERRCODE='23514'; END $$; CREATE CONSTRAINT TRIGGER task_board_test_refuse_talisman AFTER INSERT ON game_task_board_receipts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.action=to_jsonb('BuyTalisman'::text)) EXECUTE FUNCTION task_board_test_refuse_talisman();")
        .execute(&harness.pool).await?;
    let command = admitted.continuity.next_command_id;
    let failed = serve_from(owner, admitted, command, &[full_request(17, 0, "")]).await;
    sqlx::raw_sql("DROP TRIGGER task_board_test_refuse_talisman ON game_task_board_receipts; DROP FUNCTION task_board_test_refuse_talisman();").execute(&harness.pool).await?;
    let failed = failed?;
    assert_ne!(failed[0].1, CommandStatus::Accepted);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT quantity FROM game_item_instances WHERE item_instance_id=$1::uuid"
        )
        .bind(&coin)
        .fetch_one(&harness.pool)
        .await?,
        50
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_item_container_entries WHERE item_instance_id=$1::uuid"
        )
        .bind(&coin)
        .fetch_one(&harness.pool)
        .await?,
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT value FROM game_character_quest_tracks WHERE track_key=$1"
        )
        .bind(task_board::POINTS)
        .fetch_one(&harness.pool)
        .await?,
        i64::try_from(initial.points)?
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_character_inbox_locations")
            .fetch_one(&harness.pool)
            .await?,
        before_items
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_audit_outbox")
            .fetch_one(&harness.pool)
            .await?,
        before_audit
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_item_grants")
            .fetch_one(&harness.pool)
            .await?,
        before_grants
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_fee_proofs")
            .fetch_one(&harness.pool)
            .await?,
        before_fees
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_task_board_receipts")
            .fetch_one(&harness.pool)
            .await?,
        before_receipts
    );
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: talisman_existing_mint_audit_and_gold_atomic_rollback"
    );

    let command = admitted.continuity.next_command_id;
    let original = serve_from(owner, admitted, command, &[full_request(17, 0, "")]).await?;
    assert_eq!(original[0].1, CommandStatus::Accepted);
    let talisman = wire::full::decode_reply(&original[0].2).map_err(debug)?;
    assert_eq!((talisman.refusal, talisman.points), (0, initial.points));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT quantity FROM game_item_instances WHERE item_instance_id=$1::uuid"
        )
        .bind(&coin)
        .fetch_one(&harness.pool)
        .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_task_board_fee_proofs WHERE fee_gold_units=5000"
        )
        .fetch_one(&harness.pool)
        .await?,
        1
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_task_board_item_grants g JOIN game_reward_claim_inbox_mint_lines l USING(item_instance_id,transaction_id) JOIN game_item_audit_outbox a USING(transaction_id) WHERE g.source_item_id=51978 AND l.source_line=51978 AND a.event_type_id=2").fetch_one(&harness.pool).await?,1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_character_inbox_locations")
            .fetch_one(&harness.pool)
            .await?,
        before_items + 1
    );
    let committed_audit =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_audit_outbox")
            .fetch_one(&harness.pool)
            .await?;
    let replay = serve_from(owner, admitted, command, &[full_request(17, 0, "")]).await?;
    assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
    assert_eq!(replay[0].2, original[0].2);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_audit_outbox")
            .fetch_one(&harness.pool)
            .await?,
        committed_audit
    );
    assert_eq!(full_board(owner, admitted, 17, 0, "").await?.refusal, 4);
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: talisman_existing_gold_fee_and_inbox_mint_once"
    );
    Ok(())
}

/// Historical fixture custody is seeded with guards disabled; production removal
/// below must supply fresh proof under the normal runtime role. The shortage case
/// puts the missing units in Inbox, which must remain inaccessible to delivery.
async fn fixture_delivery_units(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    item: &crate::durability::task_board::TaskBoardItem,
    required: u32,
    shortage: u32,
) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let backpack:Option<String>=sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_optional(&mut *tx).await?;
    let backpack = if let Some(backpack) = backpack {
        backpack
    } else {
        let native = owner
            .active_generation
            .ok_or("active generation")?
            .native_gameplay()
            .ok_or("native gameplay")?;
        let policy = native
            .item_policy_for_source_id(2854)
            .ok_or("backpack policy")?;
        let definition = &policy.record().production_definition;
        let backpack:String=sqlx::query_scalar("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,$3,1,1,game_character_uuid_v7()) RETURNING item_instance_id::text")
            .bind(id(42).as_slice()).bind(&definition.production_key).bind(&definition.revision_ref).fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,$2::uuid,encode($3,'hex')::uuid,game_character_uuid_v7())")
            .bind(id(41).as_slice()).bind(&backpack).bind(id(42).as_slice()).execute(&mut *tx).await?;
        backpack
    };
    let carried = (required / 3).clamp(1, 100);
    let depot = (required / 3).clamp(1, 100);
    let stash = required
        .checked_sub(carried + depot + shortage)
        .ok_or("delivery quantity")?;
    for (kind, quantity) in [(0, carried), (1, depot), (2, shortage)] {
        if quantity == 0 {
            continue;
        }
        let instance:String=sqlx::query_scalar("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',$2,$3,$4,1,game_character_uuid_v7()) RETURNING item_instance_id::text")
            .bind(id(42).as_slice()).bind(&item.key).bind(&item.revision).bind(i64::from(quantity)).fetch_one(&mut *tx).await?;
        match kind {
            0 => {
                sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT $1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::uuid,coalesce(max(placement_ordinal),0)+1,game_character_uuid_v7() FROM game_item_container_entries WHERE parent_item_instance_id=$4::uuid")
                .bind(&instance).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(&backpack).execute(&mut *tx).await?;
            }
            1 => {
                sqlx::query("INSERT INTO game_item_character_depot_locations(item_instance_id,world_id,character_id,town_id,box_number,ordinal) SELECT $1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,coalesce(max(ordinal),0)+1 FROM game_item_character_depot_locations WHERE character_id=encode($3,'hex')::uuid AND town_id=1 AND box_number=1")
                .bind(&instance).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
            }
            _ => {
                sqlx::query("INSERT INTO game_item_character_inbox_locations(item_instance_id,world_id,character_id,ordinal) SELECT $1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,coalesce(max(ordinal),0)+1 FROM game_item_character_inbox_locations WHERE character_id=encode($3,'hex')::uuid")
                .bind(&instance).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
            }
        }
    }
    sqlx::query("INSERT INTO game_character_depot_box_counters(character_id,town_id,box_number,next_ordinal,committed) SELECT character_id,1,1,max(ordinal)+1,count(*) FROM game_item_character_depot_locations WHERE character_id=encode($1,'hex')::uuid AND town_id=1 AND box_number=1 GROUP BY character_id ON CONFLICT(character_id,town_id,box_number) DO UPDATE SET next_ordinal=EXCLUDED.next_ordinal,committed=EXCLUDED.committed")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_inbox_counters(character_id,next_ordinal,committed) SELECT character_id,max(ordinal)+1,count(*) FROM game_item_character_inbox_locations WHERE character_id=encode($1,'hex')::uuid GROUP BY character_id ON CONFLICT(character_id) DO UPDATE SET next_ordinal=EXCLUDED.next_ordinal,committed=EXCLUDED.committed")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    let entry: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_stash_entries(entry_id,transaction_id,character_id,world_id,definition_key,definition_revision,amount,quantity_before,quantity_after) VALUES($1::uuid,game_character_uuid_v7(),encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,$6,0,$6)")
        .bind(&entry).bind(id(41).as_slice()).bind(id(42).as_slice()).bind(&item.key).bind(&item.revision).bind(i64::from(stash)).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_stash_balances(character_id,world_id,definition_key,definition_revision,quantity,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6::uuid)")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(&item.key).bind(&item.revision).bind(i64::from(stash)).bind(&entry).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
