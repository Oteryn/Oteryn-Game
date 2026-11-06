//! Jorge's actual NPC talk route, item custody and repeat exchange receipts.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

#[test]
fn raid_tokens_jorge_repeatable_exchange_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("hunting-for-tokens", || {
        super::quest_world_callbacks::scenario(
            "hunting-for-tokens",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED raid_tokens_jorge_repeatable_exchange_and_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [jorge] = super::classic_quests::install(owner, admitted, ["oteryn:npc.jorge"]).await?;
    super::classic_quests::talk(owner, admitted, jorge, 9000, "hi").await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: "oteryn:item.tibia.i19083".into(),
            count: 20,
        },
        400,
        true,
    )
    .await?;
    super::classic_quests::talk(owner, admitted, jorge, 9001, "abacus").await?;
    assert_eq!(
        carried_tokens(h).await?,
        20,
        "a souvenir needs an open source offer"
    );
    let before_offer: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades WHERE side=3")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(before_offer, 0);
    let before_experience: i64 = sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    super::classic_quests::talk(owner, admitted, jorge, 10001, "souvenir").await?;
    // All SQL fragments below are closed fixture literals; no player text enters DDL.
    for (stage, table, condition, command) in [
        (
            "debit",
            "game_item_instances",
            "NEW.definition_production_key='oteryn:item.tibia.i19151' AND EXISTS(SELECT 1 FROM game_npc_trades WHERE transaction_id=NEW.minted_transaction_id AND side=3)",
            10002,
        ),
        (
            "mint",
            "game_item_audit_outbox",
            "EXISTS(SELECT 1 FROM game_npc_trades WHERE transaction_id=NEW.transaction_id AND side=3)",
            10003,
        ),
    ] {
        sqlx::query("CREATE SEQUENCE fixture_token_failure_seen")
            .execute(&h.pool)
            .await?;
        sqlx::query(
            "GRANT USAGE,SELECT ON SEQUENCE fixture_token_failure_seen TO oteryn_game_runtime",
        )
        .execute(&h.pool)
        .await?;
        let function = format!(
            "CREATE FUNCTION fixture_token_fail() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF {condition} THEN PERFORM nextval('fixture_token_failure_seen'); RAISE EXCEPTION 'injected token exchange failure after {stage}'; END IF; RETURN NEW; END $$"
        );
        sqlx::query(sqlx::AssertSqlSafe(function))
            .execute(&h.pool)
            .await?;
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE TRIGGER fixture_token_fail BEFORE INSERT ON {table} FOR EACH ROW EXECUTE FUNCTION fixture_token_fail()")))
            .execute(&h.pool).await?;
        let failed = send(owner, admitted, jorge, command, "abacus").await?;
        assert!(
            failed.unknown,
            "storage failure must not acknowledge a trade after {stage}"
        );
        assert!(
            sqlx::query_scalar::<_, bool>("SELECT is_called FROM fixture_token_failure_seen")
                .fetch_one(&h.pool)
                .await?,
            "fault reached after {stage}"
        );
        let state:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19083'),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19151'),(SELECT count(*) FROM game_npc_trades WHERE side=3)")
            .fetch_one(&h.pool).await?;
        assert_eq!(state, (20, 0, 0), "atomic rollback after {stage}");
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP TRIGGER fixture_token_fail ON {table}"
        )))
        .execute(&h.pool)
        .await?;
        sqlx::query("DROP FUNCTION fixture_token_fail()")
            .execute(&h.pool)
            .await?;
        sqlx::query("DROP SEQUENCE fixture_token_failure_seen")
            .execute(&h.pool)
            .await?;
    }
    super::classic_quests::talk(owner, admitted, jorge, 10004, "abacus").await?;
    let values: (i64, i64) = sqlx::query_as("SELECT \
        coalesce((SELECT sum(quantity)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19083'),0), \
        (SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19151')")
        .fetch_one(&h.pool).await?;
    assert_eq!(
        values,
        (0, 1),
        "one exact20token debit and one fresh replica"
    );
    let experience: i64 = sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        experience, before_experience,
        "an item exchange grants no quest XP"
    );
    let offers = oteryn_gameplay_quests::quest::raid_tokens::offers()?;
    for (i, (item, name)) in offers
        .iter()
        .enumerate()
        .skip(1)
        .chain(std::iter::once((offers.len(), &offers[0])))
    {
        let previous = offers[(i - 1).min(offers.len() - 1)].0;
        drop_replica(owner, admitted, previous, 20000 + i as u64).await?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: "oteryn:item.tibia.i19083".into(),
                count: 20,
            },
            400 + i as u64,
            true,
        )
        .await?;
        let command = 10010 + i as u64 * 3;
        super::classic_quests::talk(owner, admitted, jorge, command, "souvenir").await?;
        super::classic_quests::talk(owner, admitted, jorge, command + 1, name).await?;
        let key = format!("oteryn:item.tibia.i{item}");
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
            .bind(&key).fetch_one(&h.pool).await?;
        assert_eq!(
            count,
            if i == offers.len() { 2 } else { 1 },
            "source souvenir: {name}"
        );
        let tokens:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19083'")
            .fetch_one(&h.pool).await?;
        assert_eq!(tokens, 0, "exact source cost for {name}");
    }
    let trades: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades WHERE side=3")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(trades, 38);
    super::classic_quests::talk(owner, admitted, jorge, 10122, &offers[0].1).await?;
    let replayed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades WHERE side=3")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(replayed, trades);
    let rebound = send(owner, admitted, jorge, 10122, &offers[1].1).await?;
    assert_eq!(
        rebound.disposition,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
    );
    assert!(
        !rebound.inventory_changed,
        "a committed nonce cannot select another replica"
    );
    // A closed topic and a short or split token stack must never debit anything.
    drop_replica(owner, admitted, offers[0].0, 21000).await?;
    super::classic_quests::talk(owner, admitted, jorge, 11000, "abacus").await?;
    for (count, n) in [(100, 500), (1, 501)] {
        mint(owner, admitted, 19083, count, n).await?;
    }
    drop_quantity(owner, admitted, 19083, Some(81), 20999).await?;
    super::classic_quests::talk(owner, admitted, jorge, 11001, "souvenir").await?;
    let split = super::classic_quests::talk(owner, admitted, jorge, 11002, "abacus").await?;
    assert!(split.iter().any(|line| line.contains("one stack")));
    assert_eq!(carried_tokens(h).await?, 20);
    drop_replica(owner, admitted, 19083, 21001).await?;
    drop_replica(owner, admitted, 19083, 21002).await?;
    for n in 510..529 {
        mint(owner, admitted, 23676, 1, n).await?;
    }
    mint(owner, admitted, 19083, 20, 530).await?;
    // A whole take frees its entry, so a20-entry backpack can accept the replica.
    super::classic_quests::talk(owner, admitted, jorge, 11003, "souvenir").await?;
    super::classic_quests::talk(owner, admitted, jorge, 11004, "abacus").await?;
    assert_eq!(carried_tokens(h).await?, 0);
    drop_replica(owner, admitted, 19151, 21003).await?;
    mint(owner, admitted, 19083, 100, 531).await?;
    super::classic_quests::talk(owner, admitted, jorge, 11005, "souvenir").await?;
    let full = send(owner, admitted, jorge, 11006, "abacus").await?;
    assert_eq!(
        full.disposition,
        oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected
    );
    assert!(!full.unknown);
    assert_eq!(
        carried_tokens(h).await?,
        100,
        "a partial take needs a new entry; refusal must not burn20 tokens"
    );
    drop_replica(owner, admitted, 23676, 21004).await?;
    super::classic_quests::talk(owner, admitted, jorge, 11007, "abacus").await?;
    assert_eq!(carried_tokens(h).await?, 80);
    let final_trades: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades WHERE side=3")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(final_trades, 40);
    let final_experience:i64=sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(final_experience, before_experience);
    drop_replica(owner, admitted, 23676, 21005).await?;
    super::classic_quests::talk(owner, admitted, jorge, 11008, "souvenir").await?;
    let mut stale = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?
        .item_fence();
    stale.character_lease_generation += 1;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let stale_result = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 11009,
                item_fence: Some(stale),
            },
            oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                npc_actor: jorge,
                text: "abacus".into(),
            },
        )
        .await;
    assert!(
        !stale_result.inventory_changed,
        "a changed session lease cannot spend the old topic"
    );
    assert_eq!(carried_tokens(h).await?, 80);
    let after_stale: i64 = sqlx::query_scalar("SELECT count(*) FROM game_npc_trades WHERE side=3")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(after_stale, 40);
    Ok(())
}

