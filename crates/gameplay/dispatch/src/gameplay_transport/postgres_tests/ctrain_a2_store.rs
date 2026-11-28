use super::fixture::{Harness, debug, fence, id, run};
use super::store_fixtures::{FakePlatform, Ledger, purchase, purchase_request, serve, uuid};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, FreshAdmissionAuthority,
};
use crate::gameplay_transport::{ComposedFreshAdmission, character_gameplay_fence};
use crate::native_admission_source::store::StoreClient;
use oteryn_protocol_oteryn::store as wire;
use std::sync::{Arc, Mutex};

#[test]
fn ctrain_a2_coin_mount_belongs_to_the_buying_character() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("coin-mount-character", || {
        run(cosmetic_case)
    })
}

#[test]
fn ctrain_a2_red_skull_preserves_claimed_store_item_and_backpack() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-backpack-death", || {
        run(async |admin| death_case(admin, false, false).await)
    })
}

#[test]
fn ctrain_a2_reconnect_finishes_legacy_store_backpack_loss() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("store-backpack-reconnect", || {
        run(async |admin| death_case(admin, true, false).await)
    })
}

#[test]
fn ctrain_a2_reconnect_drops_loot_beside_nested_store_claim() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("nested-store-backpack-reconnect", || {
        run(async |admin| death_case(admin, true, true).await)
    })
}

