//! Production quest dialogue uses the current character, NPC, and quest owners.
use super::fixture::{Harness, debug, id, run};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult};
use crate::foundation::WorldId;
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const APE: &str = "oteryn:quest.the_ape_city_quest";
const APE_TRACK: &str = "oteryn:quest-progress/quest/u7_6/the_ape_city/";
const ROOK_TRACK: &str = "oteryn:quest-progress/quest/u8_2/the_beginning_quest/";

#[test]
fn carlos_food_sales_advance_the_saved_tutorial_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("carlos-food-sales", || {
        run(async |admin| {
            for (item, sell_all) in [("oteryn:item.tibia.i3577", false), ("oteryn:item.tibia.i3582", true)] {
                let h = Harness::create_with_admission_seed(admin.clone(), "carlos_food_sale", true, false).await?;
                let result = exercise(&h, Scenario::CarlosSale { item, sell_all }).await;
                h.cleanup().await?;
                result?;
            }
            Ok(())
        })
    })
}

#[test]
fn ape_city_and_beginning_production_npc_progress_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("ape-city-and-beginning", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "ape_city_beginning", true, false).await?;
            let result = exercise(&h, Scenario::Progress).await;
            h.cleanup().await?;
            result
        })
    })
}

#[test]
fn hairycles_quotes_all_three_portable_statue_kits_after_ape_city() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("hairycles-statue-kits", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "hairycles_statue_kits", true, false)
                    .await?;
            let result = exercise(&h, Scenario::HairyclesShop).await;
            h.cleanup().await?;
            result
        })
    })
}

enum Scenario<'a> {
    Progress,
    CarlosSale { item: &'a str, sell_all: bool },
    HairyclesShop,
}

async fn exercise(h: &Harness, scenario: Scenario<'_>) -> TestResult {
    super::prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
    let deps = Dependencies::with_content_and_capacity(
        pin.into_channel_pin(),
        crate::content::qualify_native_entry_room(world).map_err(debug)?,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        7,
    )?;
    deps.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(controller.active().ok_or("active generation")?);
    owner.quest_catalogue = Some(Arc::new(
        load_embedded_quest_state("content-1").map_err(debug)?.catalogue().clone(),
    ));
    let mut admitted = deps.admitted;
    if matches!(scenario, Scenario::CarlosSale { .. }) {
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        let fence = owner.current_quest_fence(admitted.game_session_id)
            .await.map_err(|()| "sale item fence lookup")?.ok_or("sale item fence")?;
        admitted.item_fence = Some(fence.item_fence());
        owner.runtime.lock().await.bind_attacker_lease(
            deps.actor, admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            ).map_err(debug)?,
        ).map_err(debug)?;
        deps.initialize(&owner).await;
    }
    owner.refresh_quest_session(admitted.game_session_id, true).await;
    let [santiago, zirella, carlos, hairycles] = super::classic_quests_fixtures::install_with_offsets(
        &owner,
        &admitted,
        [
            "oteryn:npc.santiago", "oteryn:npc.zirella",
            "oteryn:npc.carlos", "oteryn:npc.hairycles",
        ],
        Some([(0, 1), (1, 1), (0, 2), (1, 2)]),
    ).await?;
    if let Scenario::CarlosSale { item, sell_all } = scenario {
        use crate::gameplay_transport::capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities};
        use oteryn_protocol_oteryn::{npc_service::CAPABILITY_NPC_SERVICE_V1, npc_shop::CAPABILITY_NPC_SHOP_V1};
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[CAPABILITY_NPC_SERVICE_V1, CAPABILITY_NPC_SHOP_V1],
        ).ok_or("food shop capabilities")?;
        return carlos_sale(h, &mut owner, &mut admitted, carlos, item, sell_all).await;
    }
    super::qd_world_fixtures::select_talk_capabilities(&owner, &mut admitted, &[3, 6, 25])?;
    if matches!(scenario, Scenario::HairyclesShop) {
        return hairycles_statue_kits(h, &mut owner, &mut admitted, hairycles).await;
    }
    let mut command = 1000;
    for (npc, phrases, track, value) in [
        (santiago, &["hi", "yes", "yes", "bye"][..], format!("{ROOK_TRACK}santiago_quest_log"), 2),
        (zirella, &["hi", "quest", "yes", "yes", "yes", "yes", "yes", "yes", "bye"][..], format!("{ROOK_TRACK}zirella_quest_log"), 6),
        (carlos, &["hi", "yes", "hi", "help", "yes", "yes", "bye"][..], format!("{ROOK_TRACK}carlos_quest_log"), 5),
        (hairycles, &["hi", "mission", "yes", "yes"][..], format!("{APE_TRACK}dworc_door"), 1),
    ] {
        conversation(&mut owner, &mut admitted, npc, &mut command, phrases).await?;
        assert_eq!(progress(h, &track).await?, value, "{track}");
    }

    seed_monument(h).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    conversation(&mut owner, &mut admitted, hairycles, &mut command, &["mission"]).await?;
    assert_eq!(progress(h, &format!("{APE_TRACK}questline")).await?, 18);
    let complete: bool = sqlx::query_scalar(
        "SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE quest_key=$1",
    ).bind(APE).fetch_one(&h.pool).await?;
    assert!(complete);
    let awarded: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
    )
    .bind("oteryn:achievement/friend_of_the_apes")
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(awarded, 1);
    conversation(&mut owner, &mut admitted, hairycles, &mut command, &["mission"]).await?;
    assert_eq!(progress(h, &format!("{APE_TRACK}questline")).await?, 18);
    Ok(())
}

