//! Crystal 201189f, creaturescripts_replica_servants.lua:1-33.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

pub(super) const NAME: &str = "fknow-energy-replica-credit";
const AT: Position = Position {
    x: 32327,
    y: 32093,
    floor: -7,
};

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    // Only the action ID and appearances select the source lever.
    let mut fixture = callback.clone();
    fixture.selector.action = Some(26663);
    fixture.selector.appearance = Some(9125);
    fixture.selector.position = Some(AT);
    quest_world_callbacks::world_with_objects(&fixture, AT, &[], &[9126])
}

#[test]
fn fknow_energy_replica_deaths_credit_the_native_killer_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        quest_world_callbacks::scenario(NAME, AT, false)
    })
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let map = owner.bundle_map().ok_or("replica map")?;
    let pos = crate::map::overlay::TilePos {
        x: AT.x,
        y: AT.y,
        floor: AT.floor,
    };
    let key = oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1)
        .ok_or("lever placement")?;
    let item = *map
        .overlay
        .base()
        .tile(AT.x, AT.y, AT.floor)
        .ok_or("lever tile")?
        .ids()
        .get(1)
        .ok_or("lever item")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    for (revision, appearance) in [(0, 9126), (1, 9125), (2, 9125)] {
        super::m44_source_lifecycle::wait_for_world_action(owner, admitted).await?;
        let payload = encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        })
        .map_err(debug)?;
        let command = admitted.continuity.next_command_id;
        let reply = quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
        assert_eq!(
            decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed
        );
        assert_eq!(
            map.facts
                .base_entry(pos, 1, item)
                .ok_or("lever facts")?
                .appearance_id,
            appearance
        );
        let active_diamond: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_forgotten_knowledge_replica_windows
             WHERE world_id=encode($1,'hex')::uuid AND replica_kind=1
               AND expires_at > statement_timestamp() AND kills=0",
        )
        .bind(admitted.world_id.as_bytes().as_slice())
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(active_diamond, 1);
    }

    super::questpc7_native_deaths::prepare_native_hunter(owner, admitted).await?;
    sqlx::query(
        "INSERT INTO game_forgotten_knowledge_replica_windows(world_id, replica_kind, expires_at)
         VALUES (encode($1,'hex')::uuid, 2, statement_timestamp() - interval '1 second')",
    )
    .bind(admitted.world_id.as_bytes().as_slice())
    .execute(&harness.pool)
    .await?;
    for (kind, first_command, window) in [("diamond", 200, 1_i16), ("golden", 210, 2)] {
        let creature = format!("oteryn:creature.{kind}_servant_replica");
        let track = format!(
            "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/{kind}_servant_counter"
        );
        for expected in 1..=5 {
            // The fixture sends each lethal notification twice through the real death queue.
            super::qengine_kill_stories::death(
                owner,
                admitted,
                &creature,
                first_command + u64::try_from(expected)?,
            )
            .await?;
            let count: i64 = sqlx::query_scalar(
                "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
            )
            .bind(&track)
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(count, expected, "{kind} replica credit");
            let world_count: i32 = sqlx::query_scalar(
                "SELECT kills FROM game_forgotten_knowledge_replica_windows
                 WHERE world_id=encode($1,'hex')::uuid AND replica_kind=$2",
            )
            .bind(admitted.world_id.as_bytes().as_slice())
            .bind(window)
            .fetch_one(&harness.pool)
            .await?;
            assert_eq!(
                i64::from(world_count),
                if window == 1 { expected } else { 0 }
            );
        }
    }
    Ok(())
}
