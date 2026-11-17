//! A native corpse, two ordinary drops, then the charged-ring arrival through dispatch.
use super::super::fixture::{Harness, debug, id};
use super::super::{ComposedFreshAdmission, TestResult};
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{
    connection::UseCommand,
    item_move::{LoopMove, LoopMoveResult, LoopTarget},
    item_view::ItemKey,
};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::item_view::WorldTilePosition;

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
    fence: CurrentCharacterItemFence,
) -> TestResult {
    let start = WorldTilePosition {
        x: 33198,
        y: 32103,
        floor: -13,
    };
    let target = WorldTilePosition { x: 33199, ..start };
    let pin = owner.runtime.lock().await.content_pin().clone();
    let revision = |digest: [u8; 32]| {
        format!(
            "sha256:{}",
            digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        )
    };
    let definition = |family: &str, key: &str| TypedDefinitionRef {
        family: family.into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    };
    let mut candidate = owner
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    owner.world_id,
                    owner.channel_id,
                    fence.scope_ownership_generation,
                    91,
                    1,
                    definition("LootTable", "fixture:elemental-live"),
                    crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY.into(),
                    0,
                ),
                item: definition("Item", "oteryn:item.tibia.i8141"),
                quantity: 1,
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            33198, 32103, 13,
                        ),
                    corpse_ref: id(92).to_vec(),
                    map_revision: revision(pin.map_revision_digest()),
                    content_revision: revision(pin.server_artifact_digest()),
                    native_room_placement_context: pin.frame_binding_digest().to_vec(),
                },
                content_revision: revision(pin.server_artifact_digest()),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = candidate.fresh_death_lane_permit().await;
    assert!(matches!(
        owner
            .root
            .commit_corpse_mint(
                &permit,
                owner.character,
                owner.holder,
                &mut candidate,
                *fence.character_id.as_bytes()
            )
            .await
            .map_err(debug)?,
        ItemMintOutcome::Committed(_)
    ));
    drop(permit);
    let item: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i8141' AND lifecycle=1 LIMIT 1").fetch_one(&h.pool).await?;
    let item: [u8; 16] = item.try_into().map_err(|_| "corpse identity")?;
    let command = |command_id| UseCommand {
        game_session_id: fence.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let request = |target| LoopMove {
        item: ItemKey::Instance(item),
        count: None,
        target,
    };
    assert!(matches!(
        crate::gameplay_transport::loop_move::move_loop_item(
            owner,
            actor,
            command(40000),
            request(LoopTarget::MainBackpack)
        )
        .await,
        LoopMoveResult::Refused(_)
    ));
    for (command_id, to) in [(40001, target), (40002, start)] {
        assert_eq!(
            crate::gameplay_transport::loop_move::move_loop_item(
                owner,
                actor,
                command(command_id),
                request(LoopTarget::Ground(to))
            )
            .await,
            LoopMoveResult::Committed
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_elemental_summon_intents")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(count, 0, "an incomplete ring leaves the corpse alone");
    }
    // The fixture supplies already charged source holes; soil use is a separate path.
    let map = owner.bundle_map().ok_or("fire map")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let charged = native
        .item_index()
        .ok_or("index")?
        .definition_ref("Item", "oteryn:item.tibia.i7745", "definition-r1")
        .ok_or("charged fire hole")?;
    for at in oteryn_gameplay_quests::quest::elemental_arrival::RECIPES[1].positions {
        let pos = crate::map::overlay::TilePos {
            x: at.x,
            y: at.y,
            floor: at.floor,
        };
        let source = *map
            .overlay
            .base()
            .tile(at.x, at.y, at.floor)
            .and_then(|t| t.ids().get(1))
            .ok_or("native hole")?;
        let before = map.facts.base_entry(pos, 1, source).ok_or("hole facts")?;
        assert_eq!(before.appearance_id, 7743);
        let after = crate::map::view::EntryFacts {
            definition: oteryn_protocol_oteryn::world_map::MapDefinition::Item(charged),
            appearance_id: 7745,
            ..before
        };
        assert!(map.facts.apply_quest_callback(
            crate::map::view::placement_key(pos, 1).ok_or("hole key")?,
            0,
            after,
            &[],
            &[],
            false,
            None
        ));
    }
    assert_eq!(
        crate::gameplay_transport::loop_move::move_loop_item(
            owner,
            actor,
            command(40003),
            request(LoopTarget::Ground(target))
        )
        .await,
        LoopMoveResult::Committed
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_elemental_summon_intents")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        count, 1,
        "the real drop must dispatch the complete elemental arrival"
    );
    let state: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(state, (0, 2));
    for _ in 0..2 {
        assert!(
            owner
                .boss_spawn_pass(&mut Default::default())
                .await
                .map_err(|_| "native boss pass")?
        );
        let runtime = owner.runtime.lock().await;
        assert_eq!(runtime.boss_spawns().len(), 1);
        let spawn = runtime.boss_spawns().first().ok_or("fire boss")?;
        assert_eq!(spawn.position.floor, 13);
        assert!(
            spawn
                .actor
                .is_some_and(|actor| runtime.contains_live_creature(actor))
        );
    }
    eprintln!("QN1_ELEMENTAL_LIVE_ARRIVAL_PG_EXECUTED");
    Ok(())
}
