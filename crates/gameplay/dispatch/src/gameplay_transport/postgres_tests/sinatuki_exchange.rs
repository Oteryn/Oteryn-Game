//! Sinatuki's historical trades and quest cutover through PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::domain::CharacterId;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::npc_trade::{
    NpcTradeCause, NpcTradeCoins, NpcTradeError, NpcTradeLineKind, NpcTradeOutcome,
    NpcTradeRequest, NpcTradeSide,
};
use crate::foundation::{
    ChannelContentPin, CharacterLease, CommandId, CommandRef, MovementFacing, NpcPlacement,
    NpcPlacementTable, NpcPlacements, WorldId,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use crate::gameplay_transport::npc_command::bank_coin_definitions;
use crate::gameplay_transport::{npc_talk::NpcTalkBook, npc_trade_command};
use oteryn_gameplay_quests::quest::reward_callbacks::SINATUKI_LEGACY_COMPLETION;
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const NPC: &str = "oteryn:npc.sinatuki";
const TROUT: &str = "oteryn:item.tibia.i7158";
const PERCH: &str = "oteryn:item.tibia.i7159";
const SHARD: &str = "oteryn:item.tibia.i7290";
const TRACKS: [(&str, i64); 2] = [
    (
        "oteryn:quest-progress/quest/u8_0/barbarian_test/questline",
        3,
    ),
    (
        "oteryn:quest-progress/quest/u8_0/the_ice_islands/questline",
        5,
    ),
];

#[test]
fn sinatuki_exchange_postgres_dispatch() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .name("sinatuki-exchange".into())
        .stack_size(64 << 20)
        .spawn(|| {
            scenario(false)
                .and_then(|()| scenario(true))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Sinatuki PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED sinatuki_exchange_postgres_dispatch {:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario(fragmented: bool) -> TestResult {
    run(async move |admin| {
        let label = if fragmented {
            "sinatuki_fragmented"
        } else {
            "sinatuki_exchange"
        };
        let h = Harness::create_with_admission_seed(admin, label, true, false).await?;
        eprintln!("POSTGRES_STARTED sinatuki_exchange_postgres_dispatch");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let deps = Dependencies::with_content(
            channel_pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        let quests = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        owner.quest_catalogue = Some(Arc::new(quests.catalogue().clone()));
        deps.initialize(&owner).await;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        let book = NpcTalkBook::from_model(&model);
        let catalogue_revision = book.catalogue_revision();
        owner.states.chat.install_npc_talk(Arc::new(book));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let at = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: NPC.into(),
                            placements: vec![NpcPlacement {
                                x: u16::try_from(at.x + 1)?,
                                y: u16::try_from(at.y)?,
                                floor: -i8::try_from(at.floor)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            let placed = runtime
                .visible_entities()
                .npcs
                .into_iter()
                .next()
                .ok_or("Sinatuki placement")?;
            EntityRef {
                identity: placed.actor.placement_identity(),
                generation: placed.generation,
            }
        };
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "item fence")?
            .ok_or("item fence")?
            .item_fence();
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        owner.refresh_quest_session(session, true).await;
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        let character = fence.character_id;
        assert!(
            !h.root
                .has_sinatuki_legacy_payout(&authority, character)
                .await
                .map_err(debug)?
        );
        let greeting = talk(&mut owner, deps.actor, command(1), npc, "hi").await?;
        assert!(
            greeting
                .conversation
                .ok_or("Sinatuki greeting")?
                .open
                .is_some()
        );
        let without_receipt = talk_outcome(&mut owner, deps.actor, command(2), npc, "nupi").await?;
        assert!(!without_receipt.unknown && !without_receipt.inventory_changed);
        assert_credit(&h, false).await?;

        seed(&h, fragmented).await?;
        set_legacy_predicate(&h, true).await?;
        let initial = if fragmented {
            [100, 100, 0]
        } else {
            [99, 99, 0]
        };
        if !fragmented {
            for (id, text, expected) in [
                (3, "bye nupi", Some(("Good bye, Fixture Hero.", false))),
                (4, "hi nupi", Some(("Greetings, Fixture Hero.", true))),
                (
                    5,
                    "chuqua nupi",
                    Some(("Chuqua jamjam!! Tiyopa Sinatuki?", true)),
                ),
                (6, "hi nupi", None),
                (7, "trade nupi", None),
                (8, "offers nupi", None),
            ] {
                let outcome = talk(&mut owner, deps.actor, command(id), npc, text).await?;
                assert_eq!(outcome.conversation.is_some(), expected.is_some(), "{text}");
                if let Some((reply, open)) = expected {
                    let conversation = outcome.conversation.ok_or("missing ordinary reply")?;
                    assert_eq!(conversation.lines, [reply], "{text}");
                    assert_eq!(conversation.open.is_some(), open, "{text}");
                }
                assert!(!outcome.inventory_changed, "{text}");
                assert_inventory(&h, initial).await?;
                assert_eq!(receipt_count(&h).await?, 0, "{text}");
            }
            for (track, minimum) in TRACKS {
                set_progress(&h, track, minimum - 1).await?;
                assert!(matches!(
                    legacy_trade(&owner, command(9), npc, catalogue_revision).await?,
                    Err(NpcTradeError::RequirementNotMet)
                ));
                assert_inventory(&h, initial).await?;
                set_progress(&h, track, minimum).await?;
            }
        }
        let paid = legacy_trade(&owner, command(10), npc, catalogue_revision)
            .await?
            .map_err(debug)?;
        let NpcTradeOutcome::Committed(paid) = paid else {
            return Err("expected a newly committed old payout".into());
        };
        let debited: u32 = paid
            .lines
            .iter()
            .filter(|line| line.kind == NpcTradeLineKind::ItemBurn)
            .map(|line| line.quantity_before - line.quantity_after)
            .sum();
        assert_eq!(debited, if fragmented { 100 } else { 99 });
        let after_payout = if fragmented { [100, 0, 5] } else { [0, 99, 5] };
        assert_inventory(&h, after_payout).await?;
        assert_eq!(receipt_count(&h).await?, 1);
        if !fragmented {
            assert!(matches!(
                legacy_trade(&owner, command(11), npc, catalogue_revision).await?,
                Ok(NpcTradeOutcome::Committed(_))
            ));
            let mut fixture = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *fixture)
                .await?;
            seed_entry(&mut fixture, 207, PERCH, 98, 6).await?;
            fixture.commit().await?;
            assert!(matches!(
                legacy_trade(&owner, command(12), npc, catalogue_revision).await?,
                Err(NpcTradeError::ItemNotFound)
            ));
        }
        let unchanged = if fragmented {
            after_payout
        } else {
            [0, 98, 10]
        };
        let trade_count = if fragmented { 1 } else { 2 };
        assert_inventory(&h, unchanged).await?;
        assert_credit(&h, false).await?;

        set_legacy_predicate(&h, false).await?;
        assert!(matches!(
            legacy_trade(&owner, command(13), npc, catalogue_revision).await?,
            Err(NpcTradeError::RequirementNotMet)
        ));
        let replay = talk(&mut owner, deps.actor, command(10), npc, "nupi").await?;
        assert!(replay.inventory_changed, "the original receipt must replay");
        assert!(replay.conversation.is_none());
        assert_credit(&h, false).await?;
        assert!(
            h.root
                .has_sinatuki_legacy_payout(&authority, character)
                .await
                .map_err(debug)?
        );
        let other = CharacterId::decode(&id(101)).map_err(debug)?;
        assert!(
            !h.root
                .has_sinatuki_legacy_payout(&authority, other)
                .await
                .map_err(debug)?
        );

        owner.refresh_quest_session(session, true).await;
        owner
            .states
            .npc_conversations
            .fixture_interrupt_quest_dialogue_commit(session, 14)?;
        let lost_ack = talk_outcome(&mut owner, deps.actor, command(14), npc, "nupi").await?;
        assert!(lost_ack.unknown);
        assert_credit(&h, true).await?;
        for _ in 0..2 {
            let replay = talk(&mut owner, deps.actor, command(14), npc, "nupi").await?;
            assert!(!replay.inventory_changed);
            assert!(replay.conversation.is_some());
        }
        let repeat = talk_outcome(&mut owner, deps.actor, command(15), npc, "nupi").await?;
        assert_eq!(repeat.disposition, NpcIntentDisposition::Rejected);
        assert!(!repeat.inventory_changed && !repeat.unknown);
        assert_inventory(&h, unchanged).await?;
        assert_eq!(receipt_count(&h).await?, trade_count);
        assert_credit(&h, true).await?;
        drop(seal);
        h.cleanup().await
    })
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    npc: EntityRef,
    text: &str,
) -> TestResult<oteryn_gameplay_npc::npc_command::NpcTalkOutcome> {
    let outcome = talk_outcome(owner, actor, command, npc, text).await?;
    assert!(!outcome.unknown);
    assert_eq!(outcome.disposition, NpcIntentDisposition::Ok, "{text}");
    Ok(outcome)
}

async fn talk_outcome(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    npc: EntityRef,
    text: &str,
) -> TestResult<oteryn_gameplay_npc::npc_command::NpcTalkOutcome> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    Ok(owner
        .npc_talk(
            actor,
            command,
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await)
}

async fn legacy_trade(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
    npc: EntityRef,
    catalogue_revision: u64,
) -> TestResult<Result<NpcTradeOutcome, NpcTradeError>> {
    let fence = command.item_fence.ok_or("trade fence")?;
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native generation")?;
    let item = generation_item_facts(
        native,
        &TypedDefinitionRef {
            family: "Item".into(),
            production_key: SHARD.into(),
            revision_ref: "definition-r1".into(),
        },
    )
    .map_err(debug)?;
    let backpack = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    let backpack = generation_item_facts(native, &backpack.backpack.definition).map_err(debug)?;
    let (gold, platinum, crystal) = {
        let runtime = owner.runtime.lock().await;
        bank_coin_definitions(owner, &runtime).ok_or("coins")?
    };
    let session = command.game_session_id;
    let id = |tag: &[u8]| npc_trade_command::derived(tag, session, command.command_id);
    let request = NpcTradeRequest {
        fluid_subtype: None,
        full_charges: None,
        cause: NpcTradeCause {
            npc_key: NPC.into(),
            catalogue_revision,
            offer_index: 0,
            side: NpcTradeSide::Exchange,
            occurrence: npc_trade_command::occurrence(session, command.command_id, npc),
        },
        command: CommandRef::new(session, CommandId::new(command.command_id).map_err(debug)?),
        quantity: 1,
        unit_price: 0,
        offer_count: 5,
        item,
        transaction_id: id(b"oteryn:pick-exchange.transaction:v1"),
        event_id: id(b"oteryn:pick-exchange.event:v1"),
        occurred_at_unix_ms: npc_trade_command::unix_ms_now(),
        output_item_instance_ids: [
            id(b"oteryn:pick-exchange.output1:v1"),
            id(b"oteryn:pick-exchange.output2:v1"),
            id(b"oteryn:pick-exchange.output3:v1"),
        ],
        coins: NpcTradeCoins {
            gold,
            platinum,
            crystal,
        },
        backpack,
    };
    Ok(owner
        .root
        .exchange_sinatuki_fish(
            owner.character,
            owner.holder,
            fence,
            request,
            &NativeItemWeights(native),
        )
        .await)
}

async fn set_legacy_predicate(h: &Harness, enabled: bool) -> TestResult {
    const LEGACY: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/migrations/2896_sinatuki_fish_exchange.sql"
    ));
    const CUTOVER: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/migrations/2969_sinatuki_single_reward_owner.sql"
    ));
    // The current binary checks the embedded migration ledger at admission.
    // Restore the old predicate in this disposable DB while the real old writer runs.
    let sql = if enabled {
        let (_, definition) = LEGACY
            .split_once("CREATE FUNCTION ")
            .ok_or("legacy predicate")?;
        let (definition, _) = definition.split_once("REVOKE ALL").ok_or("legacy grant")?;
        format!("CREATE OR REPLACE FUNCTION {definition}")
    } else {
        CUTOVER
            .split_once("CREATE INDEX")
            .ok_or("cutover index")?
            .0
            .to_owned()
    };
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
        .execute(&h.pool)
        .await?;
    Ok(())
}