async fn drop_replica(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u16,
    command: u64,
) -> TestResult {
    drop_quantity(owner, admitted, item, None, command).await
}

async fn drop_quantity(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u16,
    count: Option<u32>,
    command: u64,
) -> TestResult {
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?
        .item_fence();
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let bag = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("bag")?;
    let key = format!("oteryn:item.tibia.i{item}");
    let item = bag
        .entries
        .iter()
        .find(|entry| {
            entry.item.definition.production_key == key
                && count.is_none_or(|quantity| entry.item.quantity > quantity)
        })
        .ok_or("carried replica")?
        .item
        .item_instance_id;
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    let result = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence,
            crate::foundation::CommandRef::new(
                admitted.game_session_id,
                crate::foundation::CommandId::new(command).map_err(debug)?,
            ),
            &NativeItemWeights(native),
            crate::durability::item_loop_move::LoopMoveRequest {
                item,
                count,
                pickup_cell: None,
                destination: crate::durability::item_loop_move::LoopDestination::Ground(
                    crate::durability::item_loop_move::LoopGround {
                        spatial_position:
                            crate::durability::map_item_mint_audit::map_item_spatial_position(
                                33293, 31677, -15,
                            ),
                        map_revision: crate::map::overlay::map_revision(
                            owner.bundle_map().ok_or("map")?.overlay.base(),
                        ),
                        content_revision: revision,
                        placement_context: frame.to_vec(),
                    },
                ),
            },
        )
        .await
        .map_err(|error| format!("Jorge replica drop {item:?}: {error:?}"))?;
    assert!(result.is_ok(), "drop replica: {result:?}");
    Ok(())
}

async fn send(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    command: u64,
    text: &str,
) -> TestResult<crate::gameplay_transport::npc_command::NpcTalkOutcome> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    Ok(owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            crate::gameplay_transport::connection::UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command,
                item_fence: Some(fence.item_fence()),
            },
            oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await)
}

async fn mint(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u16,
    count: u16,
    n: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33293,
                y: 31677,
                floor: -15,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count,
        },
        n,
        true,
    )
    .await
}
async fn carried_tokens(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i19083'")
        .fetch_one(&h.pool).await?)
}
