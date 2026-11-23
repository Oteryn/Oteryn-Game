//! The published ticket and return routes use the fenced travel owner.
use super::fixture::{Harness, debug, id, run};
use super::npcfix_m38_travel_guards_fixtures::NoPremium;
use super::npcfix_m38_travel_return_fixtures::{commit, complete, request};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::npc_catalogue::travel::TravelRoute;
use crate::durability::npc_travel::TravelRefusal;
use crate::foundation::{ChannelContentPin, MovementLocalPosition, WorldId};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use std::sync::Arc;

const TRACK: &str = "oteryn:quest-progress/travora/ticket_purchased";

#[test]
fn travora_ticket_and_last_port_survive_expiry_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("travora-ticket-and-return", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "travora_ticket", true, false).await?;
            let result = exercise_published_routes(&h).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise_published_routes(h: &Harness) -> TestResult {
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let book = NpcTalkBook::from_model(&catalogue.service_model_for_project().map_err(debug)?);
    let route = |npc: &str, keyword: &str| -> TestResult<TravelRoute> {
        book.get(npc)
            .and_then(|entry| {
                entry
                    .travel_routes
                    .iter()
                    .find(|route| route.keyword == keyword)
            })
            .cloned()
            .ok_or_else(|| format!("missing published route {npc}/{keyword}").into())
    };
    let inbound = route("oteryn:npc.petros", "travora")?;
    let greyhound = "oteryn:npc.captain_greyhound";
    let thais = route(greyhound, "thais")?;
    let darashia = route("oteryn:npc.captain_tiberius", "darashia")?;
    let ankrahmun = route("oteryn:npc.captain_tiberius", "ankrahmun")?;
    let quest_state = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let purchase = quest_state
        .catalogue()
        .transition("oteryn:quest-reward/travora/petros")
        .ok_or("published Petros ticket purchase")?;
    let callbacks =
        oteryn_gameplay_quests::quest::reward_callbacks::offered_for(quest_state.catalogue())
            .map_err(debug)?;
    let sale = callbacks
        .iter()
        .find(|callback| callback.key == purchase.key)
        .ok_or("published Petros offer")?;
    assert_eq!(sale.fee.as_ref().map(|fee| fee.gold), Some(1000));
    let [purchase_effect] = purchase.effects.as_slice() else {
        return Err("ticket purchase must write one track".into());
    };
    assert_eq!(purchase_effect.track, TRACK);
    assert_eq!(
        inbound
            .ticket
            .as_ref()
            .map(|ticket| ticket.valid_for_seconds),
        Some(86400)
    );
    assert_eq!(
        (
            inbound.route.destination.x,
            inbound.route.destination.y,
            inbound.route.destination.floor
        ),
        (32057, 32368, 6)
    );

    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, activated) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let room = activated.qualified_room();
    let channel_pin = ChannelContentPin::from_activation(
        world,
        activated.activation_sequence(),
        activated.identity().server_artifact_digest(),
        activated.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (33289, 32480, 6),
    );
    let deps = Dependencies::with_content_and_capacity(
        channel_pin,
        room.clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        3,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = controller.active();
    owner.quest_catalogue = Some(Arc::new(quest_state.catalogue().clone()));
    let session = deps.admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    assert_eq!(
        h.root
            .quote_npc_travel(&authority, &h.node, fence, inbound.clone(), &NoPremium)
            .await
            .map_err(debug)?,
        Err(TravelRefusal::UnavailableRoute)
    );

    sqlx::query("INSERT INTO game_type2_audit_activation VALUES(1,now())")
        .execute(&h.pool)
        .await?;
    super::guild_bank_fixtures::bank_seed::balance(h, id(40), id(41), 2000, 80).await?;
    owner.refresh_quest_session(session, true).await;
    let [captain, petros] = super::classic_quests_fixtures::install(
        &owner,
        &deps.admitted,
        [greyhound, "oteryn:npc.petros"],
    )
    .await?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let talk = super::classic_quests_fixtures::talk;
    talk(&mut owner, &deps.admitted, captain, 1, "hi").await?;
    talk(&mut owner, &deps.admitted, captain, 2, "ticket").await?;
    let armed = owner
        .states
        .npc_conversations
        .fixture_quest_topic(session)?;
    assert_ne!(armed, 0);
    let quote = talk(&mut owner, &deps.admitted, captain, 3, &thais.keyword).await?;
    let quoted_fare = format!("{} gold", thais.route.price);
    assert!(
        quote
            .iter()
            .any(|line| line.contains(&thais.keyword) && line.contains(&quoted_fare)),
        "{quote:?}"
    );
    let topic_after_quote = owner
        .states
        .npc_conversations
        .fixture_quest_topic(session)?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(60))
        .ok_or("clock")?;
    let expired = talk(&mut owner, &deps.admitted, captain, 4, "yes").await?;
    let (purchased, balance): (bool, i64) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE track_key=$1 AND value>0),(SELECT balance FROM game_account_bank_balances)"
    ).bind(TRACK).fetch_one(&h.pool).await?;
    assert_eq!(
        (
            topic_after_quote,
            purchased,
            balance,
            expired
                .iter()
                .any(|line| line.contains("offer has expired"))
        ),
        (0, false, 2000, true),
        "travel quote must replace ticket topic {armed}: {expired:?}"
    );
    talk(&mut owner, &deps.admitted, captain, 5, "bye").await?;
    talk(&mut owner, &deps.admitted, petros, 6, "hi").await?;
    let offer = talk(&mut owner, &deps.admitted, petros, 7, "ticket").await?;
    assert!(
        offer.iter().any(|line| line.contains("1000 gold")),
        "{offer:?}"
    );
    assert_eq!(
        owner
            .states
            .npc_conversations
            .fixture_quest_topic(session)?,
        sale.topic
    );
    talk(&mut owner, &deps.admitted, petros, 8, "no").await?;
    talk(&mut owner, &deps.admitted, petros, 9, "yes").await?;
    let (purchased, balance): (bool, i64) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE track_key=$1 AND value>0),(SELECT balance FROM game_account_bank_balances)"
    ).bind(TRACK).fetch_one(&h.pool).await?;
    assert_eq!(
        (purchased, balance),
        (false, 2000),
        "declined ticket must stay cancelled"
    );
    talk(&mut owner, &deps.admitted, petros, 10, "ticket").await?;
    talk(&mut owner, &deps.admitted, petros, 11, "yes").await?;
    let (stamp, balance): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),(SELECT balance FROM game_account_bank_balances)"
    ).bind(TRACK).fetch_one(&h.pool).await?;
    assert!(stamp >= now, "ticket records the database clock");
    assert_eq!(balance, 1000);
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    assert_eq!(
        h.root
            .quote_npc_travel(&authority, &h.node, fence, inbound.clone(), &NoPremium)
            .await
            .map_err(debug)?,
        Ok(0)
    );

    let mut passage = request(&owner, &deps, 48101, "petros").await?;
    passage.route = inbound.clone();
    passage.arrival.destination = crate::durability::character_last_position::LastPositionCell {
        x: inbound.route.destination.x,
        y: inbound.route.destination.y,
        floor: inbound.route.destination.floor,
    };
    commit(&owner, session, passage.clone()).await?;
    deps.runtime
        .lock()
        .await
        .place_admitted_respawn(
            deps.actor,
            session,
            Some(MovementLocalPosition {
                x: passage.arrival.destination.x,
                y: passage.arrival.destination.y,
                floor: passage.arrival.destination.floor,
            }),
            |_| true,
        )
        .map_err(debug)?;
    complete(&owner, &deps, passage.arrival.occurrence).await?;
    sqlx::query("UPDATE game_newhaven_travel_receipts SET arrived_at=clock_timestamp()-interval '4 seconds'")
        .execute(&h.pool).await?;
    // Age the purchased ticket in the fixture without changing the production clock.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let aged = sqlx::query(
        "UPDATE game_character_quest_tracks SET value=value-86400 WHERE track_key=$1 AND value=$2",
    )
    .bind(TRACK)
    .bind(stamp)
    .execute(&mut *tx)
    .await?;
    assert_eq!(aged.rows_affected(), 1);
    tx.commit().await?;

    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    for (route, expected) in [
        (inbound, Err(TravelRefusal::UnavailableRoute)),
        (ankrahmun, Err(TravelRefusal::UnavailableRoute)),
        (darashia, Ok(0)),
    ] {
        assert_eq!(
            h.root
                .quote_npc_travel(&authority, &h.node, fence, route, &NoPremium)
                .await
                .map_err(debug)?,
            expected
        );
    }
    Ok(())
}
