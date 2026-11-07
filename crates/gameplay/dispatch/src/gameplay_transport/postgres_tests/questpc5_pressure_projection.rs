//! Grouped, player-qualified pressure projections share the existing map owner.
use super::TestResult;
#[test]
fn questpc5_grouped_pressure_source_projection() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let started = std::time::Instant::now();
        let h = super::fixture::Harness::create_with_admission_seed(
            admin,
            "questpc5_grouped_pressure",
            false,
            false,
        )
        .await?;
        eprintln!(
            "POSTGRES_CASE_STARTED grouped_pressure {:?}",
            started.elapsed()
        );
        let doc: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/quests/shared-access-callbacks.json"
        )))?;
        let mut callback = doc["callbacks"]
            .as_array()
            .ok_or("callbacks")?
            .iter()
            .find(|c| c["key"] == "oteryn:quest-callback/thais-lighthouse-plate")
            .ok_or("source pressure shape")?
            .clone();
        callback["occupancy_requirement"] = serde_json::json!({"positions":[{"x":33190,"y":31629,"floor":-13},{"x":33191,"y":31629,"floor":-13}],"players_only":true,"eject":{"x":33211,"y":31631,"floor":-13}});
        callback["selector"]["position"] = serde_json::json!({"x":33190,"y":31629,"floor":-13});
        callback["passages"] = serde_json::json!([{"position":{"x":33210,"y":31630,"floor":-13},"appearance":1295,"replacement":null}]);
        let _: oteryn_gameplay_quests::quest::world_callbacks::Callback =
            serde_json::from_value(callback)?;
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_CASE_COMPLETED grouped_pressure {:?}",
            started.elapsed()
        );
        Ok(())
    })?;
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
    assert_eq!(
        callbacks
            .iter()
            .filter(|c| c.occupancy_requirement.is_some())
            .count(),
        6
    );
    super::qengine_access::scenario(
        "demon-helmet-pressure-1",
        Position {
            x: 33190,
            y: 31629,
            floor: -13,
        },
    )?;
    super::qengine_access::scenario(
        "devil-helmet-pressure",
        Position {
            x: 32468,
            y: 32119,
            floor: -14,
        },
    )
}

