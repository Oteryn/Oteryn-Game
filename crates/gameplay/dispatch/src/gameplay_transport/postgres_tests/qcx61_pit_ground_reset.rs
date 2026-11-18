//! Crystal 201189f8 data-global/lib/quests/svargrond_arena.lua:217-232.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::item_mint::{
    CORPSE_MATERIALIZATION_PURPOSE_KEY, CorpseContainerPlacement, CorpseLootMintRequest,
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

const INSIDE: Position = Position {
    x: 32208,
    y: 31098,
    floor: -7,
};
const OUTSIDE: Position = Position {
    x: 32215,
    y: 31098,
    floor: -7,
};

pub(super) struct Observed {
    ordinary: [Vec<u8>; 3],
    corpse: [u8; 16],
    loot: [u8; 16],
}

#[test]
fn native_arena_entry_retires_only_the_original_top_ground_item() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(
        "qcx61-pit-ground-reset",
        super::arena_match::native_pit_ground_reset,
    )
}

pub(super) async fn seed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult<Observed> {
    let fence = admitted.item_fence.ok_or("item fence")?;
    let mut ids = Vec::new();
    for (index, (appearance, position)) in [(3031, INSIDE), (3035, INSIDE), (3035, OUTSIDE)]
        .into_iter()
        .enumerate()
    {
        super::quest_world_callbacks::input_item(
            owner,
            fence,
            &Offering {
                position,
                item: format!("oteryn:item.tibia.i{appearance}"),
                count: 1,
            },
            516 + u64::try_from(index)?,
            false,
        )
        .await?;
        let cell = crate::durability::map_item_mint_audit::map_item_spatial_position(
            position.x,
            position.y,
            position.floor,
        );
        let id: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(item_instance_id) FROM game_item_ground_locations
             WHERE spatial_position=$1 ORDER BY stack_ordinal DESC LIMIT 1",
        )
        .bind(cell)
        .fetch_one(&h.pool)
        .await?;
        ids.push(id);
    }
    let [corpse, loot] = seed_corpse(owner, fence).await?;
    owner.states.loop_ground.invalidate();
    Ok(Observed {
        ordinary: ids.try_into().map_err(|_| "three observed items")?,
        corpse,
        loot,
    })
}

async fn seed_corpse(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
) -> TestResult<[[u8; 16]; 2]> {
    let revision = |hash: [u8; 32]| {
        format!(
            "sha256:{}",
            hash.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )
    };
    let ground = {
        let runtime = owner.runtime.lock().await;
        let pin = runtime.content_pin();
        GroundPlacement {
            spatial_position: [
                32208_i32.to_be_bytes().as_slice(),
                31099_i32.to_be_bytes().as_slice(),
                7_i16.to_be_bytes().as_slice(),
            ]
            .concat(),
            corpse_ref: id(25).to_vec(),
            map_revision: revision(runtime.map_revision_digest()),
            content_revision: revision(pin.server_artifact_digest()),
            native_room_placement_context: pin.frame_binding_digest().to_vec(),
        }
    };
    let cause = |key: &str, draw| {
        ItemMintCause::for_test_scope(
            fence.runtime_scope,
            fence.scope_ownership_generation,
            101,
            1,
            TypedDefinitionRef {
                family: "LootTable".into(),
                production_key: "fixture:arena.pit-retirement".into(),
                revision_ref: "definition-r1".into(),
            },
            key.to_owned(),
            draw,
        )
    };
    let item = |appearance| TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{appearance}"),
        revision_ref: "definition-r1".into(),
    };
    // Use a native movable corpse like the production Elemental Spheres case.
    let mut corpse = owner
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause: cause(CORPSE_MATERIALIZATION_PURPOSE_KEY, 0),
                item: item(8139),
                quantity: 1,
                ground,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("Item lane")?;
    let ItemMintOutcome::Committed(corpse) = owner
        .root
        .commit_corpse_mint(
            &permit,
            owner.character,
            owner.holder,
            &mut corpse,
            *fence.character_id.as_bytes(),
        )
        .await
        .map_err(debug)?
    else {
        return Err("fresh corpse".into());
    };
    let mut loot = owner
        .root
        .freeze_corpse_loot_mint(
            owner.character,
            owner.holder,
            CorpseLootMintRequest {
                cause: cause("fixture:arena.pit-retirement-gold", 1),
                item: item(3031),
                quantity: 7,
                placement: CorpseContainerPlacement {
                    corpse_item_instance_id: corpse.item_instance_id,
                    placement_ordinal: 1,
                    fluid_subtype: None,
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let ItemMintOutcome::Committed(loot) = owner
        .root
        .commit_corpse_loot_mint(&permit, owner.character, owner.holder, &mut loot)
        .await
        .map_err(debug)?
    else {
        return Err("fresh corpse loot".into());
    };
    Ok([corpse.item_instance_id, loot.item_instance_id])
}

pub(super) async fn assert_retirement(h: &Harness, observed: Observed) -> TestResult {
    let [under, top, outside] = observed.ordinary;
    let result: (i16, bool, i64) = sqlx::query_as(
        "SELECT i.lifecycle,g.item_instance_id IS NOT NULL,
             (SELECT count(*) FROM game_item_decay_retire_receipts r
              JOIN game_item_audit_outbox a USING(item_instance_id,transaction_id,event_id)
              WHERE r.item_instance_id=i.item_instance_id AND r.cause_kind='ARENA_RESET')
         FROM game_item_instances i LEFT JOIN game_item_ground_locations g USING(item_instance_id)
         WHERE i.item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(top)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(result, (2, false, 1));
    let corpse_receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_decay_retire_receipts r
         JOIN game_item_audit_outbox a USING (item_instance_id,transaction_id,event_id)
         JOIN game_item_instances i USING (item_instance_id)
         WHERE r.ground_root_item_instance_id=encode($1,'hex')::uuid
           AND r.item_instance_id IN (encode($1,'hex')::uuid,encode($2,'hex')::uuid)
           AND r.cause_kind='ARENA_RESET' AND i.lifecycle=2 AND i.quantity=0",
    )
    .bind(observed.corpse.as_slice())
    .bind(observed.loot.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        corpse_receipts, 2,
        "the original corpse and its retained loot retire together"
    );
    for unchanged in [under, outside] {
        let still_ground: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM game_item_ground_locations g
             JOIN game_item_instances i USING(item_instance_id)
             WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1)",
        )
        .bind(unchanged)
        .fetch_one(&h.pool)
        .await?;
        assert!(still_ground);
    }
    Ok(())
}