async fn open_store(
    h: &Harness,
    deps: &Dependencies,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    advertise_claims: bool,
) -> TestResult<AdmittedSession> {
    let mut admitted = deps.admitted;
    let ledger = Ledger {
        balance: 1_000,
        world: uuid(admitted.world_id.as_bytes()),
        advertise_claims,
        ..Ledger::default()
    };
    owner.store = Some(Arc::new(tokio::sync::Mutex::new(StoreClient::new(
        Box::new(FakePlatform(Arc::new(Mutex::new(ledger)))),
        "oteryn-game/store-consumer".into(),
    ))));
    deps.initialize(owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let (session, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(admitted.game_session_id)
        .await?;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = crate::gameplay_transport::item_fence_of(session);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[wire::CAPABILITY_STORE_V1])
            .ok_or("Store capability")?;
    Ok(admitted)
}

async fn cosmetic_case(admin: String) -> TestResult {
    use crate::domain::appearance::{
        AccountUnlocks, AppearanceContent, EarnedCosmetics, UnlockFact,
    };
    const OFFER: &str = "store:mount:armoured-war-horse";

    let h = Harness::create_appearance(admin, "ctrain_a2_coin_mount").await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let (generation, _) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
        deps.admitted.world_id,
        None,
    );
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = generation.active();
    let mut admitted = open_store(&h, &deps, &mut owner, false).await?;
    let offer = crate::content::store_cosmetics::offer(OFFER).ok_or("mount offer")?;
    let purchase = purchase(
        &owner,
        &mut admitted,
        purchase_request(80, OFFER, offer.price)?,
    )
    .await?;
    assert_eq!(purchase.code, wire::StorePurchaseCode::Accepted);

    h.seed_appearance_observer().await?;
    super::appearance_change_postgres_fixtures::observer_visibility_tests::admit(&h).await?;
    let sibling_session = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
    let buyer = character_gameplay_fence::current(&owner, admitted.game_session_id, false)
        .await
        .ok_or("buyer fence")?;
    let sibling = character_gameplay_fence::current(&owner, sibling_session, false)
        .await
        .ok_or("sibling fence")?;

    // Model the post-transfer world mismatch in the receipt because the
    // world-transfer command is unsupported by this server.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE store_delivery_receipts SET world_id=$1 WHERE operation_id=$2")
        .bind(sqlx::types::Uuid::from_bytes(id(94)))
        .bind(sqlx::types::Uuid::from_bytes(
            purchase.operation_id.ok_or("delivery ID")?,
        ))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let content = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.cosmetics())
        .ok_or("cosmetics content")?;
    let read_mount = async |fence| -> Result<_, Box<dyn std::error::Error>> {
        let state = h
            .root
            .read_current_character_appearance_state(
                &authority,
                &h.node,
                fence,
                content.earned_grant_keys(),
            )
            .await
            .map_err(debug)?;
        Ok(EarnedCosmetics {
            content,
            facts: &state.earned,
        }
        .mount(&offer.unlock_key))
    };
    assert_eq!(
        read_mount(buyer).await?,
        UnlockFact::Compatible,
        "world change keeps the paid mount"
    );

    // Set up the second login on the buyer's Account. Only one current session
    // may hold the Account fence, so the buyer leaves before the sibling reads.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE game_session_id=$1",
    )
    .bind(sqlx::types::Uuid::from_bytes(id(50)))
    .execute(&mut *tx)
    .await?;
    let account = sqlx::types::Uuid::from_bytes(id(40));
    let character = sqlx::types::Uuid::from_bytes(id(101));
    let session = sqlx::types::Uuid::from_bytes(id(110));
    for table_update in [
        "UPDATE game_character_roots SET account_id=$1 WHERE character_id=$2",
        "UPDATE game_durability_admission_character_guards SET account_id=$1 WHERE character_id=$2",
        "UPDATE game_durability_reconnect_sessions SET account_id=$1 WHERE character_id=$2",
    ] {
        sqlx::query(table_update)
            .bind(account)
            .bind(character)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE game_durability_admission_account_guards SET presence_character_id=$2,holder_game_session_id=$3 WHERE account_id=$1")
        .bind(account).bind(character).bind(session).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(
        read_mount(sibling).await?,
        UnlockFact::Absent,
        "paid mount belongs to the buying Character"
    );
    eprintln!("CTRAIN_A2_POSTGRES_EXECUTED: paid mount and sibling appearance");
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn death_case(admin: String, legacy: bool, nested: bool) -> TestResult {
    let h =
        Harness::create_with_admission_seed(admin, "ctrain_a2_store_death", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let (generation, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            deps.admitted.world_id,
            None,
        );
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = generation.active();
    let mut admitted = open_store(&h, &deps, &mut owner, true).await?;
    let claimed = claim_sword(&h, &owner, &mut admitted, &pin, nested).await?;
    if nested {
        nest_claimed_sword(&h, claimed).await?;
    }
    prepare_red_skull(&h, &authority).await?;
    let current = character_gameplay_fence::current(&owner, admitted.game_session_id, false)
        .await
        .ok_or("death fence")?;
    let request = crate::gameplay_transport::death_ml_loss_postgres::death(90)?;
    let death = if legacy {
        // Reproduce the retained loss from before binding backfill. Use the
        // real death writer so the original selection and binding stay intact.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM game_item_bindings WHERE item_instance_id=$1")
            .bind(claimed)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        let root = owner
            .runtime
            .lock()
            .await
            .content_pin()
            .server_artifact_digest();
        let death = h
            .root
            .settle_runtime_player_death_with_items(
                &owner.revision_sequencer,
                &authority,
                &h.node,
                current,
                request,
                None,
                None,
                Some(root),
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            death.items.as_ref().ok_or("legacy death selection")?.lost,
            [id(70)]
        );
        sqlx::query(
            "INSERT INTO game_item_bindings(item_instance_id,world_id,character_id,cause)
                     SELECT item_instance_id,world_id,character_id,'STORE_PURCHASE'
                     FROM store_delivery_receipts WHERE item_instance_id=$1",
        )
        .bind(claimed)
        .execute(&h.pool)
        .await?;
        death
    } else {
        let death = owner
            .settle_source_player_death(current, request, None, None)
            .await
            .map_err(|error| format!("red-skull Store backpack settlement failed: {error:?}"))?;
        // P1: One bound item protects every unbound item in the same
        // backpack from death loss. The root selection now drops its unbound branches.
        assert_eq!(
            death.items.as_ref().ok_or("death selection")?.lost,
            [id(70)]
        );
        death
    };
    let after = crate::durability::character_progression::CurrentCharacterGameplayFence {
        expected_character_revision: death.committed_character_revision,
        ..current
    };

    let items_complete: bool = sqlx::query_scalar("SELECT game_player_death_items_complete($1)")
        .bind(sqlx::types::Uuid::from_bytes(*death.occurrence.as_bytes()))
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        items_complete, !legacy,
        "pending death must require lost items to move to the corpse"
    );

    let restarted = Dependencies::new()?;
    let reconnect = restarted.owner(&h, &authority);
    let permit = reconnect
        .spell_lane_permit()
        .await
        .ok_or("reconnect lane")?;
    reconnect
        .resume_pending_player_death_items(&permit, after)
        .await
        .map_err(|error| {
            format!("legacy Store backpack settlement failed on reconnect: {error:?}")
        })?;
    drop(permit);
    h.root
        .consume_pending_respawn(&authority, &h.node, after, Some(death.occurrence))
        .await
        .map_err(debug)?
        .ok_or("pending respawn")?;

    let parent: sqlx::types::Uuid = sqlx::query_scalar(if nested {
        "SELECT n.parent_item_instance_id FROM game_item_nested_entries n
         JOIN game_item_container_entries e ON e.item_instance_id=n.parent_item_instance_id
         JOIN game_item_container_slots s ON s.item_instance_id=e.parent_item_instance_id
         JOIN game_item_bindings b ON b.item_instance_id=n.item_instance_id
         WHERE n.item_instance_id=$1 AND e.character_id=$2 AND s.character_id=$2
           AND b.character_id=$2 AND b.cause='STORE_PURCHASE'"
    } else {
        "SELECT e.parent_item_instance_id FROM game_item_container_entries e
         JOIN game_item_container_slots s ON s.item_instance_id=e.parent_item_instance_id
         JOIN game_item_bindings b ON b.item_instance_id=e.item_instance_id
         WHERE e.item_instance_id=$1 AND e.character_id=$2 AND s.character_id=$2
           AND b.character_id=$2 AND b.cause='STORE_PURCHASE'"
    })
    .bind(claimed)
    .bind(sqlx::types::Uuid::from_bytes(id(41)))
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(*parent.as_bytes(), id(if nested { 72 } else { 70 }));
    let ordinary_dropped: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e
         JOIN game_item_ground_locations g ON g.item_instance_id=e.parent_item_instance_id
         WHERE e.item_instance_id=$1 AND g.native_room_placement_context=$2)",
    )
    .bind(sqlx::types::Uuid::from_bytes(id(71)))
    .bind(death.occurrence.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert!(
        ordinary_dropped,
        "unbound backpack sibling must drop (legacy={legacy})"
    );
    if nested {
        let nested_dropped: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e
             JOIN game_item_ground_locations g ON g.item_instance_id=e.parent_item_instance_id
             WHERE e.item_instance_id=$1 AND g.native_room_placement_context=$2)",
        )
        .bind(sqlx::types::Uuid::from_bytes(id(73)))
        .bind(death.occurrence.as_bytes().as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert!(
            nested_dropped,
            "unbound sibling inside retained bag must drop"
        );
    }
    eprintln!(
        "CTRAIN_A2_POSTGRES_EXECUTED: Store backpack survives death and reconnect, legacy={legacy}"
    );
    drop(reconnect);
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn claim_sword(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    pin: &crate::content::NativeEntryContentPin,
    nested: bool,
) -> TestResult<sqlx::types::Uuid> {
    let bought = purchase(
        owner,
        admitted,
        purchase_request(80, "store:exercise-sword", 25)?,
    )
    .await?;
    assert_eq!(bought.code, wire::StorePurchaseCode::Accepted);
    let operation = bought.operation_id.ok_or("Store delivery ID")?;
    let entries = [
        ("oteryn:item.tibia.i3031", 1),
        ("oteryn:item.tibia.i2854", 1),
    ];
    super::item_look_fixtures::seed_main_backpack(h, &entries[..if nested { 2 } else { 1 }])
        .await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4) ON CONFLICT DO NOTHING")
        .bind(admitted.world_id.as_bytes().as_slice())
        .bind(admitted.channel_id.as_bytes().as_slice())
        .execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(admitted.world_id.as_bytes().as_slice())
        .bind(admitted.channel_id.as_bytes().as_slice())
        .bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice())
        .bind(pin.frame_binding().digest().as_slice())
        .execute(&h.pool).await?;
    let request = wire::StoreInboxClaimRequest {
        fulfilment_unit_id: uuid(&operation),
    };
    let reply = serve(
        owner,
        admitted,
        &[(
            wire::COMMAND_TYPE_STORE_INBOX_CLAIM,
            wire::encode_inbox_claim_request(&request).map_err(debug)?,
        )],
    )
    .await?;
    let (status, payload) = reply.first().ok_or("Store claim reply")?;
    assert_eq!(*status, crate::foundation::CommandStatus::Accepted);
    assert_eq!(
        wire::decode_claim_receipt(payload).map_err(debug)?.result,
        wire::StoreClaimCode::Committed
    );
    Ok(sqlx::query_scalar(
        "SELECT r.item_instance_id FROM store_delivery_receipts r
         JOIN game_item_container_entries e USING (item_instance_id)
         JOIN game_item_bindings b USING (item_instance_id)
         WHERE r.operation_id=$1 AND e.parent_item_instance_id=$2 AND b.cause='STORE_PURCHASE'",
    )
    .bind(sqlx::types::Uuid::from_bytes(operation))
    .bind(sqlx::types::Uuid::from_bytes(id(70)))
    .fetch_one(&h.pool)
    .await?)
}