use super::fixture::{Harness, debug, id};
use crate::foundation::{ExactActorRef, GameSessionId};
use crate::gameplay_transport::{
    ComposedFreshAdmission, connection::AdmittedSession, connection::FreshAdmissionAuthority,
};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
fn tile(p: Position) -> crate::map::overlay::TilePos {
    crate::map::overlay::TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
async fn refresh(owner: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) -> TestResult {
    let permit = owner
        .enter_input(session)
        .await
        .ok_or("pressure reconciliation")?;
    drop(permit);
    Ok(())
}
fn admit_player(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    n: u8,
    p: Position,
) -> TestResult<(ExactActorRef, GameSessionId)> {
    let session = GameSessionId::decode(&id(n)).map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(actor, super::quest_world_callbacks::local(p))
        .map_err(debug)?;
    Ok((actor, session))
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    callback: &Callback,
    trigger: Position,
) -> TestResult {
    let started = std::time::Instant::now();
    let fence = admitted.item_fence.ok_or("fence")?;
    let before = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    super::qengine_access::step(owner, admitted, 1, StepDirection::East).await?;
    let map = owner.bundle_map().ok_or("map")?;
    if callback.occupancy_items.is_empty() {
        let walls: Vec<_> = callback
            .passages
            .iter()
            .map(|p| {
                oteryn_world_bundle::bundle::placement_key(
                    p.position.floor,
                    p.position.x,
                    p.position.y,
                    1,
                )
                .ok_or("wall")
            })
            .collect::<Result<_, _>>()?;
        for key in &walls {
            assert!(
                !map.facts.base_hidden(*key),
                "one player cannot open the group"
            );
        }
        let second = callback
            .occupancy_requirement
            .as_ref()
            .ok_or("group")?
            .positions[1];
        let creature = owner
            .runtime
            .lock()
            .await
            .admit_pinned_test_creature(super::quest_world_callbacks::local(second))
            .map_err(debug)?;
        refresh(owner, admitted.game_session_id).await?;
        for key in &walls {
            assert!(
                !map.facts.base_hidden(*key),
                "a creature cannot replace the second player"
            );
        }
        owner
            .runtime
            .lock()
            .await
            .remove_test_actor(creature)
            .map_err(debug)?;
        let (partner, session) = {
            let mut runtime = owner.runtime.lock().await;
            let from = Position {
                x: second.x + 1,
                ..second
            };
            let (partner, session) = admit_player(&mut runtime, 98, from)?;
            let expected = runtime.read_actor_position(partner).map_err(debug)?;
            let closed = crate::map::floor::BundleDoors(map.facts.doors());
            let proof = crate::map::floor::prepare_bundle_step(
                &runtime,
                owner.movement_cells,
                partner,
                session,
                expected,
                crate::movement::CardinalStep::West,
                false,
                &closed,
            )
            .map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
            (partner, session)
        };
        let _ = session;
        refresh(owner, admitted.game_session_id).await?;
        for (key, p) in walls.iter().zip(&callback.passages) {
            assert!(map.facts.base_hidden(*key));
            assert!(
                !map.facts
                    .quest_callback_blocked(map.overlay.base(), tile(p.position), true)
            );
        }
        let (straddler, creature) = {
            let mut runtime = owner.runtime.lock().await;
            let (straddler, _) = admit_player(&mut runtime, 97, callback.passages[1].position)?;
            let creature = runtime
                .admit_pinned_test_creature(super::quest_world_callbacks::local(
                    callback.passages[2].position,
                ))
                .map_err(debug)?;
            (straddler, creature)
        };
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        super::qengine_access::step(owner, admitted, 2, StepDirection::West).await?;
        for (key, p) in walls.iter().zip(&callback.passages) {
            assert!(!map.facts.base_hidden(*key));
            assert!(
                map.facts
                    .quest_callback_blocked(map.overlay.base(), tile(p.position), true)
            );
        }
        let to = super::quest_world_callbacks::local(
            callback
                .occupancy_requirement
                .as_ref()
                .and_then(|r| r.eject)
                .ok_or("ejection")?,
        );
        for actor in [straddler, creature] {
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                to,
                "every occupied wall is ejected to the literal source cell"
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        super::qengine_access::step(owner, admitted, 3, StepDirection::East).await?;
        for key in &walls {
            assert!(map.facts.base_hidden(*key));
        }
        owner
            .runtime
            .lock()
            .await
            .remove_test_actor(partner)
            .map_err(debug)?;
        refresh(owner, admitted.game_session_id).await?;
        for key in &walls {
            assert!(
                !map.facts.base_hidden(*key),
                "retirement closes without a stale StepOut"
            );
        }
        let key =
            oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
                .ok_or("plate")?;
        let revision = map.facts.object_revision(key);
        refresh(owner, admitted.game_session_id).await?;
        assert_eq!(map.facts.object_revision(key), revision);
        eprintln!(
            "POSTGRES_CASE_OBSERVED two_players_required_three_walls_native_player_and_creature_ejection_retirement_idempotence {:?}",
            started.elapsed()
        );
    } else {
        let at = tile(callback.occupancy_items[0].position);
        let entries = map.facts.movement_items(at);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].facts.appearance_id, 435);
        refresh(owner, admitted.game_session_id).await?;
        assert_eq!(
            map.facts.movement_items(at)[0].source,
            entries[0].source,
            "unchanged occupancy preserves the created instance"
        );
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        super::qengine_access::step(owner, admitted, 2, StepDirection::West).await?;
        assert!(map.facts.movement_items(at).is_empty());
        let creature = owner
            .runtime
            .lock()
            .await
            .admit_pinned_test_creature(super::quest_world_callbacks::local(trigger))
            .map_err(debug)?;
        refresh(owner, admitted.game_session_id).await?;
        assert!(
            map.facts.movement_items(at).is_empty(),
            "a creature cannot activate this player-only source"
        );
        owner
            .runtime
            .lock()
            .await
            .remove_test_actor(creature)
            .map_err(debug)?;
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        super::qengine_access::step(owner, admitted, 3, StepDirection::East).await?;
        assert_eq!(map.facts.movement_items(at).len(), 1);
        super::quest_catalogue_chests::relocate_source_actor(
            owner,
            admitted,
            actor,
            Position {
                x: trigger.x - 1,
                ..trigger
            },
        )
        .await?;
        refresh(owner, admitted.game_session_id).await?;
        assert!(
            map.facts.movement_items(at).is_empty(),
            "forced relocation cleans up without StepOut"
        );
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        super::qengine_access::step(owner, admitted, 4, StepDirection::East).await?;
        assert_eq!(map.facts.movement_items(at).len(), 1);
        owner
            .runtime
            .lock()
            .await
            .remove_test_actor(actor)
            .map_err(debug)?;
        refresh(owner, admitted.game_session_id).await?;
        assert!(
            map.facts.movement_items(at).is_empty(),
            "retirement releases the owned source item"
        );
        eprintln!(
            "POSTGRES_CASE_OBSERVED player_only_remote_item_exact_creation_removal_relocation_retirement_idempotence {:?}",
            started.elapsed()
        );
    }
    assert_eq!(
        owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?,
        before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
        receipts
    );
    Ok(())
}