async fn assert_credit(h: &Harness, complete: bool) -> TestResult {
    let counts: (i64, i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
            (SELECT count(*) FROM game_character_quest_states
             WHERE quest_key='oteryn:quest.sinatuki_quest' AND completed_character_revision IS NOT NULL),
            (SELECT count(*) FROM game_character_quest_tracks
             WHERE track_key='oteryn:quest-progress/qc/sinatuki/fish' AND value=2),
            (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1),
            (SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1 AND mint_transaction_id IS NULL),
            (SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/jamjam'),
            (SELECT count(*) FROM game_account_achievement_grant_requests WHERE achievement_key='oteryn:achievement/jamjam'),
            (SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$1),
            (SELECT count(*) FROM game_reward_claim_mint_receipts)",
    ).bind(SINATUKI_LEGACY_COMPLETION).fetch_one(&h.pool).await?;
    let expected = i64::from(complete);
    assert_eq!(
        counts,
        (
            expected, expected, expected, expected, expected, expected, 0, 0
        )
    );
    Ok(())
}

async fn seed(h: &Harness, fragmented: bool) -> TestResult {
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=250 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).execute(&mut *fixture).await?;
    for (track, value) in TRACKS {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,'oteryn:quest.the_ice_islands_quest',$3,2)")
            .bind(id(41).as_slice()).bind(track).bind(value).execute(&mut *fixture).await?;
    }
    mint_fixture(&mut fixture, 200, "oteryn:item.tibia.i2854", 1).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(200).as_slice()).bind(id(42).as_slice()).bind(id(245).as_slice())
        .execute(&mut *fixture).await?;
    let entries = if fragmented {
        &[
            (201, TROUT, 25, 1),
            (202, TROUT, 25, 2),
            (203, TROUT, 25, 3),
            (204, TROUT, 25, 4),
            (205, PERCH, 100, 5),
        ][..]
    } else {
        &[
            (201, TROUT, 60, 1),
            (202, TROUT, 39, 2),
            (203, PERCH, 99, 3),
        ][..]
    };
    for &(item, key, count, ordinal) in entries {
        seed_entry(&mut fixture, item, key, count, ordinal).await?;
    }
    fixture.commit().await?;
    Ok(())
}