async fn nest_claimed_sword(h: &Harness, claimed: sqlx::types::Uuid) -> TestResult {
    // Seed a nested layout after the real claim has established Store custody.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    let bag = sqlx::types::Uuid::from_bytes(id(72));
    let gold = sqlx::types::Uuid::from_bytes(id(71));
    let nested_gold = sqlx::types::Uuid::from_bytes(id(73));
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
                  definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
                 SELECT $1,world_id,definition_family,definition_production_key,definition_revision_ref,
                        quantity,lifecycle,minted_transaction_id FROM game_item_instances WHERE item_instance_id=$2")
        .bind(nested_gold).bind(gold).execute(&mut *tx).await?;
    sqlx::query("WITH moved AS (DELETE FROM game_item_container_entries WHERE item_instance_id=$1 RETURNING *)
                 INSERT INTO game_item_nested_entries
                   (item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)
                 SELECT item_instance_id,world_id,$2::uuid,placement_ordinal,placed_transaction_id FROM moved
                 UNION ALL
                 SELECT $3::uuid,world_id,$2::uuid,1,placed_transaction_id FROM moved")
        .bind(claimed).bind(bag).bind(nested_gold).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn prepare_red_skull(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/build-training.json"
        ),
        "content-1",
    )
    .map_err(debug)?;
    let before = DurableBuildState::default();
    let after = convert_vocation(&formula, &before, "druid").map_err(debug)?;
    let mut current = fence(1)?;
    current.character_lease_generation = 2;
    h.root
        .commit_character_build(
            authority,
            &h.node,
            current,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(100)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before,
                after,
                pruned_stance: None,
            },
            &formula,
        )
        .await
        .map_err(debug)?;
    sqlx::query(
        "INSERT INTO game_character_pvp_state(character_id,world_id,skull,skull_until)
                 VALUES($1,$2,1,clock_timestamp()+interval '1 day')",
    )
    .bind(sqlx::types::Uuid::from_bytes(id(41)))
    .bind(sqlx::types::Uuid::from_bytes(id(42)))
    .execute(&h.pool)
    .await?;
    Ok(())
}
