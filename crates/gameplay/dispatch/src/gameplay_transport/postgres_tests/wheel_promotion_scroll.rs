//! Carried scroll USE, its audit receipt and both Wheel budget consumers.
use super::atelier_fixtures::fixture as atelier_fixture;
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_wheel::{
    WheelChangeFacts, WheelChangeRequest, WheelCommitOutcome, WheelOccurrence, WheelRuleset,
    WheelSlots,
};
use crate::durability::item_use_audit::decode_item_use_envelope;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_progression::wheel::WheelRefusal;
use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::sync::Arc;
use std::time::Duration;

const SCROLLS: [(u8, &str, u32); 6] = [
    (170, "oteryn:item.tibia.i43946", 3),
    (172, "oteryn:item.tibia.i43946", 3),
    (174, "oteryn:item.tibia.i43947", 5),
    (176, "oteryn:item.tibia.i43948", 9),
    (178, "oteryn:item.tibia.i43949", 13),
    (180, "oteryn:item.tibia.i43950", 20),
];

#[test]
fn wheel_promotion_scroll_use_postgres_grants_once_checks_level_and_funds_allocation() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(Box::pin(scenario(admin)))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "promotion-scroll scenario panicked")??;
    Ok(())
}

async fn scenario(admin: String) -> TestResult {
    let h =
        Harness::create_with_admission_seed(admin, "wheel_promotion_scroll", true, false).await?;
    // The harness already applied all migrations. Reapply this one before using its schema.
    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/migrations/3358_wheel_promotion_scroll.sql"
    )))
    .execute(&h.pool)
    .await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?;
    super::tutorial_vitals_fixture::publish_native_activation(&h, &pin).await?;
    let room = pin.qualified_room().clone();
    let deps = Dependencies::with_content(
        pin.into_channel_pin(),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = controller.active();
    deps.initialize(&owner).await;
    let session = deps.admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    let purchase = atelier_fixture::prepare(&h).await?;
    seed_scrolls(&h).await?;
    eprintln!("wheel promotion scroll PostgreSQL fixture ready");

    let use_scroll = async |owner: &ComposedFreshAdmission<'_, '_, '_>, command_id, seed| {
        owner
            .use_item(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: Some(fence.item_fence()),
                },
                ItemKey::Instance(id(seed)),
            )
            .await
            .disposition
    };
    assert_eq!(
        use_scroll(&owner, 1, 170).await,
        UseDisposition::RequirementNotMet
    );
    assert_eq!(item_state(&h, 170).await?, (1, 1, true));

    atelier_fixture::eligible(&h.pool).await?;
    let stale_fence = crate::durability::item_transfer::CurrentCharacterItemFence {
        connection_generation: crate::foundation::ConnectionGeneration::new(
            fence.item_fence().connection_generation.get() + 1,
        )
        .map_err(debug)?,
        ..fence.item_fence()
    };
    assert_eq!(
        owner
            .use_item(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 90,
                    item_fence: Some(stale_fence),
                },
                ItemKey::Instance(id(170)),
            )
            .await
            .disposition,
        UseDisposition::Rejected
    );
    assert_eq!(item_state(&h, 170).await?, (1, 1, true));

    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_npc_purchase(&h.root, &authority, &h.node, fence, purchase, &premium)
        .await
        .map_err(debug)?;
    assert!(matches!(
        outcome,
        crate::durability::npc_purchase::PurchaseOutcome::Committed(_)
    ));

    let data = WheelGemData::embedded().map_err(debug)?;
    let rules =
        Arc::new(WheelRuleset::from_catalogue(data.revision(), data.wheel()).map_err(debug)?);
    let wheel_view = async || {
        h.root
            .read_wheel_presentation(
                &authority,
                &h.node,
                fence.item_fence(),
                Arc::clone(&rules),
                &premium,
            )
            .await
            .map_err(debug)
    };
    let mut points = [0; 36];
    points[14] = 11;
    let allocate = async |occurrence| {
        owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_wheel(
                &h.root,
                &authority,
                &h.node,
                fence,
                WheelChangeRequest {
                    occurrence: WheelOccurrence::from_bytes(id(occurrence)).map_err(debug)?,
                    expected_wheel_revision: 0,
                    slots: WheelSlots::new(points).map_err(debug)?,
                    vessels: None,
                },
                Arc::clone(&rules),
                WheelChangeFacts { at_temple: true },
                &premium,
            )
            .await
            .map_err(debug)
    };
    let draft = WheelSlots::new(points).map_err(debug)?;
    let before = wheel_view().await?;
    assert_eq!(before.budget.available, 10);
    assert_eq!(
        before.validate_draft(&rules, &draft, &before.vessels, true),
        Err(WheelRefusal::OverPoints)
    );
    assert_eq!(
        allocate(186).await?,
        WheelCommitOutcome::Refused(WheelRefusal::OverPoints)
    );

    assert_eq!(use_scroll(&owner, 2, 170).await, UseDisposition::Committed);
    assert_eq!(item_state(&h, 170).await?, (0, 2, false));
    assert_eq!(use_scroll(&owner, 2, 170).await, UseDisposition::Committed);
    let abridged = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", SCROLLS[0].1, "definition-r1"))
        .ok_or("pinned Abridged scroll definition")?;
    assert_eq!(
        owner
            .use_item_by_definition(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 3,
                    item_fence: Some(fence.item_fence()),
                },
                abridged
            )
            .await
            .disposition,
        UseDisposition::NotPossible
    );
    assert_eq!(item_state(&h, 172).await?, (1, 1, true));
    assert_audited_burn(&h).await?;
    let first = wheel_view().await?;
    assert_eq!(
        (first.budget.available, first.budget.unlock_points),
        (13, 13)
    );
    assert_eq!(
        first.validate_draft(&rules, &draft, &first.vessels, true),
        Ok(())
    );
    assert!(matches!(
        allocate(188).await?,
        WheelCommitOutcome::Committed(_)
    ));

    let mut expected = 13;
    for (command, (seed, _, points)) in (4..).zip(SCROLLS.into_iter().skip(2)) {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_secs(1))
            .ok_or("advance item-use clock")?;
        assert_eq!(
            use_scroll(&owner, command, seed).await,
            UseDisposition::Committed
        );
        assert_eq!(item_state(&h, seed).await?, (0, 2, false));
        expected += points;
        let actual = wheel_view().await?.budget;
        assert_eq!(
            (actual.available, actual.unlock_points),
            (expected, expected)
        );
    }
    assert_eq!(expected, 60);
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_use_records u JOIN game_item_audit_outbox a \
         USING(transaction_id,event_id) WHERE u.cause_kind=3358 AND u.shape=1",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(audited, 5);
    eprintln!("WHEEL_PROMOTION_SCROLL POSTGRES_CASE_COMPLETED");
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}