async fn receipt_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_npc_trades WHERE npc_key=$1")
            .bind(NPC)
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn mint_fixture(
    fixture: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    seed: u8,
    item: &str,
    count: u32,
) -> TestResult {
    sqlx::raw_sql(sqlx::AssertSqlSafe(
        crate::durability::item_fee_burn::test_support::item(seed, item, "definition-r1", count),
    ))
    .execute(&mut **fixture)
    .await?;
    Ok(())
}

async fn seed_entry(
    fixture: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    seed: u8,
    item: &str,
    count: u32,
    ordinal: i64,
) -> TestResult {
    mint_fixture(fixture, seed, item, count).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::bigint,encode($6,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(200).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice())
        .bind(ordinal).bind(id(246).as_slice()).execute(&mut **fixture).await?;
    Ok(())
}
async fn set_progress(h: &Harness, track: &str, value: i64) -> TestResult {
    let mut fixture = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *fixture)
        .await?;
    sqlx::query("UPDATE game_character_quest_tracks SET value=$3 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(id(41).as_slice()).bind(track).bind(value).execute(&mut *fixture).await?;
    fixture.commit().await?;
    Ok(())
}

async fn assert_inventory(h: &Harness, expected: [i64; 3]) -> TestResult {
    for (item, quantity) in [TROUT, PERCH, SHARD].into_iter().zip(expected) {
        let actual: i64 = sqlx::query_scalar(
            "SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i
             JOIN game_item_container_entries e USING(item_instance_id)
             WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.definition_production_key=$2",
        ).bind(id(41).as_slice()).bind(item).fetch_one(&h.pool).await?;
        assert_eq!(actual, quantity, "{item}");
    }
    Ok(())
}
