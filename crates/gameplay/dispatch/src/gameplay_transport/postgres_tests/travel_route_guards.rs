//! NPC_TALK through the paid-travel receipt, fare and arrival path.
use super::fixture::{Harness, debug, id, run};
use super::npc_paid_travel_fixtures::{talk, travel_world};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use sqlx::types::Uuid;
use std::sync::Arc;

#[test]
fn travel_route_guards_scarab_cheese_in_a_closed_bag_refuses_confirmation_without_charging()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Passage::Sinbeard)
}

#[test]
fn travel_route_guards_petros_rechecks_cheese_acquired_after_the_quote() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Passage::Petros)
}

#[test]
fn travel_route_guards_meluna_requires_a_current_wedding_within_24_hours() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Passage::Meluna)
}

#[test]
fn travel_route_guards_fibula_return_allows_an_unmarried_passenger() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, Passage::Fibula)
}

#[derive(Clone, Copy)]
enum Passage {
    Sinbeard,
    Petros,
    Meluna,
    Fibula,
}

impl Passage {
    fn checks_cheese(self) -> bool {
        matches!(self, Self::Sinbeard | Self::Petros)
    }

    fn route(self) -> (&'static str, &'static str) {
        match self {
            Self::Sinbeard => ("oteryn:npc.captain_sinbeard", "darashia"),
            Self::Petros => ("oteryn:npc.petros", "issavi"),
            Self::Meluna => ("oteryn:npc.ferryman_kamil", "meluna"),
            Self::Fibula => ("oteryn:npc.ferryman_kamil_meluna", "fibula"),
        }
    }
}

fn run_case(admin: String, passage: Passage) -> TestResult {
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            run(async move |_| scenario(admin, passage).await).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "travel route guards PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED TRAVEL_ROUTE_GUARDS {} duration={:?}",
        passage.route().1,
        started.elapsed()
    );
    Ok(())
}

async fn scenario(admin: String, passage: Passage) -> TestResult {
    reset();
    let h = Harness::create_with_admission_seed(admin, "travel_route_guards", true, false).await?;
    let result = exercise(&h, passage).await;
    h.cleanup().await?;
    result
}

async fn exercise(h: &Harness, passage: Passage) -> TestResult {
    prepare_current_session(h).await?;
    if matches!(passage, Passage::Meluna) {
        let now = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
        crate::world_party_runtime_postgres_cases::committed_fresh_player_owned(
            &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                root: h.root.clone(),
                node: h.node.clone(),
                recovery: &h.recovery,
            },
            60,
            63,
            171,
            now,
        )
        .await?;
    }

    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("active content")?;
    let (npc_key, keyword) = passage.route();
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    let book = NpcTalkBook::from_model(&model);
    let mut entry = book.get(npc_key).ok_or("authored travel NPC")?.clone();
    let route = entry
        .travel_routes
        .iter_mut()
        .find(|route| route.keyword == keyword)
        .ok_or("authored travel route")?;
    // The content lane owns admission. Exercise the engine against the pinned route.
    route.held = false;
    let destination = route.route.destination.clone();
    let fare = i64::try_from(route.route.price)?;
    let bundle = travel_world(
        world,
        &[(
            u16::try_from(destination.x)?,
            u16::try_from(destination.y)?,
            -i8::try_from(destination.floor)?,
        )],
    )?;
    let room = pin.qualified_room().clone();
    let cells = bundle
        .movement_cells(room.movement_cells())
        .map_err(debug)?;
    let deps = Dependencies::with_content_and_capacity(
        ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            bundle.frame_binding_digest(world),
            bundle.map_revision_digest(),
            (32310, 32210, 6),
        ),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        2,
    )?;
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let coordinator = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.premium_coordinator = Some(&coordinator);
    let session = deps.admitted.game_session_id;
    owner.admit_premium(id(40), session);
    deps.initialize(&owner).await;
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(book.with(npc_key, entry)));
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
        let installed = runtime
            .place_npcs(
                &NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: npc_key.into(),
                        placements: vec![NpcPlacement {
                            x: 32311,
                            y: 32210,
                            floor: -6,
                            direction: MovementFacing::South,
                        }],
                    }],
                },
                |_| true,
                true,
            )
            .map_err(debug)?;
        let npc = installed.actors.first().ok_or("placed travel NPC")?;
        EntityRef {
            identity: npc.placement_identity(),
            generation: npc.actor_local_generation(),
        }
    };

    if matches!(passage, Passage::Sinbeard) {
        seed_cheese(h).await?;
    }
    talk(&owner, &deps, npc, 1, "hi").await?;
    let opening = talk(&owner, &deps, npc, 2, keyword).await?;
    match passage {
        Passage::Sinbeard | Passage::Petros => {
            assert!(line(&opening).contains(&format!("{fare} gold")));
            if matches!(passage, Passage::Petros) {
                seed_cheese(h).await?;
            }
            let refused = talk(&owner, &deps, npc, 3, "yes").await?;
            let expected = if matches!(passage, Passage::Sinbeard) {
                "Scarab cheese? You smell like dragon breath! Get off my boat at once!"
            } else {
                "That cheese of yours stinks! Please leave!"
            };
            assert_eq!(line(&refused), expected);
            assert!(!refused.inventory_changed);
            assert_eq!(travel_writes(h).await?, (0, 0, 0));
            assert_eq!(cheese_quantity(h).await?, 3);
            move_cheese_bag_to_inbox(h).await?;
        }
        Passage::Meluna => {
            assert_eq!(line(&opening), "Sorry, not possible.");
            assert_eq!(travel_writes(h).await?, (0, 0, 0));
            seed_wedding(h).await?;
            owner.clock_origin -= std::time::Duration::from_secs(1);
            let quoted = talk(&owner, &deps, npc, 3, keyword).await?;
            assert!(line(&quoted).contains("0 gold"));
            set_wedding_age(h, 86_400_000).await?;
            let refused = talk(&owner, &deps, npc, 4, "yes").await?;
            assert_eq!(line(&refused), "Sorry, not possible.");
            assert_eq!(travel_writes(h).await?, (0, 0, 0));
            set_wedding_age(h, 60_000).await?;
        }
        Passage::Fibula => assert!(line(&opening).contains("0 gold")),
    }
    if !matches!(passage, Passage::Fibula) {
        owner.clock_origin -= std::time::Duration::from_secs(1);
        let quote = talk(&owner, &deps, npc, 5, keyword).await?;
        assert!(line(&quote).contains(&format!("{fare} gold")));
    }
    let travelled = talk(&owner, &deps, npc, 6, "yes").await?;
    assert_eq!(line(&travelled), "Set the sails!");
    assert_eq!(travelled.inventory_changed, fare > 0);
    let position = deps
        .runtime
        .lock()
        .await
        .read_actor_position(deps.actor)
        .map_err(debug)?
        .position();
    assert_eq!(
        (position.x, position.y, position.floor),
        (destination.x, destination.y, destination.floor)
    );
    assert_eq!(travel_writes(h).await?, (1, i64::from(fare > 0), 0));
    if passage.checks_cheese() {
        assert_eq!(cheese_quantity(h).await?, 3);
    }
    Ok(())
}