async fn hairycles_statue_kits(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
) -> TestResult {
    use super::qengine_rewards_fixtures::talk_outcome;
    use crate::gameplay_transport::furniture_catalog::construction_kit_to;
    use oteryn_protocol_oteryn::npc_service::NpcTradeSide;
    use std::num::NonZeroU32;

    assert_eq!(
        talk_outcome(owner, admitted, npc, 1, "hi")
            .await?
            .disposition,
        NpcIntentDisposition::Ok
    );
    let locked = talk_outcome(owner, admitted, npc, 2, "trade").await?;
    assert!(
        locked.window.is_none_or(|window| window.offers.is_empty()),
        "stage zero stock"
    );
    let track = format!("{APE_TRACK}questline");
    let hash = owner
        .quest_catalogue
        .as_ref()
        .and_then(|catalogue| catalogue.definition_hash(APE))
        .ok_or("Ape City definition")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_character_quest_states
         (character_id, quest_key, pinned_content_revision, definition_hash, started_character_revision,
          completed_character_revision, committed_character_revision)
         VALUES (encode($1,'hex')::uuid, $2, 'content-1', $3, 2, 2, 2)",
    ).bind(id(41).as_slice()).bind(APE).bind(hash.as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_character_quest_tracks
         (character_id, track_key, quest_key, value, committed_character_revision)
         VALUES (encode($1,'hex')::uuid, $2, $3, 18, 2)",
    )
    .bind(id(41).as_slice())
    .bind(&track)
    .bind(APE)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let response = talk_outcome(owner, admitted, npc, 3, "trade").await?;
    assert_eq!(response.disposition, NpcIntentDisposition::Ok);
    let window = response
        .window
        .filter(|window| window.open.is_some())
        .ok_or("Hairycles shop window")?;
    let runtime = owner.runtime.lock().await;
    let index = crate::gameplay_transport::item_ref_admission::pinned_index(
        &runtime,
        owner.active_generation,
    )
    .ok_or("Hairycles item index")?;
    let mut kits: Vec<_> = window
        .offers
        .iter()
        .filter(|offer| offer.side == NpcTradeSide::Buy && offer.unit_price == 65)
        .filter_map(|offer| index.definition_at(NonZeroU32::new(offer.item_definition_ref)?))
        .map(|(key, _)| key)
        .collect();
    kits.sort_unstable();
    assert_eq!(
        kits,
        [
            "oteryn:item.tibia.i5086",
            "oteryn:item.tibia.i5087",
            "oteryn:item.tibia.i5088"
        ]
    );
    for (kit, statue) in [(5086, 5046), (5087, 5055), (5088, 5056)] {
        assert_eq!(construction_kit_to(kit), Some(statue));
    }
    eprintln!("POSTGRES_CASE_COMPLETED hairycles_statue_kits");
    Ok(())
}

