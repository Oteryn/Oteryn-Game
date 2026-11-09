//! Genuine corpse MINT provenance, selected top UUID and database-clock freshness.
use crate::bestiary_postgres_harness as f;
use crate::domain::bestiary::BestiaryRace;
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, GroundPlacement, ItemMintCause, ItemMintOutcome,
    ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::spell_item_transaction::{
    assert_spell_item_scope_in_transaction, corpse_target::read_current_fresh,
};
use crate::durability::spell_items_abi::SpellGroundTarget;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::ScopeOwnershipGeneration;
use oteryn_gameplay_entities::carrier::CombatDeathFixture;
use std::num::NonZeroU32;

#[test]
fn engnpc2_fresh_corpse_postgres_exact_provenance_and_clock() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    f::runtime()?.block_on(async {
        let started = std::time::Instant::now();
        let h = f::Harness::create(admin, "engnpc2_fresh_corpse", true).await?;
        let result = exercise(&h).await;
        h.cleanup().await?;
        result?;
        assert!(started.elapsed().as_secs_f64() > 0.01);
        eprintln!(
            "PG_EXECUTED engnpc2_fresh_corpse_postgres_exact_provenance_and_clock duration={:?}",
            started.elapsed()
        );
        Ok(())
    })
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let scope = f::fence(1)?.runtime_scope;
    let mut native = CombatDeathFixture::new_for_scope(
        scope,
        ScopeOwnershipGeneration::new(1).map_err(f::debug)?,
    )
    .map_err(f::debug)?;
    native
        .strike("fixture:engnpc2.slug", CombatDeathFixture::HEALTH)
        .map_err(f::debug)?;
    let corpse = native.project_death_projection().map_err(f::debug)?;
    let death = corpse.occurrence().death_key().map_err(f::debug)?;
    let at = corpse.position();
    let mut spatial = at.x.to_be_bytes().to_vec();
    spatial.extend_from_slice(&at.y.to_be_bytes());
    spatial.extend_from_slice(&at.floor.to_be_bytes());
    let (frame, map, content) = corpse.context_markers();
    let mut context = Vec::new();
    for value in [frame, map, content] {
        context.extend_from_slice(&value.to_be_bytes());
    }
    let ground = GroundPlacement {
        spatial_position: spatial.clone(),
        corpse_ref: death.native_bytes(),
        map_revision: "map-1".into(),
        content_revision: "content-1".into(),
        native_room_placement_context: context.clone(),
    };
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "fixture:shared-corpse".into(),
        revision_ref: "definition-r1".into(),
    };
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    let race =
        BestiaryRace::new("oteryn:creature.slug", "slug-r1", vec![5, 50, 500]).map_err(f::debug)?;
    let cause = ItemMintCause::from_creature_death(
        death,
        definition.clone(),
        CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
        0,
    )
    .map_err(f::debug)?
    .with_corpse_race(race)
    .map_err(f::debug)?;
    let mut minted = h
        .root
        .freeze_item_mint(
            &recovery,
            &h.node,
            ItemMintRequest {
                cause,
                item: definition.clone(),
                quantity: 1,
                ground,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(f::debug)?;
    let item = *minted.item_instance_id();
    let permit = minted.fresh_death_lane_permit().await;
    assert!(matches!(
        h.root
            .commit_corpse_mint(
                &permit,
                &recovery,
                &h.node,
                &mut minted,
                f::id(f::CHARACTER)
            )
            .await
            .map_err(f::debug)?,
        ItemMintOutcome::Committed(_)
    ));
    drop(permit);
    let target = SpellGroundTarget {
        spatial_position: spatial,
        map_revision: "map-1".into(),
        content_revision: "content-1".into(),
        placement_context: context,
    };
    let permit = SpellLanePermit::of_fresh_scope(scope).await?;
    let mut tx = h.pool.begin().await?;
    let authority = assert_spell_item_scope_in_transaction(
        &mut tx, &permit, &h.root, &recovery, &h.node, scope, 1,
    )
    .await
    .map_err(f::debug)?;
    let age = NonZeroU32::new(60_000).ok_or("age")?;
    assert_eq!(
        read_current_fresh(
            &mut tx,
            &authority,
            &target,
            item,
            "oteryn:creature.slug",
            "slug-r1",
            age
        )
        .await
        .map_err(f::debug)?,
        Some(definition)
    );
    let mut competing = h.pool.begin().await?;
    let busy = sqlx::query("SELECT 1 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE NOWAIT")
        .bind(item.as_slice()).fetch_one(&mut *competing).await;
    assert!(
        busy.as_ref().is_err_and(|error| error
            .as_database_error()
            .and_then(|error| error.code())
            .is_some_and(|code| code == "55P03")),
        "the original corpse row remains locked through the caller transaction"
    );
    competing.rollback().await?;
    let mut moved = target.clone();
    moved.spatial_position[3] ^= 1;
    assert!(
        read_current_fresh(
            &mut tx,
            &authority,
            &moved,
            item,
            "oteryn:creature.slug",
            "slug-r1",
            age
        )
        .await
        .map_err(f::debug)?
        .is_none(),
        "another cell cannot reuse the selected corpse UUID"
    );
    for (selected, race, revision) in [
        (f::id(99), "oteryn:creature.slug", "slug-r1"),
        (item, "oteryn:creature.rat", "slug-r1"),
        (item, "oteryn:creature.slug", "slug-r2"),
    ] {
        assert!(
            read_current_fresh(&mut tx, &authority, &target, selected, race, revision, age)
                .await
                .map_err(f::debug)?
                .is_none()
        );
    }
    sqlx::query("SELECT pg_sleep(0.02)")
        .execute(&mut *tx)
        .await?;
    assert!(
        read_current_fresh(
            &mut tx,
            &authority,
            &target,
            item,
            "oteryn:creature.slug",
            "slug-r1",
            NonZeroU32::new(1).ok_or("short age")?
        )
        .await
        .map_err(f::debug)?
        .is_none()
    );
    tx.rollback().await?;
    let mut other = h.pool.begin().await?;
    assert!(
        read_current_fresh(
            &mut other,
            &authority,
            &target,
            item,
            "oteryn:creature.slug",
            "slug-r1",
            age
        )
        .await
        .is_err(),
        "proof must not cross physical transactions"
    );
    other.rollback().await?;
    assert_eq!(
        h.count("game_character_quest_receipts").await?,
        0,
        "read grants no progress"
    );
    assert_eq!(h.root_revision().await?, "1");
    Ok(())
}
