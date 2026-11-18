//! Crystal 201189f dreamers_challenge_quest/actions_stone_teleport.lua:4-22,30-67.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::{input_item, local, scenario, wire, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::{
    character_progression::CurrentCharacterGameplayFence,
    item_mint::TypedDefinitionRef,
    item_use::fluid::{ItemFluidChange, ItemFluidRequest, ItemFluidSplash},
};
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    world_callbacks::{self, Callback, FLUID_POOL, Offering, Position, SACRIFICE_FIRE, STONE_KEY},
};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, decode_use_result,
    encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::sync::Arc;

pub(super) const PREFIX: &str = "dreamers-stone-partial-offerings";
pub(super) const FULL: &str = "dreamers-stone-full-offerings";
const LEVER: Position = at(32873, 32263);
const PORTAL: Position = at(32881, 32270);
const EXIT: Position = Position {
    x: 32920,
    y: 32296,
    floor: -13,
};
const MATERIALS: [(Position, &str); 4] = [
    (at(32878, 32270), FLUID_POOL),
    (at(32881, 32267), "oteryn:item.tibia.i3052"),
    (at(32881, 32273), "oteryn:item.tibia.i6299"),
    (at(32884, 32270), SACRIFICE_FIRE),
];
const WELLS: [(Position, u16, u16); 4] = [
    (at(32874, 32263), 2094, 2098),
    (at(32875, 32263), 2095, 2099),
    (at(32874, 32264), 2096, 2100),
    (at(32875, 32264), 2097, 2101),
];

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -14 }
}
fn tile(p: Position) -> TilePos {
    TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}

pub(super) fn binding() -> TestResult<Callback> {
    let passages = WELLS.map(|(position, appearance, replacement)| {
        serde_json::json!({
            "position":position,"appearance":appearance,"replacement":replacement,"if_present":true
        })
    });
    let offerings = MATERIALS.map(|(position, item)| {
        serde_json::json!({
            "position":position,"item":item,"count":1
        })
    });
    Ok(serde_json::from_value(serde_json::json!({
        "key":STONE_KEY,"quest":"oteryn:quest.dreamer_s_challenge_quest","trigger":"use",
        "selector":{"native":{"position":LEVER,"ordinal":1,"appearance":2772}},
        "lever":[2772,2773],"reversible":true,"opening_only":true,"requires_world_admission":true,
        "return_on_failure":false,"storage":[],"passages":passages,"offerings":offerings,
        "effects":{"offering_portal":{"position":PORTAL,"sealed":1842,"opened":1949}},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/actions_stone_teleport.lua:4-68"]
    }))?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(
        callback,
        LEVER,
        2772,
        1,
        &[(PORTAL, 1842)],
        &[1949],
        None,
        &[PORTAL, EXIT],
    )
}

pub(super) fn catalogue(
    base: &QuestStateCatalogue,
    callback: &Callback,
) -> TestResult<Arc<QuestStateCatalogue>> {
    let base = base
        .with_fixture_transition(world_callbacks::primary_transition(callback)?)
        .map_err(debug)?;
    let exit = super::dreamers_stone_exit::binding()?;
    let document = serde_json::json!({"schema":1,"callbacks":[callback,&exit]}).to_string();
    Ok(Arc::new(world_callbacks::with_fixture_world_document(
        &base, &document,
    )?))
}

#[test]
fn dreamers_stone_lever_consumes_only_the_prefix_before_the_missing_life_ring() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(PREFIX, || scenario(PREFIX, LEVER, false))
}

#[test]
fn dreamers_stone_lever_consumes_pool_rings_and_live_fire_before_opening_the_exit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(FULL, || scenario(FULL, LEVER, false))
}

async fn pool(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    const VIAL: &str = "oteryn:item.tibia.i2874";
    input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: LEVER,
            item: VIAL.into(),
            count: 1,
        },
        31,
        true,
    )
    .await?;
    let vial: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT item_instance_id FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1"
    ).bind(VIAL).fetch_one(&harness.pool).await?;
    let change = |before, after| ItemFluidChange {
        item_instance_id: *vial.as_bytes(),
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: VIAL.into(),
            revision_ref: "definition-r1".into(),
        },
        fluid_before: before,
        fluid_after: after,
    };
    owner
        .root
        .commit_item_fluid(
            owner.character,
            owner.holder,
            fence.item_fence(),
            ItemFluidRequest {
                command_id: 2901,
                transaction_id: id(118),
                change: change(0, 1),
                other: None,
            },
        )
        .await
        .map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(
        &placement_key(tile(MATERIALS[0].0), 0)
            .ok_or("pool tile")?
            .to_be_bytes(),
    );
    owner
        .root
        .commit_item_fluid_at(
            owner.character,
            owner.holder,
            fence.item_fence(),
            ItemFluidRequest {
                command_id: 2902,
                transaction_id: id(119),
                change: change(1, 0),
                other: None,
            },
            Some(ItemFluidSplash {
                placement,
                fluid_subtype: 1,
                splash_item_id: 2886,
                lifetime_millis: 10_000,
            }),
        )
        .await
        .map_err(debug)?;
    Ok(())
}