async fn assert_audited_burn(h: &Harness) -> TestResult {
    let bytes: Vec<u8> = sqlx::query_scalar(
        "SELECT a.envelope FROM game_item_use_records u JOIN game_item_audit_outbox a \
         USING(transaction_id,event_id) WHERE u.cause_kind=3358 AND u.command_id=2",
    )
    .fetch_one(&h.pool)
    .await?;
    let (_, use_event) = decode_item_use_envelope(&bytes).map_err(debug)?;
    let before = use_event.source_before.ok_or("audited source")?;
    assert_eq!(use_event.cause.ok_or("audited cause")?.kind, 3358);
    assert_eq!(before.item_instance_id.as_slice(), id(170));
    assert_eq!(before.quantity, 1);
    assert_eq!(
        use_event.source_after.ok_or("audited retirement")?.quantity,
        0
    );
    Ok(())
}

async fn item_state(h: &Harness, seed: u8) -> TestResult<(i64, i16, bool)> {
    Ok(sqlx::query_as(
        "SELECT quantity,lifecycle,EXISTS(SELECT 1 FROM game_item_container_entries e \
         WHERE e.item_instance_id=i.item_instance_id) FROM game_item_instances i \
         WHERE i.item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(seed).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

async fn seed_scrolls(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (ordinal, (seed, key, _)) in (2_i64..).zip(SCROLLS) {
        let sql =
            crate::durability::item_fee_burn::test_support::item(seed, key, "definition-r1", 1);
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO game_item_container_entries(character_id,parent_item_instance_id, \
             item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES( \
             encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid, \
             encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid)",
        )
        .bind(id(41).as_slice())
        .bind(id(210).as_slice())
        .bind(id(seed).as_slice())
        .bind(id(42).as_slice())
        .bind(ordinal)
        .bind(id(seed + 1).as_slice())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
