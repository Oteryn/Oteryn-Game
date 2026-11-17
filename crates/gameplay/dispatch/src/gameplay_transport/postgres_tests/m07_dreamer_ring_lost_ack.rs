//! A Dreamer ring offering committed with a lost acknowledgement publishes its trees on replay.
use super::TestResult;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    dreamer_death_ring,
    world_callbacks::{Offering, Position},
};

pub(super) const NAME: &str = "m07-ring-offering-lost-ack";

#[test]
fn lost_ack_dreamer_ring_replay_republishes_trees() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(
            NAME,
            Position {
                x: 100,
                y: 100,
                floor: -11,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &super::fixture::Harness,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
) -> TestResult {
    use crate::gameplay_transport::item_move::{
        LoopMove, LoopMoveReplay, LoopMoveResult, LoopTarget,
    };
    use crate::gameplay_transport::item_view::ItemKey;
    use oteryn_protocol_oteryn::item_view::WorldTilePosition;
    const RING: &str = "oteryn:item.tibia.i6299";
    let keys = dreamer_death_ring::TREES.map(|p| {
        oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, 1).ok_or("tree key")
    });
    let keys = [keys[0]?, keys[1]?, keys[2]?];
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 99,
                y: 101,
                floor: -11,
            },
            item: RING.into(),
            count: 1,
        },
        61,
        true,
    )
    .await?;
    let ring: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_instances \
         WHERE definition_production_key=$1 AND lifecycle=1",
    )
    .bind(RING)
    .fetch_one(&h.pool)
    .await?;
    let ring: [u8; 16] = ring.try_into().map_err(|_| "ring id")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: admitted.continuity.next_command_id,
        item_fence: Some(fence.item_fence()),
    };
    let cursor = owner
        .bundle_map()
        .ok_or("map")?
        .facts
        .movement_effects_since(0)
        .last()
        .map_or(0, |e| e.0);
    owner.fixture_interrupt_dreamer_ring_commit(command.game_session_id, command.command_id)?;
    ready(owner).await?;
    assert_eq!(
        owner
            .move_loop_item(
                actor,
                command,
                LoopMove {
                    slot: None,
                    item: ItemKey::Instance(ring),
                    count: None,
                    target: LoopTarget::Ground(WorldTilePosition {
                        x: 99,
                        y: 101,
                        floor: -11,
                    }),
                },
            )
            .await,
        LoopMoveResult::Unknown,
        "the acknowledgement of the committed offering is lost"
    );
    let rows: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_dreamer_ring_offerings),\
         (SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1)",
    )
    .bind(RING)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(rows, (1, 0), "the offering committed and consumed the ring");
    {
        let map = owner.bundle_map().ok_or("map")?;
        for key in keys {
            assert!(!map.facts.base_hidden(key), "nothing was published yet");
        }
        assert!(
            map.facts
                .movement_effects_since(cursor)
                .iter()
                .all(|e| e.2 != 13)
        );
    }
    ready(owner).await?;
    assert_eq!(
        owner.committed_loop_move(actor, command).await,
        Some(LoopMoveReplay::Moved),
        "the resend replays the committed offering"
    );
    let cursor = {
        let map = owner.bundle_map().ok_or("map")?;
        for key in keys {
            assert!(map.facts.base_hidden(key), "the replay publishes the trees");
        }
        let cues = map.facts.movement_effects_since(cursor);
        assert_eq!(cues.iter().filter(|e| e.2 == 13).count(), 3);
        cues.last().map_or(cursor, |e| e.0)
    };
    ready(owner).await?;
    assert_eq!(
        owner.committed_loop_move(actor, command).await,
        Some(LoopMoveReplay::Moved)
    );
    let map = owner.bundle_map().ok_or("map")?;
    for key in keys {
        assert!(map.facts.base_hidden(key));
    }
    assert!(
        map.facts.movement_effects_since(cursor).is_empty(),
        "a second replay presents nothing"
    );
    let offerings: i64 = sqlx::query_scalar("SELECT count(*) FROM game_dreamer_ring_offerings")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(offerings, 1, "the replay writes no second offering");
    eprintln!("M07_DREAMER_RING_LOST_ACK_REPLAY_PUBLISHED");
    Ok(())
}

async fn ready(owner: &super::ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    if owner.root.has_ready_demand() {
        owner
            .root
            .maintain_ready_once()
            .await
            .map_err(super::fixture::debug)?;
    }
    Ok(())
}