// Retain the exact Ground/spell line shape produced by the fire-field rune.
// The real USE transaction must retire this instance through all current guards.
async fn field(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let content = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let map_revision = crate::map::overlay::map_revision(map.overlay.base());
    let (pos, key) = MATERIALS[3];
    let spatial =
        crate::durability::map_item_mint_audit::map_item_spatial_position(pos.x, pos.y, pos.floor);
    let item = id(122);
    let source = id(123);
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
        .bind(item.as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(key).bind(source.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal)
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8,$9,1)")
        .bind(item.as_slice()).bind(owner.world_id.as_bytes().as_slice()).bind(owner.channel_id.as_bytes().as_slice())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(&spatial).bind(source.as_slice()).bind(&map_revision)
        .bind(&content).bind(frame.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_spell_item_lines(transaction_id,ordinal,operation_kind,item_instance_id,definition_family,definition_production_key,definition_revision,
        quantity_before,quantity_after,state_revision_before,world_id,channel_id,spatial_position,map_revision,
        content_revision,placement_context,content_generation_digest,immovable_block_solid,expires_at_unix_ms)
        VALUES(encode($1,'hex')::uuid,1,1,encode($2,'hex')::uuid,'Item',$3,'definition-r1',0,1,0,
          encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7,$8,$9,$10,false,
          floor(extract(epoch from clock_timestamp())*1000)::bigint+70000)")
        .bind(source.as_slice()).bind(item.as_slice()).bind(key).bind(owner.world_id.as_bytes().as_slice())
        .bind(owner.channel_id.as_bytes().as_slice()).bind(&spatial).bind(&map_revision).bind(&content)
        .bind(frame.as_slice()).bind(digest.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    full: bool,
) -> TestResult {
    for &(position, item) in &MATERIALS[if full { 1 } else { 2 }..3] {
        input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position,
                item: item.into(),
                count: 1,
            },
            u64::from(position.y),
            false,
        )
        .await?;
    }
    if full {
        field(owner, harness, fence).await?;
    }
    pool(owner, harness, fence).await?;
    let map = owner.bundle_map().ok_or("map")?;
    let source = placement_key(tile(LEVER), 1).ok_or("lever key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&source.to_be_bytes());
    let payload = |expected_revision| {
        encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision,
        })
        .map_err(debug)
    };
    let reply = wire(owner, admitted, 1, COMMAND_TYPE_USE_INTENT, payload(0)?).await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );

    let appearance = |position: Position| -> TestResult<u16> {
        let at = tile(position);
        let base = map
            .overlay
            .base()
            .tile(at.x, at.y, at.floor)
            .ok_or("tile")?;
        Ok(map
            .facts
            .base_entry(at, 1, *base.ids().get(1).ok_or("placement")?)
            .ok_or("entry")?
            .appearance_id)
    };
    assert_eq!(appearance(LEVER)?, 2773);
    assert_eq!(appearance(PORTAL)?, if full { 1949 } else { 1842 });
    for (at, _, active) in WELLS {
        assert_eq!(appearance(at)?, active);
    }
    let receipt_counts = || {
        sqlx::query_as::<_, (i64, i64, i64)>(
            "SELECT (SELECT count(*) FROM game_quest_stone_splash_consumptions),
                (SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$1),
                (SELECT count(*) FROM game_item_instances
                 WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i6299')",
        )
        .bind(STONE_KEY)
        .fetch_one(&harness.pool)
    };
    let expected_counts = if full { (1, 3, 0) } else { (1, 0, 1) };
    assert_eq!(receipt_counts().await?, expected_counts);

    // Crystal actions_stone_teleport.lua:16-23,33-43 only restores the
    // wells on the return face. The opened portal stays until a player uses it.
    let reply = wire(owner, admitted, 2, COMMAND_TYPE_USE_INTENT, payload(1)?).await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(appearance(LEVER)?, 2772);
    for (at, dormant, _) in WELLS {
        assert_eq!(appearance(at)?, dormant);
    }
    assert_eq!(appearance(PORTAL)?, if full { 1949 } else { 1842 });
    assert_eq!(receipt_counts().await?, expected_counts);
    if full {
        let actor = admitted.runtime_actor.ok_or("actor")?;
        {
            let mut runtime = owner.runtime.lock().await;
            let teleport = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                owner.movement_cells(),
                actor,
                admitted.game_session_id,
                runtime.read_actor_position(actor).map_err(debug)?,
                local(at(32880, 32270)),
                false,
                &crate::map::floor::BundleDoors(&map.facts),
            )
            .map_err(debug)?;
            runtime.commit_source_step(teleport).map_err(debug)?;
        }
        let reply = wire(
            owner,
            admitted,
            3,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(StepDirection::East),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(EXIT)
        );
        assert_eq!(appearance(PORTAL)?, 1842);
    }
    Ok(())
}
