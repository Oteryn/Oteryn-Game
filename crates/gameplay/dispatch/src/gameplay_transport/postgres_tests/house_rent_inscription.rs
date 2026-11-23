use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async as run;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_loop_move::DepotSearchRetrieval;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{ChannelContentPin, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::item_inscription::{CAPABILITY_ITEM_INSCRIPTION_READ_V1, Reply};

const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";

#[test]
fn rent_warning_letter_is_readable_after_inbox_claim_postgres() -> TestResult {
    run_inscription_case(false)
}

#[test]
fn retrieve_all_claims_legacy_and_new_house_rent_letters_postgres() -> TestResult {
    run_inscription_case(true)
}

fn run_inscription_case(with_legacy: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    run(async move |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_with_admission_seed(admin, "rent_inscription", true, false).await?;
        let result = scenario(&h, with_legacy).await;
        h.cleanup().await?;
        result?;
        println!(
            "PG_MARKER house-rent warning inscription elapsed={:?}",
            started.elapsed()
        );
        Ok(())
    })
}

async fn scenario(h: &Harness, with_legacy: bool) -> TestResult {
    super::reset();
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let base = Dependencies::new()?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            base.admitted.world_id,
            None,
        );
    let active = controller.active().ok_or("active content")?;
    let native = active.native_gameplay().ok_or("native content")?;
    let content_pin = ChannelContentPin::from_activation(
        base.admitted.world_id,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        pin.qualified_room().map_revision_digest(),
        (6, 0, 7),
    );
    let deps = Dependencies::with_content(content_pin, base.room, base.spells)?;
    deps.runtime
        .lock()
        .await
        .initialize_pinned_test_position(
            deps.actor,
            MovementLocalPosition {
                x: 6,
                y: 0,
                floor: 7,
            },
        )
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(admitted.game_session_id)
        .await?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
    admitted.item_fence = Some(fence);
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[4, 6, 115, CAPABILITY_ITEM_INSCRIPTION_READ_V1],
    )
    .ok_or("inscription offer")?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice())
        .bind(pin.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    super::item_look_fixtures::seed_main_backpack(h, &[]).await?;
    seed_due_house(h).await?;

    let scope = fence.runtime_scope;
    if with_legacy {
        install_legacy_tick(h).await?;
        assert!(
            h.root
                .advance_house_rent(&authority, &h.node, scope, 1)
                .await?
        );
        let old_revision: String = sqlx::query_scalar(
            "SELECT i.definition_revision_ref FROM game_item_instances i
             JOIN game_house_rent_letter_mints m ON m.letter_id=i.item_instance_id",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(old_revision, "house-rent-warning");
        apply_warning_migration(h).await?;
        // Move the fixture to the next due period, with the first letter still in the Inbox.
        sqlx::query("UPDATE game_house_properties SET paid_until=statement_timestamp()-interval '1 hour',revision=revision+1 WHERE house_key=$1")
            .bind(HOUSE).execute(&h.pool).await?;
        sqlx::query("UPDATE game_house_rent_state SET warnings=0,next_rent_at=statement_timestamp() WHERE house_key=$1")
            .bind(HOUSE).execute(&h.pool).await?;
    }
    assert!(
        h.root
            .advance_house_rent(&authority, &h.node, scope, 1)
            .await?
    );
    assert!(
        !h.root
            .advance_house_rent(&authority, &h.node, scope, 1)
            .await?
    );
    let letters: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(letter_id) FROM game_house_rent_inbox_letters WHERE house_key=$1 ORDER BY delivered_at",
    )
    .bind(HOUSE)
    .fetch_all(&h.pool)
    .await?;
    let expected = if with_legacy { 2 } else { 1 };
    assert_eq!(
        letters.len(),
        expected,
        "repeating the tick produces one new letter"
    );
    let item = ItemKey::Instance(letters[expected - 1].as_slice().try_into()?);
    assert_eq!(
        owner.item_inscription_read(admitted, Some(item)).await,
        None
    );

    let permit = SpellLanePermit::of_fresh_scope(scope)
        .await
        .map_err(debug)?;
    let claim = h
        .root
        .commit_depot_search_retrieval(
            &permit,
            &authority,
            &h.node,
            fence,
            CommandRef::new(admitted.game_session_id, CommandId::new(1).map_err(debug)?),
            &NativeItemWeights(native),
            3506,
            true,
        )
        .await?;
    drop(permit);
    assert_eq!(
        claim,
        Ok(DepotSearchRetrieval::Retrieved(u32::try_from(expected)?)),
        "the claim must move every stamped letter in this Inbox"
    );
    let deadline: String = sqlx::query_scalar(
        "SELECT to_char((paid_until + interval '168 hours') AT TIME ZONE 'UTC',
                        'YYYY-MM-DD HH24:MI:SS \"UTC\"')
         FROM game_house_properties WHERE house_key=$1",
    )
    .bind(HOUSE)
    .fetch_one(&h.pool)
    .await?;
    let inscription = owner.item_inscription_read(admitted, Some(item)).await;
    assert_eq!(
        inscription.as_deref().and_then(Reply::decode),
        Some(Reply {
            text: format!(
                "Warning! \nThe monthly rent of 400000 gold for your house \"Admiral's Avenue 1\" is payable. Pay by {deadline} or you will lose this house."
            ),
            max_length: 1_999,
            writable: false,
        }),
        "rent warning should be readable after Inbox claim (claim outcome: {claim:?})"
    );
    Ok(())
}

async fn seed_due_house(h: &Harness) -> TestResult {
    sqlx::query(include_str!(
        "../../../../../../apps/game-server/tests/support/owned_house_fixture.sql"
    ))
    .bind(id(42).as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    sqlx::query("UPDATE game_house_properties SET rent_gold=400000, paid_until=statement_timestamp()-interval '1 day', revision=revision+1 WHERE house_key=$1")
        .bind(HOUSE).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_public_house_catalogue(world_id,house_key,name,kind,town,size_sqm,beds,rent_gold,content_digest) SELECT world_id,house_key,'Admiral''s Avenue 1',kind,'oteryn:content.area.city.liberty_bay',91,2,rent_gold,content_digest FROM game_house_properties WHERE house_key=$1")
        .bind(HOUSE).execute(&h.pool).await?;
    Ok(())
}

async fn install_legacy_tick(h: &Harness) -> TestResult {
    let predecessor =
        include_str!("../../../../../../apps/game-server/migrations/0941_house_rent_parity.sql");
    // The harness already has 0941's schema. Restore only its tick for the historical mint.
    let start = predecessor
        .find("CREATE OR REPLACE FUNCTION game_house_rent_tick")
        .ok_or("0941 rent tick")?;
    let tick = predecessor[start..].replace(
        "OUT operation TEXT)",
        "OUT operation TEXT,OUT cancelled_operation TEXT)",
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(tick))
        .execute(&h.pool)
        .await?;
    Ok(())
}

async fn apply_warning_migration(h: &Harness) -> TestResult {
    let migration = include_str!(
        "../../../../../../apps/game-server/migrations/3121_house_rent_warning_text.sql"
    );
    let mut tx = h.pool.begin().await?;
    sqlx::raw_sql(migration).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