fn line(outcome: &crate::gameplay_transport::npc_command::NpcTalkOutcome) -> String {
    outcome
        .conversation
        .as_ref()
        .map(|conversation| conversation.lines.join(" "))
        .unwrap_or_default()
}

async fn travel_writes(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_newhaven_travel_receipts), (SELECT count(*) FROM game_item_fee_burns), (SELECT count(*) FROM game_character_pending_arrivals)")
        .fetch_one(&h.pool).await?)
}

async fn cheese_quantity(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=$1 AND lifecycle=1",
    )
    .bind(Uuid::from_bytes(id(73)))
    .fetch_one(&h.pool)
    .await?)
}

async fn seed_cheese(h: &Harness) -> TestResult {
    super::item_look_fixtures::seed_main_backpack(
        h,
        &[
            ("oteryn:item.tibia.i3035", 2),
            ("oteryn:item.tibia.i2854", 1),
            ("oteryn:item.tibia.i169", 3),
        ],
    )
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::raw_sql("SET LOCAL session_replication_role=replica; INSERT INTO game_type2_audit_activation VALUES(1,now())")
        .execute(&mut *tx).await?;
    sqlx::query("WITH cheese AS (DELETE FROM game_item_container_entries WHERE item_instance_id=$1 RETURNING *)
        INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)
        SELECT item_instance_id,world_id,$2,1,placed_transaction_id FROM cheese")
        .bind(Uuid::from_bytes(id(73))).bind(Uuid::from_bytes(id(72))).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn move_cheese_bag_to_inbox(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("WITH bag AS (DELETE FROM game_item_container_entries WHERE item_instance_id=$1 RETURNING *)
        INSERT INTO game_item_character_inbox_locations(character_id,item_instance_id,world_id,ordinal)
        SELECT character_id,item_instance_id,world_id,1 FROM bag")
        .bind(Uuid::from_bytes(id(72))).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn seed_wedding(h: &Harness) -> TestResult {
    let actor = Uuid::from_bytes(id(41));
    let partner = Uuid::from_bytes(id(60));
    let receipt = Uuid::from_bytes(id(140));
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_marriage_receipts(
        purchase_occurrence_id, character_id, account_id, command_session_id, command_id, request_binding,
        npc_key, npc_identity, npc_generation, original_character_revision, committed_character_revision,
        level_before, level_after, experience_before, experience_after, profile_revision, ruleset_revision,
        content_revision, simulation_revision, evidence_revision, declaration_revision, policy_revision,
        reward_revision, committed_at, partner_id, operation, wire_binding, actor_before,
        actor_after, partner_before, partner_after, reply, world_id)
        SELECT $3, s.character_id, r.account_id, $4, 1, sha256(uuid_send($3)),
        'oteryn:npc.lynda', uuid_send($3), 1, s.character_revision, s.character_revision + 1,
        s.level, s.level, s.total_experience, s.total_experience, s.profile_revision, s.ruleset_revision,
        s.content_revision, s.simulation_revision, s.evidence_revision, s.declaration_revision, s.policy_revision,
        s.reward_revision, (extract(epoch FROM statement_timestamp()) * 1000)::bigint - 60000, $2,
        4, sha256(uuid_send($3)), game_marriage_after('null', $1, $2, 3, $3),
        game_marriage_after(game_marriage_after('null', $1, $2, 3, $3), $1, $2, 2, $3),
        game_marriage_after('null', $2, $1, 3, $3),
        game_marriage_after(game_marriage_after('null', $2, $1, 3, $3), $2, $1, 2, $3),
        'You are now married. Take your engraved wedding ring.', r.world_id
        FROM game_character_progression_state s JOIN game_character_roots r USING(character_id)
        WHERE s.character_id=$1")
        .bind(actor).bind(partner).bind(receipt).bind(Uuid::from_bytes(id(50)))
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_marriages
        SELECT character_id, partner_id, 2, 2, $3 FROM (VALUES ($1::uuid,$2::uuid),($2,$1)) AS pair(character_id,partner_id)")
        .bind(actor).bind(partner).bind(receipt).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn set_wedding_age(h: &Harness, age_ms: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_character_marriage_receipts
        SET committed_at=(extract(epoch FROM statement_timestamp()) * 1000)::bigint - $1
        WHERE operation=4",
    )
    .bind(age_ms)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