async fn carlos_sale(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    item: &str,
    sell_all: bool,
) -> TestResult {
    use oteryn_protocol_oteryn::npc_service as npc_wire;
    use oteryn_protocol_oteryn::npc_shop as shop_wire;

    super::item_look_fixtures::seed_main_backpack(h, &[(item, 1)]).await?;
    let mut dialogue_command = 1000;
    conversation(owner, admitted, npc, &mut dialogue_command,
        &["hi", "yes", "hi", "help", "yes", "yes", "bye"]).await?;
    let greet = format!("{ROOK_TRACK}carlos_npc_greet_storage");
    let log = format!("{ROOK_TRACK}carlos_quest_log");
    conversation(owner, admitted, npc, &mut dialogue_command, &["hi", "yes"]).await?;
    assert_eq!(progress(h, &greet).await?, 6, "the tutorial unlocks the food shop");
    let book = owner.states.chat.npc_talk().ok_or("NPC book")?;
    let carlos = book.get("oteryn:npc.carlos").ok_or("Carlos")?;
    let (index, price) = {
        let runtime = owner.runtime.lock().await;
        let items = crate::gameplay_transport::item_ref_admission::pinned_index(&runtime, owner.active_generation)
            .ok_or("item index")?;
        let (index, (shown, _)) = oteryn_gameplay_npc::npc_command::projected(carlos.offers(), items)
            .enumerate().find(|(_, (_, source))| source.item.key == item).ok_or("food offer")?;
        (u32::try_from(index)?, shown.unit_price)
    };
    let talk = |text: &str| npc_wire::encode_npc_talk_intent(&npc_wire::NpcTalkIntent {
        npc_actor: npc, text: text.to_owned(),
    }).map_err(debug);
    let (kind, payload) = if sell_all {
        (shop_wire::COMMAND_TYPE_NPC_SHOP, shop_wire::encode_npc_shop_intent(&shop_wire::NpcShopIntent {
            npc_actor: npc, catalogue_revision: book.catalogue_revision(), offer_index: index,
            side: npc_wire::NpcTradeSide::Sell, action: shop_wire::NpcShopAction::SellAll,
            expected_unit_price: price,
        }).map_err(debug)?)
    } else {
        (npc_wire::COMMAND_TYPE_NPC_TRADE_INTENT, npc_wire::encode_npc_trade_intent(&npc_wire::NpcTradeIntent {
            npc_actor: npc, catalogue_revision: book.catalogue_revision(), offer_index: index,
            side: npc_wire::NpcTradeSide::Sell, quantity: 1, expected_unit_price: price,
        }).map_err(debug)?)
    };
    owner.clock_origin -= std::time::Duration::from_secs(3);
    let replies = super::questl2_fixture::wire_sequence(
        owner, admitted, vec![(1, 7, talk("hi")?), (2, 7, talk("trade")?), (3, kind, payload)],
    ).await?;
    for (reply, text) in replies.iter().zip(["hi", "trade"]) {
        assert_eq!(npc_wire::decode_npc_intent_result(reply)
            .map_err(|error| format!("Carlos {text} result {reply:?}: {error:?}"))?, NpcIntentDisposition::Ok);
    }
    let sale = replies.get(2).ok_or("sale response")?;
    let disposal = if sell_all {
        shop_wire::decode_npc_shop_result(sale)
            .map_err(|error| format!("Carlos sell-all result {sale:?}: {error:?}"))?.disposition
    } else {
        npc_wire::decode_npc_intent_result(sale)
            .map_err(|error| format!("Carlos sale result {sale:?}: {error:?}"))?
    };
    assert_eq!(disposal, NpcIntentDisposition::Ok, "{item}");
    assert_eq!(progress(h, &greet).await?, 7, "first successful sale of {item}");
    if sell_all {
        owner.states.npc_conversations.forget(admitted.game_session_id);
        owner.refresh_quest_session(admitted.game_session_id, false).await;
        conversation(owner, admitted, npc, &mut dialogue_command, &["hi"]).await?;
    } else {
        conversation(owner, admitted, npc, &mut dialogue_command, &["ready"]).await?;
    }
    assert_eq!(progress(h, &greet).await?, 8, "returning to Carlos completes the tutorial");
    assert_eq!(progress(h, &log).await?, 7);
    Ok(())
}

async fn conversation(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    command: &mut u64,
    phrases: &[&str],
) -> TestResult {
    for phrase in phrases {
        *command += 1;
        let outcome = super::qengine_rewards_fixtures::talk(owner, admitted, npc, *command, phrase).await?;
        if *phrase != "bye" {
            assert_eq!(outcome, NpcIntentDisposition::Ok, "{phrase}");
        }
    }
    Ok(())
}

async fn progress(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    ).bind(track).fetch_one(&h.pool).await?)
}

// Prepare the persisted outcome of the monument USE. The actual report must
// read this revision through the production session and grant its own receipt.
async fn seed_monument(h: &Harness) -> TestResult {
    let mut transaction = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *transaction)
        .await?;
    let stage = format!("{APE_TRACK}questline");
    let revision: String = sqlx::query_scalar(
        "UPDATE game_character_quest_tracks SET value=17 WHERE track_key=$1 AND value=1
         RETURNING committed_character_revision::text",
    )
    .bind(stage)
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_quest_tracks
         (character_id, track_key, quest_key, value, committed_character_revision)
         VALUES (encode($1,'hex')::uuid,$2,$3,1,$4::text::numeric)",
    )
    .bind(id(41).as_slice())
    .bind(format!("{APE_TRACK}snake_destroyer"))
    .bind(APE)
    .bind(revision)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}
